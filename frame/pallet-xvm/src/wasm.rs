//! WASM (substrate contracts) support for XVM pallet.

use crate::*;
use codec::HasCompact;
use frame_support::traits::Currency;
use pallet_contracts::chain_extension::UncheckedFrom;
use scale_info::TypeInfo;
use sp_runtime::traits::Get;
use sp_std::fmt::Debug;

pub struct WASM<I, T>(sp_std::marker::PhantomData<(I, T)>);

type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

impl<I, T> SyncVM<T::AccountId> for WASM<I, T>
where
    I: Get<VmId>,
    T: pallet_contracts::Config + frame_system::Config,
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
    <BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(context: XvmContext, from: T::AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult {
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "Start WASM XVM: {:?}, {:?}, {:?}",
            from, to, input,
        );
        let gas_limit = context.max_weight;
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM xvm call gas (weight) limit: {:?}", gas_limit);
        let dest = Decode::decode(&mut to.as_ref()).map_err(|_| XvmCallError {
            error: XvmError::EncodingFailure,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;
        let res = pallet_contracts::Pallet::<T>::call(
            frame_support::dispatch::RawOrigin::Signed(from).into(),
            dest,
            Default::default(),
            gas_limit.into(),
            None,
            input,
        )
        .map_err(|_e| XvmCallError {
            error: XvmError::ExecutionError(Vec::default()), // TODO: make error mapping make more sense
            consumed_weight: 42u64, //TODO: e.post_info.actual_weight.ref_time().unwrap_or(gas_limit),
        })?;

        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM XVM call result: {:?}", res
        );

        Ok(XvmCallOk {
            output: Default::default(), // TODO: Fill in with output from the call
            consumed_weight: 42u64, //TODO: e.post_info.actual_weight.ref_time().unwrap_or(gas_limit),
        })
    }
}
