use crate::constants::DEFAULT_MAX_CALL_DEPTH;
use crate::engine::{ExecutionTrace, Kernel, SystemApi, TrackReceipt};
use crate::engine::{ResourceChange, Track};
use crate::fee::FeeReserve;
use crate::fee::UnlimitedLoanFeeReserve;
use crate::ledger::{ReadableSubstateStore, TypedInMemorySubstateStore, WriteableSubstateStore};
use crate::transaction::TransactionResult;
use crate::types::ResourceMethodAuthKey::Withdraw;
use crate::types::*;
use transaction::model::ExecutableInstruction;

#[derive(TypeId, Encode, Decode)]
struct SystemComponentState {
    xrd: scrypto::resource::Vault,
}

const XRD_SYMBOL: &str = "XRD";
const XRD_NAME: &str = "Radix";
const XRD_DESCRIPTION: &str = "The Radix Public Network's native token, used to pay the network's required transaction fees and to secure the network through staking to its validator nodes.";
const XRD_URL: &str = "https://tokens.radixdlt.com";
const XRD_MAX_SUPPLY: i128 = 24_000_000_000i128;
const XRD_VAULT_ID: VaultId = (Hash([0u8; 32]), 0);
const XRD_VAULT: scrypto::resource::Vault = scrypto::resource::Vault(XRD_VAULT_ID);

const SYS_FAUCET_COMPONENT_NAME: &str = "Faucet";

use crate::model::*;
use crate::wasm::{DefaultWasmEngine, InstructionCostRules, WasmInstrumenter, WasmMeteringParams};

pub struct GenesisReceipt {
    pub sys_faucet_package_address: PackageAddress,
    pub sys_utils_package_address: PackageAddress,
    pub account_package_address: PackageAddress,
}

// TODO: This would be much better handled if bootstrap was implemented as an executed transaction
// TODO: rather than a state snapshot.
pub fn execute_genesis<'s, R: FeeReserve>(
    mut track: Track<'s, R>,
) -> (TrackReceipt, GenesisReceipt) {
    let mut wasm_engine = DefaultWasmEngine::new();
    let mut wasm_instrumenter = WasmInstrumenter::new();
    let mut execution_trace = ExecutionTrace::new();

    let mut kernel = Kernel::new(
        Hash([0u8; Hash::LENGTH]),
        vec![],
        true,
        DEFAULT_MAX_CALL_DEPTH,
        &mut track,
        &mut wasm_engine,
        &mut wasm_instrumenter,
        WasmMeteringParams::new(InstructionCostRules::tiered(1, 5, 10, 5000), 512),
        &mut execution_trace,
        vec![],
    );

    let sys_faucet_package =
        extract_package(include_bytes!("../../../assets/sys_faucet.wasm").to_vec())
            .expect("Failed to construct sys-faucet package");
    let sys_utils_package =
        extract_package(include_bytes!("../../../assets/sys_utils.wasm").to_vec())
            .expect("Failed to construct sys-utils package");
    let account_package = extract_package(include_bytes!("../../../assets/account.wasm").to_vec())
        .expect("Failed to construct account package");

    let result = kernel
        .invoke_function(
            FnIdentifier::Native(NativeFnIdentifier::TransactionProcessor(
                TransactionProcessorFnIdentifier::Run,
            )),
            ScryptoValue::from_typed(&TransactionProcessorRunInput {
                instructions: vec![
                    ExecutableInstruction::PublishPackage {
                        package: scrypto_encode(&sys_faucet_package),
                    },
                    ExecutableInstruction::PublishPackage {
                        package: scrypto_encode(&sys_utils_package),
                    },
                    ExecutableInstruction::PublishPackage {
                        package: scrypto_encode(&account_package),
                    },
                ],
            }),
        )
        .unwrap();

    let results: Vec<Vec<u8>> = scrypto_decode(&result.raw).unwrap();
    let sys_faucet_package_address: PackageAddress = scrypto_decode(&results[0]).unwrap();
    let sys_utils_package_address: PackageAddress = scrypto_decode(&results[1]).unwrap();
    let account_package_address: PackageAddress = scrypto_decode(&results[2]).unwrap();

    println!(
        "account_package_address: {:?}",
        account_package_address.to_vec()
    );

    // Radix token resource address
    let mut metadata = HashMap::new();
    metadata.insert("symbol".to_owned(), XRD_SYMBOL.to_owned());
    metadata.insert("name".to_owned(), XRD_NAME.to_owned());
    metadata.insert("description".to_owned(), XRD_DESCRIPTION.to_owned());
    metadata.insert("url".to_owned(), XRD_URL.to_owned());

    let mut resource_auth = HashMap::new();
    resource_auth.insert(Withdraw, (rule!(allow_all), LOCKED));

    let mut xrd_resource_manager = ResourceManager::new(
        ResourceType::Fungible { divisibility: 18 },
        metadata,
        resource_auth,
    )
    .expect("Failed to construct XRD resource manager");
    let minted_xrd = xrd_resource_manager
        .mint_fungible(XRD_MAX_SUPPLY.into(), RADIX_TOKEN.clone())
        .expect("Failed to mint XRD");
    track.create_uuid_substate(
        SubstateId::ResourceManager(RADIX_TOKEN),
        xrd_resource_manager,
        true,
    );

    let mut ecdsa_resource_auth = HashMap::new();
    ecdsa_resource_auth.insert(Withdraw, (rule!(allow_all), LOCKED));
    let ecdsa_token = ResourceManager::new(
        ResourceType::NonFungible,
        HashMap::new(),
        ecdsa_resource_auth,
    )
    .expect("Failed to construct ECDSA resource manager");
    track.create_uuid_substate(SubstateId::ResourceManager(ECDSA_TOKEN), ecdsa_token, true);

    let system_token =
        ResourceManager::new(ResourceType::NonFungible, HashMap::new(), HashMap::new())
            .expect("Failed to construct SYSTEM_TOKEN resource manager");
    track.create_uuid_substate(
        SubstateId::ResourceManager(SYSTEM_TOKEN),
        system_token,
        true,
    );

    let initial_xrd = ResourceChange {
        resource_address: RADIX_TOKEN,
        component_address: SYS_FAUCET_COMPONENT,
        vault_id: XRD_VAULT_ID,
        amount: minted_xrd.total_amount(),
    };

    let system_vault = Vault::new(minted_xrd);
    track.create_uuid_substate(SubstateId::Vault(XRD_VAULT_ID), system_vault, false);

    let sys_faucet_component_info = ComponentInfo::new(
        sys_faucet_package_address,
        SYS_FAUCET_COMPONENT_NAME.to_owned(),
        vec![],
    );
    let sys_faucet_component_state =
        ComponentState::new(scrypto_encode(&SystemComponentState { xrd: XRD_VAULT }));
    track.create_uuid_substate(
        SubstateId::ComponentInfo(SYS_FAUCET_COMPONENT),
        sys_faucet_component_info,
        true,
    );
    track.create_uuid_substate(
        SubstateId::ComponentState(SYS_FAUCET_COMPONENT),
        sys_faucet_component_state,
        true,
    );
    track.create_uuid_substate(SubstateId::System, System { epoch: 0 }, true);

    let track_receipt = track.finalize(Ok(Vec::new()), vec![initial_xrd]);
    (
        track_receipt,
        GenesisReceipt {
            sys_faucet_package_address,
            sys_utils_package_address,
            account_package_address,
        },
    )
}

