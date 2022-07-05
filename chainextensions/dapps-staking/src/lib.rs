#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{
    traits::{Saturating, Zero},
    DispatchError,
};

use chain_extension_traits::ChainExtensionExec;
use codec::{Decode, Encode};
use frame_support::log::trace;
use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};
use pallet_dapps_staking;
use sp_core::H160;
use sp_std::marker::PhantomData;

/// This is only used to encode SmartContract enum
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<Account> {
    // EVM smart contract instance.
    Evm(H160),
    // Wasm smart contract instance. Not used in this precompile
    Wasm(Account),
}

enum DappsStakingFunc {
    CurrentEra = 1,
    UnbondingPeriod = 2,
    EraRewards = 3,
    EraStaked = 4,
    StakedAmount = 5,
    StakedAmountOnContract = 6,
    ReadContractStake = 7,
}

impl TryFrom<u32> for DappsStakingFunc {
    type Error = DispatchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => return Ok(DappsStakingFunc::CurrentEra),
            2 => return Ok(DappsStakingFunc::UnbondingPeriod),
            3 => return Ok(DappsStakingFunc::EraRewards),
            4 => return Ok(DappsStakingFunc::EraStaked),
            5 => return Ok(DappsStakingFunc::StakedAmount),
            6 => return Ok(DappsStakingFunc::StakedAmountOnContract),
            7 => return Ok(DappsStakingFunc::ReadContractStake),
            _ => {
                return Err(DispatchError::Other(
                    "DappsStakingExtension: Unimplemented func_id",
                ))
            }
        }
    }
}

// pub struct DappsStakingExtension;
pub struct DappsStakingExtension<R>(PhantomData<R>);

impl<T: pallet_dapps_staking::Config> ChainExtensionExec<T> for DappsStakingExtension<T> {
    fn execute_func<E>(func_id: u32, env: Environment<E, InitState>) -> Result<(), DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId:
            UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]> + From<[u8; 32]>,
    {
        let func_id = DappsStakingFunc::try_from(func_id)?;
        let mut env = env.buf_in_buf_out();
        let result_encoded;

        match func_id {
            // DappsStaking - read_current_era()
            DappsStakingFunc::CurrentEra => {
                let era = pallet_dapps_staking::CurrentEra::<T>::get();
                result_encoded = era.encode();
            }

            // DappsStaking - read_unbonding_period()
            DappsStakingFunc::UnbondingPeriod => {
                // let result = T::UnbondingPeriod::get();
                // result_encoded = result.encode();
                let arg: u32 = env.read_as()?;
                let read_reward = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("StakedAmount call failed"));
                let reward = read_reward.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.dapps)
                });
                result_encoded = reward.encode();
            }

            // DappsStaking - read_era_reward()
            DappsStakingFunc::EraRewards => {
                let arg: u32 = env.read_as()?;
                let read_reward = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("general_era_info call failed"));
                let reward = read_reward.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.dapps)
                });
                result_encoded = reward.encode();
            }

            // DappsStaking - read_era_staked()
            DappsStakingFunc::EraStaked => {
                let arg: u32 = env.read_as()?;
                let read_staked = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("general_era_info call failed"));
                let staked = read_staked.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.stakers)
                });
                result_encoded = staked.encode();
            }

            // DappsStaking - read_staked_amount()
            DappsStakingFunc::StakedAmount => {
                let staker: T::AccountId = env.read_as()?;
                let staked = pallet_dapps_staking::Ledger::<T>::get(&staker);
                result_encoded = staked.encode();
            }

            // DappsStaking - read_staked_amount_on_contract()
            DappsStakingFunc::StakedAmountOnContract => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let staker: T::AccountId = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;

                let staking_info =
                    pallet_dapps_staking::GeneralStakerInfo::<T>::get(&staker, &contract);
                result_encoded = staking_info.encode();
            }

            // DappsStaking - read_contract_stake()
            DappsStakingFunc::ReadContractStake => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let current_era = pallet_dapps_staking::CurrentEra::<T>::get();
                let staking_info =
                    pallet_dapps_staking::Pallet::<T>::contract_stake_info(&contract, current_era)
                        .unwrap_or_default();
                result_encoded = staking_info.encode();
            }
        }
        trace!(
            target: "runtime",
            "[ChainExtension] DappsStakingExtension result={:?}",
            &result_encoded
        );
        env.write(&result_encoded, false, None).map_err(|_| {
            DispatchError::Other("[ChainExtension] DappsStakingExtension failed to write result")
        })?;
        Ok(())
    }
}

impl<R> DappsStakingExtension<R> {
    /// Helper method to decode type SmartContract enum
    pub fn decode_smart_contract(
        contract_bytes: [u8; 32],
    ) -> Result<<R as pallet_dapps_staking::Config>::SmartContract, DispatchError>
    where
        R: pallet_dapps_staking::Config,
        R::AccountId: From<[u8; 32]>,
    {
        let account: R::AccountId = contract_bytes.into();
        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this chain extension,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = Contract::<R::AccountId>::Wasm(account).encode();

        // encoded enum will add one byte before the contract's address
        // therefore we need to decode len(u32) + 1 byte = 33
        let smart_contract = <R as pallet_dapps_staking::Config>::SmartContract::decode(
            &mut &contract_enum_encoded[..33],
        )
        .map_err(|_| {
            DispatchError::Other(
                "[ChainExtension] Error while decoding SmartContract in ChainExtension",
            )
        })?;

        Ok(smart_contract)
    }
}
