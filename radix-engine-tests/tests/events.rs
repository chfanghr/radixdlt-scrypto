use radix_engine::blueprints::epoch_manager::{
    ClaimXrdEvent, EpochChangeEvent, RegisterValidatorEvent, RoundChangeEvent, StakeEvent,
    UnregisterValidatorEvent, UnstakeEvent, UpdateAcceptingStakeDelegationStateEvent,
};
use radix_engine::blueprints::resource::*;
use radix_engine::errors::{ApplicationError, RuntimeError};
use radix_engine::ledger::create_genesis;
use radix_engine::system::kernel_modules::events::EventError;
use radix_engine::system::node_modules::access_rules::SetRuleEvent;
use radix_engine::system::node_modules::metadata::SetMetadataEvent;
use radix_engine::types::*;
use radix_engine_interface::api::node_modules::auth::AuthAddresses;
use radix_engine_interface::api::node_modules::metadata::{MetadataEntry, MetadataValue};
use radix_engine_interface::blueprints::account::*;
use radix_engine_interface::blueprints::epoch_manager::{
    EpochManagerNextRoundInput, ValidatorUpdateAcceptDelegatedStakeInput,
    EPOCH_MANAGER_NEXT_ROUND_IDENT, VALIDATOR_UPDATE_ACCEPT_DELEGATED_STAKE_IDENT,
};
use scrypto::prelude::Mutability::LOCKED;
use scrypto::prelude::{AccessRule, FromPublicKey, ResourceMethodAuthKey};
use scrypto::NonFungibleData;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;
use transaction::ecdsa_secp256k1::EcdsaSecp256k1PrivateKey;
use transaction::model::{Instruction, SystemTransaction};

// TODO: In the future, the ClientAPI should only be able to add events to the event store. It
// should not be able to have full control over it.

// TODO: Creation of proofs triggers withdraw and deposit events when the amount is still liquid.
// This is not the intended behavior. Should figure out a solution to that so that it doesn't emit
// that and clean up this test to have one event.

//=========
// Scrypto
//=========

#[test]
fn scrypto_cant_emit_unregistered_event() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/events");

    let manifest = ManifestBuilder::new()
        .call_function(
            package_address,
            "ScryptoEvents",
            "emit_event",
            manifest_args!(12u64),
        )
        .build();

    // Act
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|runtime_error| {
        matches!(
            runtime_error,
            RuntimeError::ApplicationError(ApplicationError::EventError(
                EventError::SchemaNotFoundError { .. },
            )),
        )
    });
}

//=======
// Vault
//=======

#[test]
fn locking_fee_against_a_vault_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 1); // One event: lock fee
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
    }
}

#[test]
fn vault_fungible_recall_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (_, _, account) = test_runner.new_account(false);
    let recallable_resource_address = test_runner.create_recallable_token(account);
    let vault_id = test_runner.get_component_vaults(account, recallable_resource_address)[0];

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .recall(vault_id, 1.into())
        .call_method(
            account,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 4); // Four events: vault lock fee, vault fungible withdraw, vault fungible recall, vault fungible deposit
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        // TODO: Currently recall first emits a withdraw event and then a recall event. Should the
        // redundant withdraw event go away or does it make sense from a user perspective?
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name)
                && is_decoded_equal(&WithdrawResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<RecallResourceEvent, _>(event_event_name)
                && is_decoded_equal(&RecallResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(3) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name)
                && is_decoded_equal(&DepositResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
    }
}

