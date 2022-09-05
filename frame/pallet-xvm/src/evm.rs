//! EVM support for XVM pallet.

use crate::*;
use sp_core::{H160, U256};
use sp_runtime::traits::Get;

pub struct EVM<I, T, C>(sp_std::marker::PhantomData<(I, T, C)>);

impl<VmId, I, T, C> SyncVM<VmId, T::AccountId> for EVM<I, T, C>
where
    I: Get<VmId>,
    T: pallet_evm::Config + frame_system::Config,
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
            target: "xvm::EVM::xvm_call",
            "Start EVM XVM: {:?}, {:?}, {:?}, {:?}", 
            from, to, input, metadata,
        );
        let data = C::convert(input, metadata)?;
        let value = U256::from(0u64);
        let max_fee_per_gas = U256::from(345089869u64);
        let gas_limit = 4000000u64;
        let nonce = frame_system::Pallet::<T>::account(from.clone()).nonce;
        let evm_to = Decode::decode(&mut to.as_ref())
            .map_err(|_| b"`to` argument decode failure".to_vec())?;

        let res = pallet_evm::Pallet::<T>::call(
            frame_support::dispatch::RawOrigin::Root.into(),
            H160::from_slice(&from.encode()[0..20]),
            H160::from_slice(&to[0..20]),
            data,
            value,
            gas_limit,
            max_fee_per_gas,
            None,
            None,
            Vec::new(),
        );

        log::trace!(
            target: "xvm::EVM::xvm_call",
            "EVM XVM call result: {:?}", res
        );

        // TODO: return error if call failure
        // TODO: return value in case of constant / view call
        Ok(Default::default())
    }
}
