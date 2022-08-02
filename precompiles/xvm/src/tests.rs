use crate::mock::*;
use crate::*;

use precompile_utils::testing::*;
use precompile_utils::EvmDataWriter;

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_argument_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(42u64)
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"input doesn't match expected length");

        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(0u8)
                    .write(Bytes(b"".to_vec()))
                    .write(Bytes(b"".to_vec()))
                    .write(Bytes(b"".to_vec()))
                    .build(),
            )
            .expect_no_logs()
            .execute_reverts(|output| output == b"can not decode XVM context");
    })
}

#[test]
fn correct_arguments_works() {
    ExtBuilder::default().build().execute_with(|| {
        precompiles()
            .prepare_test(
                TestAccount::Alice,
                PRECOMPILE_ADDRESS,
                EvmDataWriter::new_with_selector(Action::XvmCall)
                    .write(Bytes(vec![0u8, 0u8]))
                    .write(Bytes(b"".to_vec()))
                    .write(Bytes(b"".to_vec()))
                    .write(Bytes(b"".to_vec()))
                    .build(),
            )
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(true).build());
    })
}