// TODO: Currently treats non-fungibles as fungible. Correct this test once recall non-fungibles
// has a dedicated instruction.
#[test]
fn vault_non_fungible_recall_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (_, _, account) = test_runner.new_account(false);
    let (recallable_resource_address, non_fungible_local_id) = {
        let mut access_rules = BTreeMap::new();
        access_rules.insert(ResourceMethodAuthKey::Withdraw, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Deposit, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Recall, (rule!(allow_all), LOCKED));

        let id = NonFungibleLocalId::Integer(IntegerNonFungibleLocalId::new(1));

        let manifest = ManifestBuilder::new()
            .lock_fee(FAUCET_COMPONENT, 100u32.into())
            .create_non_fungible_resource(
                NonFungibleIdType::Integer,
                BTreeMap::new(),
                access_rules,
                Some([(id.clone(), EmptyStruct {})]),
            )
            .call_method(
                account,
                ACCOUNT_DEPOSIT_BATCH_IDENT,
                manifest_args!(ManifestExpression::EntireWorktop),
            )
            .build();
        let receipt = test_runner.execute_manifest(manifest, vec![]);
        receipt.expect_commit_success();
        (
            receipt
                .expect_commit(true)
                .entity_changes
                .new_resource_addresses[0],
            id,
        )
    };
    let vault_id = test_runner.get_component_vaults(account, recallable_resource_address)[0];

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .recall(vault_id, 1.into())
        .call_method(
            account,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 4); // Four events: vault lock fee, vault non-fungible withdraw, vault non-fungible recall, vault non-fungible deposit
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        // TODO: Currently recall first emits a withdraw event and then a recall event. Should the
        // redundant withdraw event go away or does it make sense from a user perspective?
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name)
                && is_decoded_equal(&WithdrawResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<RecallResourceEvent, _>(event_event_name)
                && is_decoded_equal(&RecallResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(3) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name)
                && is_decoded_equal(
                    &DepositResourceEvent::Ids([non_fungible_local_id.clone()].into()),
                    event_data
                ) =>
                true,
            _ => false,
        });
    }
}

//==================
// Resource Manager
//==================

#[test]
fn resource_manager_new_vault_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (_, _, account) = test_runner.new_account(false);

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .create_fungible_resource(
            18,
            Default::default(),
            BTreeMap::<ResourceMethodAuthKey, (AccessRule, AccessRule)>::new(),
            Some(1.into()),
        )
        .call_method(
            account,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 3); // Three events: vault lock fee, resource manager create vault, vault fungible deposit
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(
                        RENodeId::GlobalObject(Address::Resource(..)),
                        NodeModuleId::SELF,
                    ),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<VaultCreationEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name)
                && is_decoded_equal(&DepositResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
    }
}

#[test]
fn resource_manager_mint_and_burn_fungible_resource_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (_, _, account) = test_runner.new_account(false);
    let resource_address = {
        let mut access_rules = BTreeMap::new();
        access_rules.insert(ResourceMethodAuthKey::Withdraw, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Deposit, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Mint, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Burn, (rule!(allow_all), LOCKED));

        let manifest = ManifestBuilder::new()
            .lock_fee(FAUCET_COMPONENT, 100u32.into())
            .create_fungible_resource(18, Default::default(), access_rules, None)
            .call_method(
                account,
                ACCOUNT_DEPOSIT_BATCH_IDENT,
                manifest_args!(ManifestExpression::EntireWorktop),
            )
            .build();
        let receipt = test_runner.execute_manifest(manifest, vec![]);
        receipt.expect_commit_success();
        receipt
            .expect_commit(true)
            .entity_changes
            .new_resource_addresses[0]
    };

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .mint_fungible(resource_address, 10.into())
        .burn(10.into(), resource_address)
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 3); // Three events: vault lock fee, resource manager mint fungible, resource manager burn fungible
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<MintResourceEvent, _>(event_event_name)
                && is_decoded_equal(&MintResourceEvent::Amount(10.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<BurnResourceEvent, _>(event_event_name)
                && is_decoded_equal(&BurnResourceEvent::Amount(10.into()), event_data) =>
                true,
            _ => false,
        });
    }
}

#[test]
fn resource_manager_mint_and_burn_non_fungible_resource_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let (_, _, account) = test_runner.new_account(false);
    let resource_address = {
        let mut access_rules = BTreeMap::new();
        access_rules.insert(ResourceMethodAuthKey::Withdraw, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Deposit, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Mint, (rule!(allow_all), LOCKED));
        access_rules.insert(ResourceMethodAuthKey::Burn, (rule!(allow_all), LOCKED));

        let manifest = ManifestBuilder::new()
            .lock_fee(FAUCET_COMPONENT, 100u32.into())
            .create_non_fungible_resource(
                NonFungibleIdType::Integer,
                BTreeMap::new(),
                access_rules,
                None::<BTreeMap<NonFungibleLocalId, EmptyStruct>>,
            )
            .call_method(
                account,
                ACCOUNT_DEPOSIT_BATCH_IDENT,
                manifest_args!(ManifestExpression::EntireWorktop),
            )
            .build();
        let receipt = test_runner.execute_manifest(manifest, vec![]);
        receipt.expect_commit_success();
        receipt
            .expect_commit(true)
            .entity_changes
            .new_resource_addresses[0]
    };

    let id = NonFungibleLocalId::Integer(IntegerNonFungibleLocalId::new(1));
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .mint_non_fungible(resource_address, [(id.clone(), EmptyStruct {})])
        .burn(1.into(), resource_address)
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 3); // Three events: vault lock fee, resource manager mint non-fungible, resource manager burn non-fungible
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<MintResourceEvent, _>(event_event_name)
                && is_decoded_equal(&MintResourceEvent::Ids([id.clone()].into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<BurnResourceEvent, _>(event_event_name)
                && is_decoded_equal(&BurnResourceEvent::Ids([id.clone()].into()), event_data) =>
                true,
            _ => false,
        });
    }
}

