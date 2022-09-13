//! EVM support for XVM pallet.

use crate::*;
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

    fn xvm_call(
        _context: XvmContext<VmId>,
        from: T::AccountId,
        to: Vec<u8>,
        input: Vec<u8>,
        metadata: Vec<u8>,
    ) -> XvmResult {
        log::trace!(
            target: "xvm::EVM::xvm_call",
            "Start EVM XVM: {:?}, {:?}, {:?}, {:?}",
            from, to, input, metadata,
        );
        let value = U256::from(0u64);
        let max_fee_per_gas = U256::from(3450898690u64);
        let gas_limit = 4000000u64;
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
        .map_err(|_| XvmCallError {
            error: XvmError::ExecutionError(Vec::default()), // TODO: make error mapping make more sense
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;

        log::trace!(
            target: "xvm::EVM::xvm_call",
            "EVM XVM call result: {:?}", res
        );

        // TODO: return error if call failure
        // TODO: return value in case of constant / view call
        Ok(XvmCallOk {
            output: Default::default(), // TODO: vec should be filled with data in case of query? Should be generic probably.
            consumed_weight: res.actual_weight.unwrap_or(PLACEHOLDER_WEIGHT), // TODO: this should be max static weight if `None`
        })
    }
}
