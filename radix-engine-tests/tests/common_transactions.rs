use radix_engine::errors::{RuntimeError, SystemError};
use radix_engine::transaction::TransactionReceipt;
use radix_engine::types::*;
use radix_engine_interface::api::node_modules::auth::{RoleDefinition, ToRoleEntry};
use radix_engine_interface::api::node_modules::ModuleConfig;
use radix_engine_interface::{metadata, metadata_init, mint_roles};
use scrypto::NonFungibleData;
use scrypto_unit::TestRunner;
use transaction::manifest::{compile, BlobProvider};
use transaction::prelude::*;
use transaction::signing::secp256k1::Secp256k1PrivateKey;
use utils::ContextualDisplay;

macro_rules! replace_variables {
    ($content:expr $(, $a:ident = $b:expr)* ) => {
        $content
            $(.replace(concat!("${", stringify!($a), "}"), &format!("{}", $b)))*
    };
}

#[test]
fn test_allocate_address_and_call_it() {
    run_manifest(|account_address, address_bech32_encoder| {
        let code_blob = include_bytes!("../../assets/radiswap.wasm").to_vec();
        let manifest = replace_variables!(
            include_str!("../../transaction/examples/address_allocation/allocate_address.rtm"),
            account_address = account_address.display(address_bech32_encoder),
            package_package_address = PACKAGE_PACKAGE.display(address_bech32_encoder),
            code_blob_hash = hash(&code_blob)
        );
        (manifest, vec![code_blob])
    })
    .expect_specific_failure(|e| match e {
        RuntimeError::SystemError(SystemError::AuthTemplateDoesNotExist(..)) => true,
        _ => false,
    });
}

/// An example manifest for transfer of funds between accounts
#[test]
fn transfer_of_funds_to_another_account_succeeds() {
    run_manifest(|this_account_address, address_bech32_encoder| {
        let private_key = Secp256k1PrivateKey::from_u64(12).unwrap();
        let public_key = private_key.public_key();
        let other_account_address = ComponentAddress::virtual_account_from_public_key(&public_key);

        let manifest = replace_variables!(
            include_str!("../../transaction/examples/account/resource_transfer.rtm"),
            xrd_resource_address = XRD.display(address_bech32_encoder),
            this_account_address = this_account_address.display(address_bech32_encoder),
            other_account_address = other_account_address.display(address_bech32_encoder)
        );
        (manifest, Vec::new())
    })
    .expect_commit_success();
}

#[test]
fn multi_account_fund_transfer_succeeds() {
    test_manifest_with_additional_accounts(
        3,
        |this_account_address, other_accounts, address_bech32_encoder| {
            let manifest = replace_variables!(
                include_str!(
                    "../../transaction/examples/account/multi_account_resource_transfer.rtm"
                ),
                xrd_resource_address = XRD.display(address_bech32_encoder),
                this_account_address = address_bech32_encoder
                    .encode(this_account_address.as_ref())
                    .unwrap(),
                account_a_component_address = other_accounts[0].display(address_bech32_encoder),
                account_b_component_address = other_accounts[1].display(address_bech32_encoder),
                account_c_component_address = other_accounts[2].display(address_bech32_encoder)
            );
            (manifest, Vec::new())
        },
    )
}

/// An example manifest for creating a new fungible resource with no initial supply
#[test]
fn creating_a_fungible_resource_with_no_initial_supply_succeeds() {
    run_manifest(|account_address, address_bech32_encoder| {
        let manifest = replace_variables!(
            include_str!(
                "../../transaction/examples/resources/creation/fungible/no_initial_supply.rtm"
            ),
            account_address = account_address.display(address_bech32_encoder)
        );
        (manifest, Vec::new())
    })
    .expect_commit_success();
}

/// An example manifest for creating a new fungible resource with an initial supply
#[test]
fn creating_a_fungible_resource_with_initial_supply_succeeds() {
    run_manifest(|account_address, address_bech32_encoder| {
        let initial_supply = dec!("10000000");

        let manifest = replace_variables!(
            include_str!(
                "../../transaction/examples/resources/creation/fungible/with_initial_supply.rtm"
            ),
            initial_supply = initial_supply,
            account_address = account_address.display(address_bech32_encoder)
        );
        (manifest, Vec::new())
    })
    .expect_commit_success();
}