//===============
// Epoch Manager
//===============

#[test]
fn epoch_manager_round_update_emits_correct_event() {
    let rounds_per_epoch = 5u64;
    let num_unstake_epochs = 1u64;
    let genesis = create_genesis(
        BTreeMap::new(),
        BTreeMap::new(),
        1u64,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();

    // Act
    let instructions = vec![Instruction::CallMethod {
        component_address: EPOCH_MANAGER,
        method_name: EPOCH_MANAGER_NEXT_ROUND_IDENT.to_string(),
        args: manifest_encode(&EpochManagerNextRoundInput {
            round: rounds_per_epoch - 1,
        })
        .unwrap(),
    }];
    let receipt = test_runner.execute_transaction(
        SystemTransaction {
            instructions,
            blobs: vec![],
            nonce: 0,
            pre_allocated_ids: BTreeSet::new(),
        }
        .get_executable(vec![AuthAddresses::validator_role()]),
    );

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 1); // One event: round change event
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<RoundChangeEvent, _>(event_event_name)
                && is_decoded_equal(&RoundChangeEvent { round: 4 }, event_data) =>
                true,
            _ => false,
        });
    }
}

#[test]
fn epoch_manager_epoch_update_emits_correct_event() {
    let rounds_per_epoch = 5u64;
    let num_unstake_epochs = 1u64;
    let genesis = create_genesis(
        BTreeMap::new(),
        BTreeMap::new(),
        1u64,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();

    // Act
    let instructions = vec![Instruction::CallMethod {
        component_address: EPOCH_MANAGER,
        method_name: EPOCH_MANAGER_NEXT_ROUND_IDENT.to_string(),
        args: manifest_encode(&EpochManagerNextRoundInput {
            round: rounds_per_epoch,
        })
        .unwrap(),
    }];
    let receipt = test_runner.execute_transaction(
        SystemTransaction {
            instructions,
            blobs: vec![],
            nonce: 0,
            pre_allocated_ids: BTreeSet::new(),
        }
        .get_executable(vec![AuthAddresses::validator_role()]),
    );

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 1); // One event: epoch change event
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<EpochChangeEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

//===========
// Validator
//===========

#[test]
fn validator_registration_emits_correct_event() {
    // Arrange
    let initial_epoch = 5u64;
    let rounds_per_epoch = 2u64;
    let num_unstake_epochs = 1u64;
    let pub_key = EcdsaSecp256k1PrivateKey::from_u64(1u64)
        .unwrap()
        .public_key();
    let genesis = create_genesis(
        BTreeMap::new(),
        BTreeMap::new(),
        initial_epoch,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();

    // Act
    let validator_address = create_validator(&mut test_runner, pub_key, rule!(allow_all));
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .register_validator(validator_address)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 2); // Two events: vault lock fee and register validator
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<RegisterValidatorEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

#[test]
fn validator_unregistration_emits_correct_event() {
    // Arrange
    let initial_epoch = 5u64;
    let rounds_per_epoch = 2u64;
    let num_unstake_epochs = 1u64;
    let pub_key = EcdsaSecp256k1PrivateKey::from_u64(1u64)
        .unwrap()
        .public_key();
    let genesis = create_genesis(
        BTreeMap::new(),
        BTreeMap::new(),
        initial_epoch,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();

    let validator_address = create_validator(&mut test_runner, pub_key, rule!(allow_all));
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .register_validator(validator_address)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    receipt.expect_commit_success();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .unregister_validator(validator_address)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 2); // Two events: vault lock fee and register validator
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<UnregisterValidatorEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

#[test]
fn validator_staking_emits_correct_event() {
    // Arrange
    let initial_epoch = 5u64;
    let rounds_per_epoch = 2u64;
    let num_unstake_epochs = 1u64;
    let pub_key = EcdsaSecp256k1PrivateKey::from_u64(1u64)
        .unwrap()
        .public_key();
    let genesis = create_genesis(
        BTreeMap::new(),
        BTreeMap::new(),
        initial_epoch,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();
    let (account_pk, _, account) = test_runner.new_account(false);

    let validator_address = create_validator(&mut test_runner, pub_key, rule!(allow_all));
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .register_validator(validator_address)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    receipt.expect_commit_success();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .withdraw_from_account(account, RADIX_TOKEN, 100.into())
        .take_from_worktop(RADIX_TOKEN, |builder, bucket| {
            builder.stake_validator(validator_address, bucket)
        })
        .call_method(
            account,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&account_pk)],
    );

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        assert_eq!(events.len(), 7); // Seven events: vault lock fee, vault withdraw fungible, resource manager mint (lp tokens), vault deposit event, validator stake event, resource manager vault create (for the LP tokens), vault deposit
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name)
                && is_decoded_equal(&WithdrawResourceEvent::Amount(100.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<MintResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(3) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name)
                && is_decoded_equal(&DepositResourceEvent::Amount(100.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(4) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<StakeEvent, _>(event_event_name)
                && is_decoded_equal(
                    &StakeEvent {
                        xrd_staked: 100.into()
                    },
                    event_data
                ) =>
                true,
            _ => false,
        });
        assert!(match events.get(5) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(
                        RENodeId::GlobalObject(Address::Resource(..)),
                        NodeModuleId::SELF,
                    ),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<VaultCreationEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(6) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

#[test]
fn validator_unstake_emits_correct_events() {
    // Arrange
    let initial_epoch = 5u64;
    let rounds_per_epoch = 2u64;
    let num_unstake_epochs = 1u64;
    let validator_pub_key = EcdsaSecp256k1PrivateKey::from_u64(2u64)
        .unwrap()
        .public_key();
    let account_pub_key = EcdsaSecp256k1PrivateKey::from_u64(1u64)
        .unwrap()
        .public_key();
    let account_with_lp = ComponentAddress::virtual_account_from_public_key(&account_pub_key);
    let mut validator_set_and_stake_owners = BTreeMap::new();
    validator_set_and_stake_owners.insert(validator_pub_key, (Decimal::from(10), account_with_lp));
    let genesis = create_genesis(
        validator_set_and_stake_owners,
        BTreeMap::new(),
        initial_epoch,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();
    let validator_address = test_runner.get_validator_with_key(&validator_pub_key);
    let validator_substate = test_runner.get_validator_info(validator_address);

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .withdraw_from_account(
            account_with_lp,
            validator_substate.liquidity_token,
            1.into(),
        )
        .take_from_worktop(validator_substate.liquidity_token, |builder, bucket| {
            builder.unstake_validator(validator_address, bucket)
        })
        .call_method(
            account_with_lp,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&account_pub_key)],
    );
    receipt.expect_commit_success();
    test_runner.set_current_epoch(initial_epoch + 1 + num_unstake_epochs);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        /*
        Nine Events:
        1. Lock Fee event
        2. Vault withdraw event (LP Tokens)
        3. Resource Manager burn event (LP Tokens)
        4. Vault withdraw event (withdraw from stake vault)
        5. Vault deposit event (deposit into stake vault)
        6. Resource Manager Mint (minting unstake redeem tokens)
        7. Validator Unstake event
        8. Resource Manager Vault creation event (unstake redeem tokens)
        9. Vault Deposit Event (unstake redeem tokens)
         */
        assert_eq!(events.len(), 9);
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name)
                && is_decoded_equal(&WithdrawResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<BurnResourceEvent, _>(event_event_name)
                && is_decoded_equal(&BurnResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(3) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(4) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(5) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(
                        RENodeId::GlobalObject(Address::Resource(resource_address)),
                        NodeModuleId::SELF,
                    ),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<MintResourceEvent, _>(event_event_name)
                && *resource_address == validator_substate.unstake_nft =>
                true,
            _ => false,
        });
        assert!(match events.get(6) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<UnstakeEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(7) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(
                        RENodeId::GlobalObject(Address::Resource(..)),
                        NodeModuleId::SELF,
                    ),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<VaultCreationEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(8) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

#[test]
fn validator_claim_xrd_emits_correct_events() {
    // Arrange
    let initial_epoch = 5u64;
    let rounds_per_epoch = 2u64;
    let num_unstake_epochs = 1u64;
    let validator_pub_key = EcdsaSecp256k1PrivateKey::from_u64(2u64)
        .unwrap()
        .public_key();
    let account_pub_key = EcdsaSecp256k1PrivateKey::from_u64(1u64)
        .unwrap()
        .public_key();
    let account_with_lp = ComponentAddress::virtual_account_from_public_key(&account_pub_key);
    let mut validator_set_and_stake_owners = BTreeMap::new();
    validator_set_and_stake_owners.insert(validator_pub_key, (Decimal::from(10), account_with_lp));
    let genesis = create_genesis(
        validator_set_and_stake_owners,
        BTreeMap::new(),
        initial_epoch,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();
    let validator_address = test_runner.get_validator_with_key(&validator_pub_key);
    let validator_substate = test_runner.get_validator_info(validator_address);
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .withdraw_from_account(
            account_with_lp,
            validator_substate.liquidity_token,
            1.into(),
        )
        .take_from_worktop(validator_substate.liquidity_token, |builder, bucket| {
            builder.unstake_validator(validator_address, bucket)
        })
        .call_method(
            account_with_lp,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&account_pub_key)],
    );
    receipt.expect_commit_success();
    test_runner.set_current_epoch(initial_epoch + 1 + num_unstake_epochs);

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .withdraw_from_account(account_with_lp, validator_substate.unstake_nft, 1.into())
        .take_from_worktop(validator_substate.unstake_nft, |builder, bucket| {
            builder.claim_xrd(validator_address, bucket)
        })
        .call_method(
            account_with_lp,
            ACCOUNT_DEPOSIT_BATCH_IDENT,
            manifest_args!(ManifestExpression::EntireWorktop),
        )
        .build();
    let receipt = test_runner.execute_manifest(
        manifest,
        vec![NonFungibleGlobalId::from_public_key(&account_pub_key)],
    );

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        /*
        Seven Events:
        1. Vault lock fee event
        2. Vault withdraw event (unstake nft)
        3. Resource Manager burn event (unstake nft)
        4. Vault withdraw event (unstaked xrd)
        5. Claim XRD
        5. Resource Manager vault creation event (XRD)
        6. Vault deposit event (XRD)
         */
        assert_eq!(events.len(), 7);
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name)
                && is_decoded_equal(&WithdrawResourceEvent::Amount(1.into()), event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<BurnResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(3) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<WithdrawResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(4) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<ClaimXrdEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(5) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(
                        RENodeId::GlobalObject(Address::Resource(..)),
                        NodeModuleId::SELF,
                    ),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<VaultCreationEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(6) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ..,
            )) if is_event_name_equal::<DepositResourceEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

#[test]
fn validator_update_stake_delegation_status_emits_correct_event() {
    // Arrange
    let initial_epoch = 5u64;
    let rounds_per_epoch = 2u64;
    let num_unstake_epochs = 1u64;
    let pub_key = EcdsaSecp256k1PrivateKey::from_u64(1u64)
        .unwrap()
        .public_key();
    let genesis = create_genesis(
        BTreeMap::new(),
        BTreeMap::new(),
        initial_epoch,
        rounds_per_epoch,
        num_unstake_epochs,
    );
    let mut test_runner = TestRunner::builder().with_custom_genesis(genesis).build();

    let validator_address = create_validator(&mut test_runner, pub_key, rule!(allow_all));
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .register_validator(validator_address)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    receipt.expect_commit_success();

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .call_method(
            validator_address,
            VALIDATOR_UPDATE_ACCEPT_DELEGATED_STAKE_IDENT,
            manifest_encode(&ValidatorUpdateAcceptDelegatedStakeInput {
                accept_delegated_stake: false,
            })
            .unwrap(),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        /*
        Two Events:
        1. Vault lock fee event
        2. AccessRule set rule
        3. Validator update delegation state
         */
        assert_eq!(events.len(), 3);
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(_, NodeModuleId::AccessRules),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<SetRuleEvent, _>(event_event_name) => true,
            _ => false,
        });
        assert!(match events.get(2) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<UpdateAcceptingStakeDelegationStateEvent, _>(
                event_event_name
            ) && is_decoded_equal(
                &UpdateAcceptingStakeDelegationStateEvent {
                    accepts_delegation: false
                },
                event_data
            ) =>
                true,
            _ => false,
        });
    }
}

