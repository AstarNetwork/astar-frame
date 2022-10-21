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

/// XVM chain extension.
pub struct XvmExtension<T>(PhantomData<T>);

impl<T> Default for XvmExtension<T> {
    fn default() -> Self {
        XvmExtension(PhantomData)
    }
}

impl<T> ChainExtension<T> for XvmExtension<T>
where
    T: pallet_contracts::Config + pallet_xvm::Config,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            XvmFuncId::XvmCall => {
                // We need to immediately charge for the worst case scenario. Gas equals Weight in pallet-contracts context.
                let remaining_weight = env.ext().gas_meter().gas_left();
                let charged_weight = env.charge_weight(remaining_weight)?;

                let caller = env.ext().caller().clone();

                let XvmCallArgs { vm_id, to, input } = env.read_as_unbounded(env.in_len())?;

                let _origin_address = env.ext().address().clone();
                let _value = env.ext().value_transferred();
                let xvm_context = XvmContext {
                    id: vm_id,
                    max_weight: remaining_weight,
                    env: None,
                };

                let call_result = pallet_xvm::Pallet::<T>::xvm_call(
                    RawOrigin::Signed(caller).into(),
                    xvm_context,
                    to,
                    input,
                );

                // Adjust the actual weight used by the call if needed.
                let actual_weight = match call_result {
                    Ok(e) => e.actual_weight,
                    Err(e) => e.post_info.actual_weight,
                };
                if let Some(actual_weight) = actual_weight {
                    env.adjust_weight(charged_weight, actual_weight);
                }

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
