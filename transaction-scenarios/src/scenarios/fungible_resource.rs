use crate::internal_prelude::*;
use radix_engine::types::*;
use radix_engine_interface::api::node_modules::ModuleConfig;
use radix_engine_interface::blueprints::account::ACCOUNT_TRY_DEPOSIT_OR_ABORT_IDENT;
use radix_engine_interface::*;

pub struct FungibleResourceScenario {
    core: ScenarioCore,
    config: FungibleResourceScenarioConfig,
}

pub struct FungibleResourceScenarioConfig {
    /* Accounts */
    pub user_account_1: VirtualAccount,
    pub user_account_2: VirtualAccount,

    /* Entities - These get created during the scenario */
    pub max_divisibility_fungible_resource: Option<ResourceAddress>,
    pub min_divisibility_fungible_resource: Option<ResourceAddress>,
    pub vault1: Option<InternalAddress>,
    pub vault2: Option<InternalAddress>,
}

impl Default for FungibleResourceScenarioConfig {
    fn default() -> Self {
        Self {
            user_account_1: secp256k1_account_1(),
            user_account_2: secp256k1_account_2(),
            max_divisibility_fungible_resource: Default::default(),
            min_divisibility_fungible_resource: Default::default(),
            vault1: Default::default(),
            vault2: Default::default(),
        }
    }
}

impl ScenarioDefinition for FungibleResourceScenario {
    type Config = FungibleResourceScenarioConfig;

    fn new_with_config(core: ScenarioCore, config: Self::Config) -> Self {
        Self { core, config }
    }
}

impl ScenarioInstance for FungibleResourceScenario {
    fn metadata(&self) -> ScenarioMetadata {
        ScenarioMetadata {
            logical_name: "fungible_resource",
        }
    }

    fn next(&mut self, previous: Option<&TransactionReceipt>) -> Result<NextAction, ScenarioError> {
        let FungibleResourceScenarioConfig {
            user_account_1,
            user_account_2,
            max_divisibility_fungible_resource,
            min_divisibility_fungible_resource,
            vault1,
            vault2,
        } = &mut self.config;
        let core = &mut self.core;

        let up_next = match core.next_stage() {
            1 => {
                core.check_start(&previous)?;
                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-create",
                    |builder| {
                        builder
                            .create_fungible_resource(
                                OwnerRole::None,
                                false,
                                18,
                                metadata! {},
                                btreemap! {
                                    Mint => (rule!(allow_all), rule!(deny_all)),
                                    Burn =>  (rule!(allow_all), rule!(deny_all)),
                                    UpdateNonFungibleData => (rule!(allow_all), rule!(deny_all)),
                                    Withdraw => (rule!(allow_all), rule!(deny_all)),
                                    Deposit => (rule!(allow_all), rule!(deny_all)),
                                    Recall => (rule!(allow_all), rule!(deny_all)),
                                    Freeze => (rule!(allow_all), rule!(deny_all)),
                                },
                                Some(dec!("100000")),
                            )
                            .try_deposit_batch_or_abort(user_account_1.address)
                    },
                    vec![],
                )
            }
            2 => {
                let commit_success = core.check_commit_success(&previous)?;
                *max_divisibility_fungible_resource =
                    Some(commit_success.new_resource_addresses()[0]);
                *vault1 = Some(commit_success.new_vault_addresses()[0]);

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-mint",
                    |builder| {
                        builder
                            .mint_fungible(max_divisibility_fungible_resource.unwrap(), dec!("100"))
                            .try_deposit_batch_or_abort(user_account_1.address)
                    },
                    vec![],
                )
            }
            3 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-burn",
                    |builder| {
                        builder
                            .withdraw_from_account(
                                user_account_1.address,
                                max_divisibility_fungible_resource.unwrap(),
                                dec!("10"),
                            )
                            .take_all_from_worktop(
                                max_divisibility_fungible_resource.unwrap(),
                                |builder, bucket| builder.burn_resource(bucket),
                            )
                    },
                    vec![&user_account_1.key],
                )
            }
            4 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-transfer-32-times",
                    |builder| {
                        let mut builder = builder.withdraw_from_account(
                            user_account_1.address,
                            max_divisibility_fungible_resource.unwrap(),
                            dec!("10"),
                        );
                        for _ in 0..32 {
                            builder = builder.take_from_worktop(
                                max_divisibility_fungible_resource.unwrap(),
                                dec!("0.001"),
                                |builder, bucket| {
                                    builder.call_method(
                                        user_account_2.address,
                                        ACCOUNT_TRY_DEPOSIT_OR_ABORT_IDENT,
                                        manifest_args!(bucket),
                                    )
                                },
                            );
                        }
                        builder.try_deposit_batch_or_abort(user_account_1.address)
                    },
                    vec![&user_account_1.key],
                )
            }
            5 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-freeze-withdraw",
                    |builder| builder.freeze_withdraw(vault1.unwrap()),
                    vec![&user_account_1.key],
                )
            }
            6 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-freeze-deposit",
                    |builder| builder.freeze_deposit(vault1.unwrap()),
                    vec![&user_account_1.key],
                )
            }
            7 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-freeze-deposit",
                    |builder| builder.freeze_burn(vault1.unwrap()),
                    vec![&user_account_1.key],
                )
            }
            8 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-recall-freezed-vault",
                    |builder| {
                        builder
                            .recall(vault1.unwrap(), dec!("2"))
                            .try_deposit_batch_or_abort(user_account_1.address)
                    },
                    vec![&user_account_1.key],
                )
            }
            9 => {
                // FIXME: re-enable this after recalling from frozen vaults is allowed.
                // core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-unfreeze-withdraw",
                    |builder| builder.unfreeze_withdraw(vault1.unwrap()),
                    vec![&user_account_1.key],
                )
            }
            10 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-unfreeze-deposit",
                    |builder| builder.unfreeze_deposit(vault1.unwrap()),
                    vec![&user_account_1.key],
                )
            }
            11 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-unfreeze-deposit",
                    |builder| builder.unfreeze_burn(vault1.unwrap()),
                    vec![&user_account_1.key],
                )
            }
            12 => {
                core.check_commit_success(&previous)?;

                core.next_transaction_with_faucet_lock_fee(
                    "nfr-max-div-recall-unfreezed-vault",
                    |builder| {
                        builder
                            .recall(vault1.unwrap(), dec!("2"))
                            .try_deposit_batch_or_abort(user_account_1.address)
                    },
                    vec![&user_account_1.key],
                )
            }
            _ => {
                core.check_commit_success(&previous)?;

                let addresses = DescribedAddresses::new()
                    .add("user_account_1", user_account_1.address.clone())
                    .add("user_account_2", user_account_2.address.clone())
                    .add(
                        "max_divisibility_fungible_resource",
                        max_divisibility_fungible_resource.unwrap(),
                    );
                return Ok(core.finish_scenario(addresses));
            }
        };
        Ok(NextAction::Transaction(up_next))
    }
}