//==========
// Metadata
//==========

#[test]
fn setting_metadata_emits_correct_events() {
    // Arrange
    let mut test_runner = TestRunner::builder().without_trace().build();
    let resource_address = create_all_allowed_resource(&mut test_runner);

    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .set_metadata(
            Address::Resource(resource_address),
            "key".into(),
            MetadataEntry::Value(MetadataValue::I32(1)),
        )
        .build();

    // Act
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    {
        receipt.expect_commit_success();
        let events = receipt.expect_commit(true).clone().application_events;
        for (id, _) in events.iter() {
            println!("{}", test_runner.event_name(&id))
        }
        /*
        Two events:
        1. Vault lock fee
        2. Metadata set entry
         */
        assert_eq!(events.len(), 2);
        assert!(match events.get(0) {
            Some((
                EventTypeIdentifier(Emitter::Method(_, NodeModuleId::SELF), ref event_event_name),
                ref event_data,
            )) if is_event_name_equal::<LockFeeEvent, _>(event_event_name)
                && is_decoded_equal(&LockFeeEvent { amount: 10.into() }, event_data) =>
                true,
            _ => false,
        });
        assert!(match events.get(1) {
            Some((
                EventTypeIdentifier(
                    Emitter::Method(_, NodeModuleId::Metadata),
                    ref event_event_name,
                ),
                ..,
            )) if is_event_name_equal::<SetMetadataEvent, _>(event_event_name) => true,
            _ => false,
        });
    }
}

