#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use codec::Decode;
use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use pallet_xvm::XvmContext;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use precompile_utils::{
    revert, succeed, Bytes, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt,
    RuntimeHelper,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    XvmCall = "xvm_call(bytes,bytes,bytes)",
}

/// A precompile that expose XVM related functions.
pub struct XvmPrecompile<T>(PhantomData<T>);

impl<R> Precompile for XvmPrecompile<R>
where
    R: pallet_evm::Config + pallet_xvm::Config,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    <R as frame_system::Config>::Call:
        From<pallet_xvm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XVM precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::NonPayable)?;

        match selector {
            // Dispatchables
            Action::XvmCall => Self::xvm_call(handle),
        }
    }
}

impl<R> XvmPrecompile<R>
where
    R: pallet_evm::Config + pallet_xvm::Config,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    <R as frame_system::Config>::Call:
        From<pallet_xvm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    fn xvm_call(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(4)?;

        // Read arguments and check it
        // TODO: This approach probably needs to be revised - does contract call need to specify gas/weight? Usually it is implicit.
        let context_raw = input.read::<Bytes>()?;
        let mut context: XvmContext<<R as pallet_xvm::Config>::VmId> =
            Decode::decode(&mut context_raw.0.as_ref())
                .map_err(|_| revert("can not decode XVM context"))?;

        // Fetch the remaining gas (weight) available for execution
        let remaining_gas = handle.remaining_gas();
        let remaining_weight = R::GasWeightMapping::gas_to_weight(remaining_gas);
        context.max_weight = remaining_weight;
        context.call_depth = 1;

        let call_to = input.read::<Bytes>()?.0;
        let call_input = input.read::<Bytes>()?.0;

        // Build call with origin.
        let origin = Some(R::AddressMapping::into_account_id(handle.context().caller)).into();
        let call = pallet_xvm::Call::<R>::xvm_call {
            context,
            to: call_to,
            input: call_input,
        };

        // Dispatch a call.
        // The underlying logic will handle updating used EVM gas based on the weight of the executed call.
        RuntimeHelper::<R>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }
}
