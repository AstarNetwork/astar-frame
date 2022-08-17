#![cfg_attr(not(feature = "std"), no_std)]
use pallet_contracts::chain_extension::{
    Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use sp_runtime::DispatchError;

/// A Trait used to implement a chain-extension for a pallet
///
/// To create a chain-extension for a pallet this trait must be implemented
///
/// T is the Config trait of the pallet
pub trait ChainExtensionExec<T: SysConfig> {
    fn execute_func<E>(
        func_id: u32,
        env: Environment<E, InitState>,
    ) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId:
            UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]> + From<[u8; 32]>;
}
