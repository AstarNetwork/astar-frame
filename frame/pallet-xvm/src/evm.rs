//! EVM support for XVM pallet.

use crate::*;
use pallet_evm::GasWeightMapping;
use sp_core::{H160, U256};
use sp_runtime::traits::Get;

/// EVM adapter for XVM calls.
///
/// This adapter supports generic XVM calls and encode it into EVM native calls
/// using Solidity ABI codec (https://docs.soliditylang.org/en/v0.8.16/abi-spec.html).
pub struct EVM<I, T>(sp_std::marker::PhantomData<(I, T)>);

impl<I, T> SyncVM<T::AccountId> for EVM<I, T>
where
    I: Get<VmId>,
    T: pallet_evm::Config + frame_system::Config,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(context: XvmContext, from: T::AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult {
        log::trace!(
            target: "xvm::EVM::xvm_call",
            "Start EVM XVM: {:?}, {:?}, {:?}",
            from, to, input,
        );
        let value = U256::zero();

        // Tells the EVM executor that no fees should be charged for this execution.
        let max_fee_per_gas = U256::zero();
        let gas_limit = T::GasWeightMapping::weight_to_gas(context.max_weight);
        log::trace!(
            target: "xvm::EVM::xvm_call",
            "EVM xvm call gas limit: {:?} or as weight: {:?}", gas_limit, context.max_weight);
        let evm_to = Decode::decode(&mut to.as_ref()).map_err(|_| XvmCallError {
            error: XvmError::EncodingFailure,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;

        let res = pallet_evm::Pallet::<T>::call(
            frame_support::dispatch::RawOrigin::Root.into(),
            H160::from_slice(&from.encode()[0..20]),
            evm_to,
            input,
            value,
            gas_limit,
            max_fee_per_gas,
            None,
            None,
            Vec::new(),
        )
        .map_err(|e| {
            let consumed_weight = if let Some(weight) = e.post_info.actual_weight {
                weight.ref_time()
            } else {
                context.max_weight.ref_time()
            };
            XvmCallError {
                error: XvmError::ExecutionError(Into::<&str>::into(e.error).into()),
                consumed_weight,
            }
        })?;

        log::trace!(
            target: "xvm::EVM::xvm_call",
            "EVM XVM call result: {:?}", res
        );

        Ok(XvmCallOk {
            output: Default::default(), // TODO: Fill output vec with response from the call
            consumed_weight: 42u64, // TODO: res.actual_weight.map(|x| x.ref_time()).unwrap_or(context.max_weight),
        })
    }
}