//=========
// Helpers
//=========

#[derive(NonFungibleData)]
struct EmptyStruct {}

#[derive(ScryptoEncode, Describe)]
struct CustomEvent {
    number: u64,
}

fn event_name<T: ScryptoDescribe>() -> String {
    let (local_type_index, schema) =
        generate_full_schema_from_single_type::<T, ScryptoCustomTypeExtension>();
    (*schema
        .resolve_type_metadata(local_type_index)
        .expect("Cant fail")
        .type_name)
        .to_owned()
}

fn is_decoded_equal<T: ScryptoDecode + PartialEq>(expected: &T, actual: &[u8]) -> bool {
    scrypto_decode::<T>(&actual).unwrap() == *expected
}

fn is_event_name_equal<T: ScryptoDescribe, S: AsRef<str>>(actual: S) -> bool {
    event_name::<T>() == actual.as_ref().to_owned()
}

fn create_validator(
    test_runner: &mut TestRunner,
    pk: EcdsaSecp256k1PublicKey,
    owner_access_rule: AccessRule,
) -> ComponentAddress {
    let manifest = ManifestBuilder::new()
        .lock_fee(FAUCET_COMPONENT, 10.into())
        .create_validator(pk, owner_access_rule)
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    receipt.expect_commit_success();
    let component_address = receipt
        .expect_commit(true)
        .entity_changes
        .new_component_addresses[0];

    component_address
}

fn create_all_allowed_resource(test_runner: &mut TestRunner) -> ResourceAddress {
    let access_rules = [
        ResourceMethodAuthKey::Burn,
        ResourceMethodAuthKey::Deposit,
        ResourceMethodAuthKey::Withdraw,
        ResourceMethodAuthKey::Mint,
        ResourceMethodAuthKey::Burn,
        ResourceMethodAuthKey::UpdateMetadata,
        ResourceMethodAuthKey::UpdateNonFungibleData,
    ]
    .into_iter()
    .map(|method| (method, (AccessRule::AllowAll, AccessRule::AllowAll)))
    .collect();

    let manifest = ManifestBuilder::new()
        .create_fungible_resource(18, BTreeMap::new(), access_rules, None)
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![]);
    receipt.expect_commit_success();
    *receipt.new_resource_addresses().get(0).unwrap()
}
