#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{Context, ExitSucceed, PrecompileOutput};
use pallet_evm::Precompile;
use sp_core::{crypto::UncheckedFrom, sr25519, H256};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use precompile_utils::{
    Bytes, EvmDataReader, EvmDataWriter, EvmResult, FunctionModifier, Gasometer,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    AssetsWithdraw = "assets_withdraw(address[],uint256[])",
}

/// A precompile to wrap substrate sr25519 functions.
pub struct Sr25519Precompile<Runtime>(PhantomData<Runtime>);

impl<Runtime: pallet_evm::Config> Precompile for Sr25519Precompile<Runtime> {
    fn execute(
        input: &[u8], //Reminder this is big-endian
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XCM precompile");

        let mut gasometer = Gasometer::new(target_gas);
        let gasometer = &mut gasometer;

        let (mut input, selector) = EvmDataReader::new_with_selector(gasometer, input)?;
        let input = &mut input;

        gasometer.check_function_modifier(context, is_static, FunctionModifier::Public)?;

        match selector {
            // Dispatchables
            Action::AssetsWithdraw => Self::assets_withdraw(input, gasometer, context),
        }
    }
}

impl<Runtime: pallet_evm::Config> Sr25519Precompile<Runtime> {
    fn assets_withdraw(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        _: &Context,
    ) -> EvmResult<PrecompileOutput> {
        // Bound check
        input.expect_arguments(gasometer, 6)?;

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(true).build(),
            logs: Default::default(),
        })
    }
}
