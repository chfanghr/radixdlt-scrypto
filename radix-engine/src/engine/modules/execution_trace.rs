use crate::engine::*;
use crate::fee::FeeReserve;
use crate::model::*;
use crate::types::*;
use scrypto::core::{FnIdent, MethodIdent, ReceiverMethodIdent};

#[derive(Debug, Clone, PartialEq, TypeId, Encode, Decode)]
pub struct ResourceChange {
    pub resource_address: ResourceAddress,
    pub component_id: ComponentId,
    pub vault_id: VaultId,
    pub amount: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionTraceReceipt {
    pub resource_changes: Vec<ResourceChange>,
}

#[derive(Debug)]
pub struct ExecutionTrace {
    /// Stores resource changes that resulted from vault's put/take operations.
    pub resource_changes: HashMap<ComponentId, HashMap<VaultId, (ResourceAddress, Decimal)>>,

    /// Stores component IDs associated with vaults that have been used to lock a fee.
    /// This, together with a FeeSummary, is later used to create ResourceChange entries
    /// for fee payments (incl. any refunds back to the vault).
    pub fee_vaults_components: HashMap<VaultId, ComponentId>,
}

impl<R: FeeReserve> Module<R> for ExecutionTrace {
    fn pre_sys_call(
        &mut self,
        track: &mut Track<R>,
        heap: &mut Vec<CallFrame>,
        input: SysCallInput,
    ) -> Result<(), ModuleError> {
        self.trace_invoke_method(
            heap 
            track,
            todo!("actor"), 
            input
        )
    }

