#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{
    traits::{Saturating, Zero},
    DispatchError,
};

use chain_extension_trait::ChainExtensionExec;
use codec::{Decode, Encode};
use dapps_staking_chain_extension_types::{
    Contract, DSError, DappsStakingAccountInput, DappsStakingEraInput, DappsStakingNominationInput,
    DappsStakingValueInput,
};
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use pallet_dapps_staking::{RewardDestination, WeightInfo};
use sp_std::marker::PhantomData;

type BalanceOf<T> = <<T as pallet_dapps_staking::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

enum DappsStakingFunc {
    CurrentEra,
    UnbondingPeriod,
    EraRewards,
    EraStaked,
    StakedAmount,
    StakedAmountOnContract,
    ReadContractStake,
    BondAndStake,
    UnbondAndUnstake,
    WithdrawUnbonded,
    ClaimStaker,
    ClaimDapp,
    SetRewardDestination,
    NominationTransfer,
}

impl TryFrom<u32> for DappsStakingFunc {
    type Error = DispatchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DappsStakingFunc::CurrentEra),
            2 => Ok(DappsStakingFunc::UnbondingPeriod),
            3 => Ok(DappsStakingFunc::EraRewards),
            4 => Ok(DappsStakingFunc::EraStaked),
            5 => Ok(DappsStakingFunc::StakedAmount),
            6 => Ok(DappsStakingFunc::StakedAmountOnContract),
            7 => Ok(DappsStakingFunc::ReadContractStake),
            8 => Ok(DappsStakingFunc::BondAndStake),
            9 => Ok(DappsStakingFunc::UnbondAndUnstake),
            10 => Ok(DappsStakingFunc::WithdrawUnbonded),
            11 => Ok(DappsStakingFunc::ClaimStaker),
            12 => Ok(DappsStakingFunc::ClaimDapp),
            13 => Ok(DappsStakingFunc::SetRewardDestination),
            14 => Ok(DappsStakingFunc::NominationTransfer),
            _ => Err(DispatchError::Other(
                "DappsStakingExtension: Unimplemented func_id",
            )),
        }
    }
}

pub struct DappsStakingExtension<R>(PhantomData<R>);

impl<T: pallet_dapps_staking::Config> ChainExtensionExec<T> for DappsStakingExtension<T> {
    fn execute_func<E>(
        func_id: u32,
        env: Environment<E, InitState>,
    ) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId:
            UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]> + From<[u8; 32]>,
    {
        let func_id = DappsStakingFunc::try_from(func_id)?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            DappsStakingFunc::CurrentEra => {
                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let era_index = pallet_dapps_staking::CurrentEra::<T>::get();
                env.write(&era_index.encode(), false, None)?;
            }

            DappsStakingFunc::UnbondingPeriod => {
                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let unbonding_period = T::UnbondingPeriod::get();
                env.write(&unbonding_period.encode(), false, None)?;
            }

            DappsStakingFunc::EraRewards => {
                let arg: u32 = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let era_info = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg);
                let reward = era_info.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.dapps)
                });
                env.write(&reward.encode(), false, None)?;
            }

            DappsStakingFunc::EraStaked => {
                let arg: u32 = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let era_info = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg);
                let staked_amount = era_info.map_or(Zero::zero(), |r| r.staked);
                env.write(&staked_amount.encode(), false, None)?;
            }

            DappsStakingFunc::StakedAmount => {
                let staker: T::AccountId = env.read_as()?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let ledger = pallet_dapps_staking::Ledger::<T>::get(&staker);
                env.write(&ledger.locked.encode(), false, None)?;
            }

            DappsStakingFunc::StakedAmountOnContract => {
                let args: DappsStakingAccountInput = env.read_as()?;
                let staker = T::AccountId::decode(&mut args.staker.as_ref()).unwrap();
                let contract = Self::decode_smart_contract(args.contract)?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight)?;

                let staking_info =
                    pallet_dapps_staking::GeneralStakerInfo::<T>::get(&staker, &contract);
                let staked_amount = staking_info.latest_staked_value();
                env.write(&staked_amount.encode(), false, None)?;
            }

            DappsStakingFunc::ReadContractStake => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;

                let base_weight = <T as frame_system::Config>::DbWeight::get().reads(1);
                env.charge_weight(base_weight.saturating_add(base_weight))?;

                let current_era = pallet_dapps_staking::CurrentEra::<T>::get();
                let staking_info =
                    pallet_dapps_staking::Pallet::<T>::contract_stake_info(&contract, current_era)
                        .unwrap_or_default();
                let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);
                env.write(&total.encode(), false, None)?;
            }

            DappsStakingFunc::BondAndStake => {
                let args: DappsStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = Self::decode_smart_contract(args.contract)?;
                let value: BalanceOf<T> = args.value;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::bond_and_stake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::bond_and_stake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::UnbondAndUnstake => {
                let args: DappsStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = Self::decode_smart_contract(args.contract)?;
                let value: BalanceOf<T> = args.value;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::unbond_and_unstake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::unbond_and_unstake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::WithdrawUnbonded => {
                let caller = env.ext().address().clone();

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::withdraw_unbonded();
                env.charge_weight(base_weight)?;

                let call_result = pallet_dapps_staking::Pallet::<T>::withdraw_unbonded(
                    RawOrigin::Signed(caller).into(),
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::ClaimStaker => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;

                let base_weight = T::WeightInfo::claim_staker_with_restake()
                    .max(T::WeightInfo::claim_staker_without_restake());
                let charged_weight = env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_staker(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );

                let actual_weight = match call_result {
                    Ok(e) => e.actual_weight,
                    Err(e) => e.post_info.actual_weight,
                };
                if let Some(actual_weight) = actual_weight {
                    env.adjust_weight(charged_weight, actual_weight);
                }

                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::ClaimDapp => {
                let args: DappsStakingEraInput = env.read_as()?;
                let contract = Self::decode_smart_contract(args.contract)?;
                let era: u32 = args.era;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::claim_dapp();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_dapp(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    era,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::SetRewardDestination => {
                let reward_destination_raw: u8 = env.read_as()?;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::set_reward_destination();
                env.charge_weight(base_weight)?;

                // Transform raw value into dapps staking enum
                let reward_destination = if reward_destination_raw == 0 {
                    RewardDestination::FreeBalance
                } else if reward_destination_raw == 1 {
                    RewardDestination::StakeBalance
                } else {
                    let error = DSError::RewardDestinationValueOutOfBounds;
                    return Ok(RetVal::Converging(error as u32));
                };

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::set_reward_destination(
                    RawOrigin::Signed(caller).into(),
                    reward_destination,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }

            DappsStakingFunc::NominationTransfer => {
                let args: DappsStakingNominationInput<BalanceOf<T>> = env.read_as()?;
                let origin_smart_contract = Self::decode_smart_contract(args.origin_contract)?;
                let target_smart_contract = Self::decode_smart_contract(args.target_contract)?;
                let value: BalanceOf<T> = args.value;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::nomination_transfer();
                env.charge_weight(base_weight)?;

                let caller = env.ext().address().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::nomination_transfer(
                    RawOrigin::Signed(caller).into(),
                    origin_smart_contract,
                    value,
                    target_smart_contract,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(DSError::Success as u32)),
                };
            }
        }

        Ok(RetVal::Converging(DSError::Success as u32))
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
