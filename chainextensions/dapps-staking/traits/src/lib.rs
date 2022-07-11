#![cfg_attr(not(feature = "std"), no_std)]
use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};
use sp_runtime::DispatchError;

pub trait ChainExtensionExec<T: SysConfig> {
    fn execute_func<E>(func_id: u32, env: Environment<E, InitState>) -> Result<(), DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId:
            UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]> + From<[u8; 32]>;
}
