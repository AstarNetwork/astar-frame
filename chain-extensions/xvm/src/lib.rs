#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use pallet_xvm::XvmContext;
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;

use xvm_chain_extension_types::{XvmCallArgs, XvmExecutionResult};

enum XvmFuncId {
    XvmCall,
    // TODO: expand with other calls too
}

impl TryFrom<u16> for XvmFuncId {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(XvmFuncId::XvmCall),
            _ => Err(DispatchError::Other(
                "Unsupported func id in Xvm chain extension",
            )),
        }
    }
}

pub struct XvmExtension<T>(PhantomType<T>);

impl<T> ChainExtension<T> for XvmExtension<T>
where
    T: pallet_contracts::Config + pallet_xvm::Config,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = Runtime>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            XvmFuncId::XvmCall => {
                // TODO: correct weight calculation directly from pallet!
                let weight = Weight::from_ref_time(1_000_000_000);
                env.charge_weight(weight)?;

                // Prepare parameters
                let caller = env.ext().caller().clone();

                let XvmCallArgs {
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
                    id: T::VmId::from(vm_id),
                    env: None,
                };

                let call_result = pallet_xvm::Pallet::<T>::xvm_call(
                    RawOrigin::Signed(caller).into(),
                    xvm_context,
                    to,
                    input,
                    metadata,
                );

                // TODO: We need to know how much of gas was spent in the other call and update the gas meter!
                // let consumed_xvm_weight = ...;
                // env.charge_weight(consumed_xvm_weight)?;

                return match call_result {
                    Err(e) => {
                        let mapped_error = XvmExecutionResult::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(XvmExecutionResult::Success as u32)),
                };
            }
        }
    }
}
