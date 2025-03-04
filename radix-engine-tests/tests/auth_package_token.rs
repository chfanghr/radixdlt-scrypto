use radix_engine::errors::{RuntimeError, SystemModuleError};
use radix_engine::system::system_modules::auth::AuthError;
use radix_engine::types::*;
use scrypto_unit::*;
use transaction::prelude::*;

#[test]
fn can_call_self_with_package_token() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/package_token");

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package_address, "Factory", "create", manifest_args!())
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_commit_success();
}

#[test]
fn cannot_call_package_protected_function_without_package_token() {
    // Arrange
    let mut test_runner = TestRunner::builder().build();
    let package_address = test_runner.compile_and_publish("./tests/blueprints/package_token");
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_function(package_address, "Factory", "create_raw", manifest_args!())
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);
    let component_address = receipt.expect_commit(true).new_component_addresses()[0];

    // Act
    let manifest = ManifestBuilder::new()
        .lock_fee_from_faucet()
        .call_method(
            component_address,
            "set_address",
            manifest_args!(component_address),
        )
        .build();
    let receipt = test_runner.execute_manifest(manifest, vec![]);

    // Assert
    receipt.expect_specific_failure(|e| {
        matches!(
            e,
            RuntimeError::SystemModuleError(SystemModuleError::AuthError(
                AuthError::Unauthorized { .. }
            ))
        )
    })
}