    fn post_sys_call(
        &mut self,
        _track: &mut Track<R>,
        _heap: &mut Vec<CallFrame>,
        _output: SysCallOutput,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_wasm_instantiation(
        &mut self,
        _track: &mut Track<R>,
        _heap: &mut Vec<CallFrame>,
        _code: &[u8],
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_wasm_costing(
        &mut self,
        _track: &mut Track<R>,
        _heap: &mut Vec<CallFrame>,
        _units: u32,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn on_lock_fee(
        &mut self,
        _track: &mut Track<R>,
        _heap: &mut Vec<CallFrame>,
        _vault_id: VaultId,
        fee: Resource,
        _contingent: bool,
    ) -> Result<Resource, ModuleError> {
        Ok(fee)
    }
}

impl ExecutionTrace {
    pub fn new() -> ExecutionTrace {
        Self {
            resource_changes: HashMap::new(),
            fee_vaults_components: HashMap::new(),
        }
    }

    fn trace_invoke_method<'s, R: FeeReserve>(
        &mut self,
        call_frames: &Vec<CallFrame>,
        track: &mut Track<'s, R>,
        actor: &REActor,
        input: &SysCallInput, 
    ) -> Result<(), RuntimeError> {
        let method_ident = match input.fn_ident {
            FnIdent::Method(ReceiverMethodIdent { method_ident, .. }) => method_ident,
            _ => return Ok(()), 
        };

        if let RENodeId::Vault(vault_id) = node_id {
            /* TODO: Warning: depends on call frame's actor being the vault's parent component!
            This isn't always the case! For example, when vault is instantiated in a blueprint
            before the component is globalized (see: test_restricted_transfer in bucket.rs).
            For now, such vault calls are NOT traced.
            Possible solution:
            1. Separately record vault calls that have a blueprint parent
            2. Hook up to when the component is globalized and convert
               blueprint-parented vaults (if any) to regular
               trace entries with component parents. */
            if let REActor::Method(FullyQualifiedReceiverMethod {
                receiver: Receiver::Ref(RENodeId::Component(component_id)),
                ..
            }) = &actor
            {
                match method_ident {
                    MethodIdent::Native(NativeMethod::Vault(VaultMethod::Put)) => {
                        let decoded_input = scrypto_decode(&input.raw).map_err(|e| {
                            RuntimeError::ApplicationError(ApplicationError::VaultError(
                                VaultError::InvalidRequestData(e),
                            ))
                        })?;

                        self.handle_vault_put(
                            component_id,
                            vault_id,
                            decoded_input,
                            next_owned_values,
                        )?;
                    }
                    MethodIdent::Native(NativeMethod::Vault(VaultMethod::Take)) => {
                        let decoded_input = scrypto_decode(&input.raw).map_err(|e| {
                            RuntimeError::ApplicationError(ApplicationError::VaultError(
                                VaultError::InvalidRequestData(e),
                            ))
                        })?;

                        let mut vault_node_ref = node_pointer.to_ref(call_frames, track);

                        let resource_address = vault_node_ref.vault().resource_address();

                        self.handle_vault_take(
                            &resource_address,
                            component_id,
                            vault_id,
                            decoded_input,
                        )?;
                    }
                    MethodIdent::Native(NativeMethod::Vault(VaultMethod::LockFee)) => {
                        self.fee_vaults_components
                            .insert(vault_id.clone(), component_id.clone());
                    }
                    MethodIdent::Native(NativeMethod::Vault(VaultMethod::LockContingentFee)) => {
                        self.fee_vaults_components
                            .insert(vault_id.clone(), component_id.clone());
                    }
                    _ => {} // no-op
                }
            }
        }

        Ok(())
    }

    fn handle_vault_put(
        &mut self,
        component_id: &ComponentId,
        vault_id: &VaultId,
        input: VaultPutInput,
        next_owned_values: &HashMap<RENodeId, HeapRootRENode>,
    ) -> Result<(), RuntimeError> {
        let bucket_id = input.bucket.0;
        let bucket_node_id = RENodeId::Bucket(bucket_id);

        let bucket_node =
            next_owned_values
                .get(&bucket_node_id)
                .ok_or(RuntimeError::KernelError(KernelError::RENodeNotFound(
                    bucket_node_id,
                )))?;

        if let HeapRENode::Bucket(bucket) = &bucket_node.root {
            if let ResourceType::Fungible { divisibility: _ } = bucket.resource_type() {
                self.record_resource_change(
                    &bucket.resource_address(),
                    component_id,
                    vault_id,
                    bucket.total_amount(),
                );
                Ok(())
            } else {
                /* TODO: Also handle non-fungible resource changes */
                Ok(())
            }
        } else {
            Err(RuntimeError::KernelError(KernelError::BucketNotFound(
                bucket_id,
            )))
        }
    }

    fn handle_vault_take(
        &mut self,
        resource_address: &ResourceAddress,
        component_id: &ComponentId,
        vault_id: &VaultId,
        input: VaultTakeInput,
    ) -> Result<(), RuntimeError> {
        self.record_resource_change(resource_address, component_id, vault_id, -input.amount);
        Ok(())
    }

    fn record_resource_change(
        &mut self,
        resource_address: &ResourceAddress,
        component_id: &ComponentId,
        vault_id: &VaultId,
        amount: Decimal,
    ) {
        let component_changes = self
            .resource_changes
            .entry(component_id.clone())
            .or_insert(HashMap::new());

        let vault_change = component_changes
            .entry(vault_id.clone())
            .or_insert((resource_address.clone(), Decimal::zero()));

        vault_change.1 += amount;
    }

    pub fn to_receipt(
        mut self,
        fee_payments: HashMap<VaultId, (ResourceAddress, Decimal)>,
    ) -> ExecutionTraceReceipt {
        // Add fee payments resource changes
        for (vault_id, (resource_address, amount)) in fee_payments {
            let component_id = self
                .fee_vaults_components
                .get(&vault_id)
                .expect("Failed to find component ID for a fee payment vault")
                .clone();
            self.record_resource_change(&resource_address, &component_id, &vault_id, -amount);
        }

        let resource_changes: Vec<ResourceChange> = self
            .resource_changes
            .into_iter()
            .flat_map(|(component_id, v)| {
                v.into_iter().map(
                    move |(vault_id, (resource_address, amount))| ResourceChange {
                        resource_address,
                        component_id,
                        vault_id,
                        amount,
                    },
                )
            })
            .filter(|el| !el.amount.is_zero())
            .collect();

        ExecutionTraceReceipt { resource_changes }
    }
}
