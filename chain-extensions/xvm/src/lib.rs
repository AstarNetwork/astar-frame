#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::{Decode, Encode};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use pallet_xvm::XvmContext;
use sp_runtime::DispatchError;
use sp_std::vec::Vec;

enum XvmFuncId {
    XvmCall,
    // TODO: expand with other calls too
}

impl TryFrom<u32> for XvmFuncId {
    type Error = DispatchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(XvmFuncId::XvmCall),
            _ => Err(DispatchError::Other(
                "Unsupported func id in Xvm chain extension",
            )),
        }
    }
}

/// Since we cannot read multiple variable length params, we need to resort to this approach. TODO: can it be improved?
#[derive(Encode, Decode)]
struct CustomParams<T: pallet_xvm::Config> {
    /// virtual machine identifier
    vm_id: T::VmId,
    /// Call destination (e.g. address)
    to: Vec<u8>,
    /// Encoded call params
    input: Vec<u8>,
    /// Metadata for the encoded params
    metadata: Vec<u8>,
}

pub struct XvmExtension;

impl<T> ChainExtension<T> for XvmExtension
where
    T: pallet_contracts::Config + pallet_xvm::Config,
{
    fn call<E>(
        func_id: u32,
        env: Environment<'_, '_, E, InitState>,
    ) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let func_id: XvmFuncId = func_id.try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            XvmFuncId::XvmCall => {
                // TODO: correct weight calculation directly from pallet!
                let weight = 1_000_000_000;
                env.charge_weight(weight)?;

                // Prepare parameters
                let caller = env.ext().caller().clone();

                let CustomParams::<T> {
                    vm_id,
                    to,
                    input,
                    metadata,
                } = env.read_as_unbounded(env.in_len())?;

                // TODO: rethink this? How do we get valid env in chain extension? Need to know which data to encode.
                // gas limit, taking into account used gas so far?
                let _origin_address = env.ext().address().clone();
                let _value = env.ext().value_transferred();
                let _gas_limit = env.ext().gas_meter().gas_left();
                let xvm_context = XvmContext {
                    id: vm_id,
                    env: None,
                };

                pallet_xvm::Pallet::<T>::xvm_call(
                    RawOrigin::Signed(caller).into(),
                    xvm_context,
                    to,
                    input,
                    metadata,
                )?;

                // TODO: We need to know how much of gas was spent in the other call and update the gas meter!
                // let consumed_xvm_weight = ...;
                // env.charge_weight(consumed_xvm_weight)?;

                Ok(RetVal::Converging(0))
            }
        }
    }
}
