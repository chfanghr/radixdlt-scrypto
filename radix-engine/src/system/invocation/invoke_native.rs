use crate::kernel::kernel_api::KernelInvokeApi;
use crate::{blueprints::transaction_processor::NativeOutput, types::*};
use radix_engine_interface::api::types::{
    AccessRulesChainInvocation, AuthZoneStackInvocation, BucketInvocation, ComponentInvocation,
    LoggerInvocation, MetadataInvocation, NativeInvocation, PackageInvocation, ProofInvocation,
    TransactionRuntimeInvocation, WorktopInvocation,
};

pub fn invoke_native_fn<Y, E>(
    invocation: NativeInvocation,
    api: &mut Y,
) -> Result<Box<dyn NativeOutput>, E>
where
    Y: KernelInvokeApi<E>,
{
    match invocation {
        NativeInvocation::Component(component_invocation) => match component_invocation {
            ComponentInvocation::Globalize(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            ComponentInvocation::GlobalizeWithOwner(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            ComponentInvocation::SetRoyaltyConfig(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            ComponentInvocation::ClaimRoyalty(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::Package(package_invocation) => match package_invocation {
            PackageInvocation::Publish(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            PackageInvocation::PublishNative(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            PackageInvocation::SetRoyaltyConfig(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            PackageInvocation::ClaimRoyalty(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::Bucket(bucket_invocation) => match bucket_invocation {
            BucketInvocation::Take(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            BucketInvocation::CreateProof(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            BucketInvocation::TakeNonFungibles(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            BucketInvocation::GetNonFungibleLocalIds(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            BucketInvocation::GetAmount(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            BucketInvocation::Put(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            BucketInvocation::GetResourceAddress(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::AuthZoneStack(auth_zone_invocation) => match auth_zone_invocation {
            AuthZoneStackInvocation::Pop(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::Push(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::CreateProof(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::CreateProofByAmount(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::CreateProofByIds(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::Clear(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::Drain(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            AuthZoneStackInvocation::AssertAuthRule(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::Proof(proof_invocation) => match proof_invocation {
            ProofInvocation::GetAmount(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            ProofInvocation::GetNonFungibleLocalIds(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            ProofInvocation::GetResourceAddress(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            ProofInvocation::Clone(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::AccessRulesChain(access_rules_invocation) => {
            match access_rules_invocation {
                AccessRulesChainInvocation::AddAccessCheck(invocation) => {
                    let rtn = api.kernel_invoke(invocation)?;
                    Ok(Box::new(rtn))
                }
                AccessRulesChainInvocation::SetMethodAccessRule(invocation) => {
                    let rtn = api.kernel_invoke(invocation)?;
                    Ok(Box::new(rtn))
                }
                AccessRulesChainInvocation::SetMethodMutability(invocation) => {
                    let rtn = api.kernel_invoke(invocation)?;
                    Ok(Box::new(rtn))
                }
                AccessRulesChainInvocation::SetGroupAccessRule(invocation) => {
                    let rtn = api.kernel_invoke(invocation)?;
                    Ok(Box::new(rtn))
                }
                AccessRulesChainInvocation::SetGroupMutability(invocation) => {
                    let rtn = api.kernel_invoke(invocation)?;
                    Ok(Box::new(rtn))
                }
                AccessRulesChainInvocation::GetLength(invocation) => {
                    let rtn = api.kernel_invoke(invocation)?;
                    Ok(Box::new(rtn))
                }
            }
        }
        NativeInvocation::Metadata(metadata_invocation) => match metadata_invocation {
            MetadataInvocation::Set(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            MetadataInvocation::Get(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::Logger(logger_invocation) => match logger_invocation {
            LoggerInvocation::Log(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::Worktop(worktop_invocation) => match worktop_invocation {
            WorktopInvocation::TakeNonFungibles(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::Put(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::Drain(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::AssertContainsNonFungibles(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::AssertContains(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::AssertContainsAmount(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::TakeAll(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            WorktopInvocation::TakeAmount(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
        NativeInvocation::TransactionRuntime(method) => match method {
            TransactionRuntimeInvocation::GetHash(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
            TransactionRuntimeInvocation::GenerateUuid(invocation) => {
                let rtn = api.kernel_invoke(invocation)?;
                Ok(Box::new(rtn))
            }
        },
    }
}
