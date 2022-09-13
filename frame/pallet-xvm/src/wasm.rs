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

impl<I, T> SyncVM<VmId, T::AccountId> for WASM<I, T>
where
    I: Get<VmId>,
    T: pallet_contracts::Config + frame_system::Config,
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
    <BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(
        _context: XvmContext<VmId>,
        from: T::AccountId,
        to: Vec<u8>,
        input: Vec<u8>,
    ) -> XvmResult {
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "Start WASM XVM: {:?}, {:?}, {:?}, {:?}",
            from, to, input,
        );
        let gas_limit = 500000000000;
        let dest = Decode::decode(&mut to.as_ref()).unwrap();
        let res = pallet_contracts::Pallet::<T>::call(
            frame_support::dispatch::RawOrigin::Signed(from).into(),
            dest,
            Default::default(),
            gas_limit,
            None,
            input,
        )
        .map_err(|_| XvmCallError {
            error: XvmError::ExecutionError(Vec::default()), // TODO: make error mapping make more sense
            consumed_weight: PLACEHOLDER_WEIGHT,             // TODO: get correct weight?
        })?;

        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM XVM call result: {:?}", res
        );

        // TODO: return error if call failure
        // TODO: return value in case of constant / view call
        Ok(XvmCallOk {
            output: Default::default(), // TODO: vec should be filled with data in case of query? Should be generic probably.
            consumed_weight: res.actual_weight.unwrap_or(PLACEHOLDER_WEIGHT), // TODO: this should be max static weight if `None`
        })
    }
}