/// An example manifest for creating a new non-fungible resource with no supply
#[test]
fn creating_a_non_fungible_resource_with_no_initial_supply_succeeds() {
    run_manifest(|account_address, address_bech32_encoder| {
        let manifest = replace_variables!(
            include_str!(
                "../../transaction/examples/resources/creation/non_fungible/no_initial_supply.rtm"
            ),
            account_address = account_address.display(address_bech32_encoder)
        );
        (manifest, Vec::new())
    })
    .expect_commit_success();
}

/// An example manifest for creating a new non-fungible resource with an initial supply
#[test]
fn creating_a_non_fungible_resource_with_initial_supply_succeeds() {
    run_manifest(|account_address, address_bech32_encoder| {
        let manifest = replace_variables!(
            include_str!("../../transaction/examples/resources/creation/non_fungible/with_initial_supply.rtm"),
            account_address =
                account_address.display(address_bech32_encoder),
                non_fungible_local_id = "#1#"
        );
        (manifest, Vec::new())
    })
    .expect_commit_success();
}

/// A sample manifest that publishes a package.
#[test]
fn publish_package_succeeds() {
    run_manifest(|account_address, address_bech32_encoder| {
        let code_blob = include_bytes!("../../assets/faucet.wasm").to_vec();

        let manifest = replace_variables!(
            include_str!("../../transaction/examples/package/publish.rtm"),
            code_blob_hash = hash(&code_blob),
            account_address = account_address.display(address_bech32_encoder),
            auth_badge_resource_address = XRD.display(address_bech32_encoder),
            auth_badge_non_fungible_local_id = "#1#"
        );
        (manifest, vec![code_blob])
    })
    .expect_commit_success();
}

/// A sample manifest for minting of a fungible resource
#[test]
fn minting_of_fungible_resource_succeeds() {
    test_manifest_with_restricted_minting_resource(
        ResourceType::Fungible { divisibility: 18 },
        |account_address,
         minter_badge_resource_address,
         mintable_fungible_resource_address,
         address_bech32_encoder| {
            let mint_amount = dec!("800");

            let manifest = replace_variables!(
                include_str!("../../transaction/examples/resources/mint/fungible/mint.rtm"),
                account_address = account_address.display(address_bech32_encoder),
                mintable_fungible_resource_address =
                    mintable_fungible_resource_address.display(address_bech32_encoder),
                minter_badge_resource_address =
                    minter_badge_resource_address.display(address_bech32_encoder),
                mint_amount = mint_amount
            );
            (manifest, Vec::new())
        },
    );
}

/// A sample manifest for minting of a non-fungible resource
#[test]
fn minting_of_non_fungible_resource_succeeds() {
    test_manifest_with_restricted_minting_resource(
        ResourceType::NonFungible {
            id_type: NonFungibleIdType::Integer,
        },
        |account_address,
         minter_badge_resource_address,
         mintable_non_fungible_resource_address,
         address_bech32_encoder| {
            let manifest = replace_variables!(
                include_str!("../../transaction/examples/resources/mint/non_fungible/mint.rtm"),
                account_address = account_address.display(address_bech32_encoder),
                mintable_non_fungible_resource_address =
                    mintable_non_fungible_resource_address.display(address_bech32_encoder),
                minter_badge_resource_address =
                    minter_badge_resource_address.display(address_bech32_encoder),
                non_fungible_local_id = "#1#"
            );
            (manifest, Vec::new())
        },
    );
}

#[test]
fn changing_account_default_deposit_rule_succeeds() {
    test_manifest_with_restricted_minting_resource(
        ResourceType::Fungible { divisibility: 18 },
        |account_address,
         minter_badge_resource_address,
         mintable_resource_address,
         address_bech32_encoder| {
            let manifest = replace_variables!(
                include_str!("../../transaction/examples/account/deposit_modes.rtm"),
                account_address = account_address.display(address_bech32_encoder),
                first_resource_address = mintable_resource_address.display(address_bech32_encoder),
                second_resource_address =
                    minter_badge_resource_address.display(address_bech32_encoder)
            );
            (manifest, Vec::new())
        },
    );
}

fn run_manifest<F>(string_manifest_builder: F) -> TransactionReceipt
where
    F: Fn(&ComponentAddress, &AddressBech32Encoder) -> (String, Vec<Vec<u8>>),
{
    // Creating a new test runner
    let mut test_runner = TestRunner::builder().build();

    // Creating the account component required for this test
    let (public_key, _, component_address) = test_runner.new_account(false);
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    // Defining the network and the bech32 encoder to use
    let network = NetworkDefinition::simulator();
    let address_bech32_encoder = AddressBech32Encoder::new(&network);

    // Run the function and get the manifest string
    let (manifest_string, blobs) =
        string_manifest_builder(&component_address, &address_bech32_encoder);
    let manifest = compile(
        &manifest_string,
        &network,
        BlobProvider::new_with_blobs(blobs),
    )
    .expect("Failed to compile manifest from manifest string");

    test_runner.execute_manifest(manifest, vec![virtual_badge_non_fungible_global_id])
}

