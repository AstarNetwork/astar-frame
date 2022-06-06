use std::assert_matches::assert_matches;

use crate::mock::*;
use crate::*;

use fp_evm::PrecompileFailure;
use pallet_evm::PrecompileSet;
use precompile_utils::EvmDataWriter;
use sp_core::H160;

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_argument_count_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        // This selector is only three bytes long when four are required.
        let bogus_selector = vec![1u8, 2u8, 3u8];

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &bogus_selector,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert { output, ..}))
                if output == b"tried to parse selector out of bounds",
        );

        let input = EvmDataWriter::new_with_selector(Action::AssetsWithdraw)
            .write(sp_core::H256::repeat_byte(0xF1))
            .build();

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert { output, ..}))
                if output == b"input doesn't match expected length",
        );
    });
}

#[test]
fn wrong_assets_len_or_fee_index_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        let input = EvmDataWriter::new_with_selector(Action::AssetsWithdraw)
            .write(vec![Address::from(H160::repeat_byte(0xF1))])
            .write(Vec::<U256>::new())
            .write(H256::repeat_byte(0xF1))
            .write(true)
            .write(U256::from(0))
            .write(U256::from(0))
            .build();

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert {output, ..}))
                if output == b"assets resolution failure"
        );

        let input2 = EvmDataWriter::new_with_selector(Action::AssetsWithdraw)
            .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
            .write(vec![U256::from(42000u64)])
            .write(H256::repeat_byte(0xF1))
            .write(true)
            .write(U256::from(0))
            .write(U256::from(2))
            .build();

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input2,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert {output, ..}))
                if output == b"bad fee index"
        );
    });
}

#[test]
fn correct_arguments_works() {
    ExtBuilder::default().build().execute_with(|| {
        let input = EvmDataWriter::new_with_selector(Action::AssetsWithdraw)
            .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
            .write(vec![U256::from(42000u64)])
            .write(H256::repeat_byte(0xF1))
            .write(true)
            .write(U256::from(0))
            .write(U256::from(0))
            .build();

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Ok(PrecompileOutput { exit_status, ..}))
                if exit_status == pallet_evm::ExitSucceed::Returned
        );
    });
}