pub fn bootstrap<S>(substate_store: &mut S) -> GenesisReceipt
where
    S: ReadableSubstateStore + WriteableSubstateStore,
{
    if substate_store
        .get_substate(&SubstateId::ResourceManager(RADIX_TOKEN))
        .is_none()
    {
        let track = Track::new(substate_store, UnlimitedLoanFeeReserve::default());
        let (track_receipt, bootstrap_receipt) = execute_genesis(track);
        if let TransactionResult::Commit(c) = track_receipt.result {
            c.state_updates.commit(substate_store);
        } else {
            panic!("Failed to bootstrap")
        }
        bootstrap_receipt
    } else {
        let mut temporary_substate_store = TypedInMemorySubstateStore::new();
        let track = Track::new(
            &mut temporary_substate_store,
            UnlimitedLoanFeeReserve::default(),
        );
        let (_track_receipt, bootstrap_receipt) = execute_genesis(track);
        bootstrap_receipt
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::Track;
    use crate::fee::UnlimitedLoanFeeReserve;
    use crate::ledger::{execute_genesis, TypedInMemorySubstateStore};
    use scrypto::constants::ACCOUNT_PACKAGE;
    use scrypto::prelude::{SYS_FAUCET_PACKAGE, SYS_UTILS_PACKAGE};

    #[test]
    fn bootstrap_receipt_should_match_constants() {
        let mut temporary_substate_store = TypedInMemorySubstateStore::new();
        let track = Track::new(
            &mut temporary_substate_store,
            UnlimitedLoanFeeReserve::default(),
        );
        let (_track_receipt, bootstrap_receipt) = execute_genesis(track);

        assert_eq!(
            bootstrap_receipt.sys_faucet_package_address,
            SYS_FAUCET_PACKAGE
        );
        assert_eq!(
            bootstrap_receipt.sys_utils_package_address,
            SYS_UTILS_PACKAGE
        );
        assert_eq!(bootstrap_receipt.account_package_address, ACCOUNT_PACKAGE);
    }
}