fn test_manifest_with_restricted_minting_resource<F>(
    resource_type: ResourceType,
    string_manifest_builder: F,
) where
    F: Fn(
        &ComponentAddress,
        &ResourceAddress,
        &ResourceAddress,
        &AddressBech32Encoder,
    ) -> (String, Vec<Vec<u8>>),
{
    // Creating a new test runner
    let mut test_runner = TestRunner::builder().without_trace().build();

    // Creating the account component required for this test
    let (public_key, _, component_address) = test_runner.new_account(false);
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    // Defining the network and the bech32 encoder to use
    let network = NetworkDefinition::simulator();
    let address_bech32_encoder = AddressBech32Encoder::new(&network);

    // Creating the minter badge and the requested resource
    let minter_badge_resource_address =
        test_runner.create_fungible_resource(dec!(1), 0, component_address);

    let manifest = match resource_type {
        ResourceType::Fungible { divisibility } => ManifestBuilder::new()
            .create_fungible_resource(
                OwnerRole::None,
                false,
                divisibility,
                FungibleResourceRoles {
                    mint_roles: mint_roles! {
                        minter => rule!(require(minter_badge_resource_address));
                        minter_updater => rule!(deny_all);
                    },
                    ..Default::default()
                },
                metadata!(),
                None,
            )
            .build(),
        ResourceType::NonFungible { id_type } => ManifestBuilder::new()
            .create_non_fungible_resource(
                OwnerRole::None,
                id_type,
                false,
                NonFungibleResourceRoles {
                    mint_roles: mint_roles! {
                        minter => rule!(require(minter_badge_resource_address));
                        minter_updater => rule!(deny_all);
                    },
                    ..Default::default()
                },
                metadata!(),
                None::<BTreeMap<NonFungibleLocalId, SampleNonFungibleData>>,
            )
            .build(),
    };
    let result = test_runner.execute_manifest_ignoring_fee(manifest, vec![]);
    let mintable_non_fungible_resource_address =
        result.expect_commit(true).new_resource_addresses()[0].clone();

    // Run the function and get the manifest string
    let (manifest_string, blobs) = string_manifest_builder(
        &component_address,
        &minter_badge_resource_address,
        &mintable_non_fungible_resource_address,
        &address_bech32_encoder,
    );
    let manifest = compile(
        &manifest_string,
        &network,
        BlobProvider::new_with_blobs(blobs),
    )
    .expect("Failed to compile manifest from manifest string");

    test_runner
        .execute_manifest(manifest, vec![virtual_badge_non_fungible_global_id])
        .expect_commit_success();
}

fn test_manifest_with_additional_accounts<F>(accounts_required: u16, string_manifest_builder: F)
where
    F: Fn(&ComponentAddress, &[ComponentAddress], &AddressBech32Encoder) -> (String, Vec<Vec<u8>>),
{
    // Creating a new test runner
    let mut test_runner = TestRunner::builder().without_trace().build();

    // Creating the account component required for this test
    let (public_key, _, component_address) = test_runner.new_account(false);
    let virtual_badge_non_fungible_global_id = NonFungibleGlobalId::from_public_key(&public_key);

    // Creating the required accounts
    let accounts = (0..accounts_required)
        .map(|_| test_runner.new_account(false).2)
        .collect::<Vec<ComponentAddress>>();

    // Defining the network and the bech32 encoder to use
    let network = NetworkDefinition::simulator();
    let address_bech32_encoder = AddressBech32Encoder::new(&network);

    // Run the function and get the manifest string
    let (manifest_string, blobs) =
        string_manifest_builder(&component_address, &accounts, &address_bech32_encoder);
    let manifest = compile(
        &manifest_string,
        &network,
        BlobProvider::new_with_blobs(blobs),
    )
    .expect("Failed to compile manifest from manifest string");

    test_runner
        .execute_manifest(manifest, vec![virtual_badge_non_fungible_global_id])
        .expect_commit_success();
}

#[derive(ScryptoSbor, NonFungibleData, ManifestSbor)]
struct SampleNonFungibleData {}
