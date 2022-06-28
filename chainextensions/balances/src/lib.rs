#![cfg_attr(not(feature = "std"), no_std)]
use chain_extension_traits::ChainExtensionExec;
use codec::Encode;
use frame_support::log::{error, trace};
use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};
use sp_runtime::DispatchError;

pub struct BalancesExtension;

impl<T: pallet_balances::Config> ChainExtensionExec<T> for BalancesExtension {
    fn execute_func<E>(func_id: u32, env: Environment<E, InitState>) -> Result<(), DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let mut env = env.buf_in_buf_out();
        match func_id {
            // Balances - free_balance()
            1 => {
                let arg: <E::T as SysConfig>::AccountId = env.read_as()?;
                let free_balance = pallet_balances::Pallet::<T>::free_balance(arg.clone());
                let free_balance_encoded = free_balance.encode();
                trace!(
                    target: "runtime",
                    "[ChainExtension]|call|func_id:{:} free_balance_encoded:{:?}, arg:{:?}",
                    func_id,
                    free_balance_encoded,
                    arg
                );
                env.write(&free_balance_encoded, false, None).map_err(|_| {
                    DispatchError::Other("ChainExtension failed to call free_balance")
                })?;
            }
            // balances - usable_balance()
            2 => {
                let arg: <E::T as SysConfig>::AccountId = env.read_as()?;
                let usable_balance = pallet_balances::Pallet::<T>::usable_balance(arg.clone());
                let usable_balance_encoded = usable_balance.encode();
                trace!(
                    target: "runtime",
                    "[ChainExtension]|call|func_id:{:} usable_balance_encoded:{:?}, arg:{:?}",
                    func_id,
                    usable_balance_encoded,
                    arg
                );
                env.write(&usable_balance_encoded, false, None)
                    .map_err(|_| {
                        DispatchError::Other("ChainExtension failed to call usable_balance")
                    })?;
            }
            _ => {
                error!("Called an unregistered `func_id`: {:}", func_id);
                return Err(DispatchError::Other(
                    "BalancesExtension: Unimplemented func_id",
                ));
            }
        }
        Ok(())
    }
}
