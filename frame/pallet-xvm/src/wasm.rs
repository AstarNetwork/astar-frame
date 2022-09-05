//! WASM (substrate contracts) support for XVM pallet.

use crate::*;
use codec::HasCompact;
use frame_support::traits::Currency;
use pallet_contracts::chain_extension::UncheckedFrom;
use scale_info::TypeInfo;
use sp_runtime::traits::Get;
use sp_std::fmt::Debug;

pub struct WASM<I, T, C>(sp_std::marker::PhantomData<(I, T, C)>);

type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

impl<VmId, I, T, C> SyncVM<VmId, T::AccountId> for WASM<I, T, C>
where
    I: Get<VmId>,
    T: pallet_contracts::Config + frame_system::Config,
    T::AccountId: UncheckedFrom<T::Hash> + AsRef<[u8]>,
    <BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
    C: XvmCodec,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(
        context: XvmContext<VmId>,
        from: T::AccountId,
        to: Vec<u8>,
        input: Vec<u8>,
        metadata: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>> {
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "Start WASM XVM: {:?}, {:?}, {:?}, {:?}", 
            from, to, input, metadata,
        );
        let data = C::convert(input, metadata)?;
        let gas_limit = 500000000000;
        let dest = Decode::decode(&mut to.as_ref()).unwrap();
        let res = pallet_contracts::Pallet::<T>::call(
            frame_support::dispatch::RawOrigin::Signed(from).into(),
            dest,
            Default::default(),
            gas_limit,
            None,
            data,
        );

        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM XVM call result: {:?}", res
        );

        // TODO: return error if call failure
        // TODO: return value in case of constant / view call
        Ok(Default::default())
    }
}
