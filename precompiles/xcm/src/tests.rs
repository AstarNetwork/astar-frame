use crate::mock::*;
use crate::*;

use precompile_utils::testing::*;
use precompile_utils::EvmDataWriter;
use sp_core::H160;

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_assets_len_or_fee_index_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawNative)
                    .write(vec![Address::from(H160::repeat_byte(0xF1))])
                    .write(Vec::<U256>::new())
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"Assets resolution failure.");

        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawNative)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(2_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"Bad fee index.");
    });
}

#[test]
fn assets_withdraw_works() {
    ExtBuilder::default().build().execute_with(|| {
        // SS58
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawNative)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());

        // H160
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsWithdrawEvm)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(Address::from(H160::repeat_byte(0xDE)))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });
}

#[test]
fn remote_transact_works() {
    ExtBuilder::default().build().execute_with(|| {
        // SS58
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::RemoteTransact)
                    .write(U256::from(0_u64))
                    .write(true)
                    .write(Address::from(Runtime::asset_id_to_address(1_u128)))
                    .write(U256::from(367))
                    .write(vec![0xff_u8, 0xaa, 0x77, 0x00])
                    .write(U256::from(3_000_000_000u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });
}

#[test]
fn reserve_transfer_works() {
    ExtBuilder::default().build().execute_with(|| {
        // SS58
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsReserveTransferNative)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(H256::repeat_byte(0xF1))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());

        // H160
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::AssetsReserveTransferEvm)
                    .write(vec![Address::from(Runtime::asset_id_to_address(1u128))])
                    .write(vec![U256::from(42000u64)])
                    .write(Address::from(H160::repeat_byte(0xDE)))
                    .write(true)
                    .write(U256::from(0_u64))
                    .write(U256::from(0_u64))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    });
}
