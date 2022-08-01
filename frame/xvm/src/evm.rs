//! EVM support for XVM pallet.

use crate::*;
use sp_runtime::traits::Get;

struct EVM<I, T, C>(sp_std::marker::PhantomData<(I, T, C)>);

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
        let data = C::encode(input, metadata);
        let value = 0;
        let gas_limit = 4000000;
        let max_fee_per_gas = 1;
        let max_priority_fee_per_gas = 1;
        let nonce = frame_system::Pallet::<T>::account(from).nonce;
        let evm_to = Decode::decode(&mut to.as_ref())
            .map_err(|_| b"`to` argument decode failure".to_vec())?;

        pallet_evm::Pallet::<T>::call(
            Default::default(),
            from,
            to,
            data,
            value,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            nonce,
            Vec::new(),
        )
    }
}
