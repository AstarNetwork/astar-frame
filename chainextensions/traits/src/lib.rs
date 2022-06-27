#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::DispatchError;
use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};

pub trait ChainExtension<R: SysConfig> {
    fn execute_func<E>(
        func_id: u32,
        env: Environment<E, InitState>,
    ) -> Result<(), DispatchError>
    where
        E: Ext<T = R>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>;
}
