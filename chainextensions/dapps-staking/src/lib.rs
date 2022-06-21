#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::DispatchError;

use codec::Encode;
use frame_support::log::{error, trace};
use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};
use pallet_dapps_staking;

pub trait AstarChainExtension {
    fn execute_func<E, R>(
        func_id: u32,
        env: Environment<E, InitState>,
    ) -> Result<(), DispatchError>
    where
        E: Ext,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
        R: pallet_dapps_staking::Config;
}

pub struct DappsStakingExtension;
impl AstarChainExtension for DappsStakingExtension {
    fn execute_func<E, R>(func_id: u32, env: Environment<E, InitState>) -> Result<(), DispatchError>
    where
        E: Ext,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
        R: pallet_dapps_staking::Config,
    {
        let mut env = env.buf_in_buf_out();
        match func_id {
            // DappsStaking - current_era()
            1 => {
                let current_era = pallet_dapps_staking::CurrentEra::<R>::get();
                let current_era_encoded = current_era.encode();
                trace!(
                    target: "runtime",
                    "[ChainExtension]|call|func_id:{:} current_era:{:?}",
                    func_id,
                    &current_era_encoded
                );

                env.write(&current_era_encoded, false, None).map_err(|_| {
                    DispatchError::Other(
                        "ChainExtension DappsStakingExtension failed to write result",
                    )
                })?;
            }

            // DappsStaking - general_era_info()
            2 => {
                let arg: u32 = env.read_as()?;
                let era_info = pallet_dapps_staking::GeneralEraInfo::<R>::get(arg)
                    .ok_or(DispatchError::Other("general_era_info call failed"));
                let era_info_encoded = era_info.encode();
                trace!(
                    target: "runtime",
                    "[ChainExtension]|call|func_id:{:} era_info_encoded:{:?}",
                    func_id,
                    &era_info_encoded
                );

                env.write(&era_info_encoded, false, None).map_err(|_| {
                    DispatchError::Other(
                        "ChainExtension DappsStakingExtension failed to write result",
                    )
                })?;
            }
            _ => {
                error!("Called an unregistered `func_id`: {:}", func_id);
                return Err(DispatchError::Other(
                    "DappsStakingExtension: Unimplemented func_id",
                ));
            }
        }
        Ok(())
    }
}
