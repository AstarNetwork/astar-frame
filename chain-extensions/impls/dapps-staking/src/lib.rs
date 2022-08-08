#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{
    traits::{Saturating, Zero},
    DispatchError,
};

use chain_extension_traits::ChainExtensionExec;
use chain_extension_types::{
    Contract, DSError, DappsStakingAccountInput, DappsStakingEraInput, DappsStakingNominationInput,
    DappsStakingValueInput,
};
use codec::{Decode, Encode};
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{Environment, Ext, InitState, SysConfig, UncheckedFrom};
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
    Register,
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
            8 => Ok(DappsStakingFunc::Register),
            9 => Ok(DappsStakingFunc::BondAndStake),
            10 => Ok(DappsStakingFunc::UnbondAndUnstake),
            11 => Ok(DappsStakingFunc::WithdrawUnbonded),
            12 => Ok(DappsStakingFunc::ClaimStaker),
            13 => Ok(DappsStakingFunc::ClaimDapp),
            14 => Ok(DappsStakingFunc::SetRewardDestination),
            15 => Ok(DappsStakingFunc::NominationTransfer),
            _ => Err(DispatchError::Other(
                "DappsStakingExtension: Unimplemented func_id",
            )),
        }
    }
}

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

        match func_id {
            DappsStakingFunc::CurrentEra => {
                let era_index = pallet_dapps_staking::CurrentEra::<T>::get();
                env.write(&era_index.encode(), false, None).map_err(|_| {
                    DispatchError::Other(
                        "[ChainExtension] DappsStakingExtension CurrentEra failed to write result",
                    )
                })?;
            }

            DappsStakingFunc::UnbondingPeriod => {
                let unbonding_period = T::UnbondingPeriod::get();
                env.write(&unbonding_period.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension UnbondingPeriod failed to write result",
                        )
                    })?;
            }

            DappsStakingFunc::EraRewards => {
                let arg: u32 = env.read_as()?;
                let era_info = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("[ChainExtension] DappsStakingExtension EraRewards general_era_info call failed"));
                let reward = era_info.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.dapps)
                });
                env.write(&reward.encode(), false, None).map_err(|_| {
                    DispatchError::Other(
                        "[ChainExtension] DappsStakingExtension EraRewards failed to write result",
                    )
                })?;
            }

            DappsStakingFunc::EraStaked => {
                let arg: u32 = env.read_as()?;
                let era_info = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("[ChainExtension] DappsStakingExtension EraStaked general_era_info call failed"));
                let staked_amount = era_info.map_or(Zero::zero(), |r| r.staked);
                env.write(&staked_amount.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension EraStaked failed to write result",
                        )
                    })?;
            }

            DappsStakingFunc::StakedAmount => {
                let staker: T::AccountId = env.read_as()?;
                let ledger = pallet_dapps_staking::Ledger::<T>::get(&staker);
                env.write(&ledger.locked.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension StakedAmount failed to write result",
                        )
                    })?;
            }

            DappsStakingFunc::StakedAmountOnContract => {
                let args: DappsStakingAccountInput = env.read_as()?;
                let staker = T::AccountId::decode(&mut args.staker.as_ref()).unwrap();
                let contract = Self::decode_smart_contract(args.contract)?;
                let staking_info =
                    pallet_dapps_staking::GeneralStakerInfo::<T>::get(&staker, &contract);
                let staked_amount = staking_info.latest_staked_value();
                env.write(&staked_amount.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension StakedAmountOnContract failed to write result",
                        )
                    })?;
            }

            DappsStakingFunc::ReadContractStake => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let current_era = pallet_dapps_staking::CurrentEra::<T>::get();
                let staking_info =
                    pallet_dapps_staking::Pallet::<T>::contract_stake_info(&contract, current_era)
                        .unwrap_or_default();
                let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);
                env.write(&total.encode(), false, None).map_err(|_| {
                    DispatchError::Other(
                        "[ChainExtension] DappsStakingExtension ReadContractStake failed to write result",
                    )
                })?;
            }

            DappsStakingFunc::Register => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::register();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::register(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );

                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension Register failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension Register failed to write result")
                        })?;
                        Ok(())
                    }
                };
            }

            DappsStakingFunc::BondAndStake => {
                let args: DappsStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = Self::decode_smart_contract(args.contract)?;
                let value: BalanceOf<T> = args.value;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::bond_and_stake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::bond_and_stake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension BondAndStake failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension BondAndStake failed to write result")
                        })?;
                        Ok(())
                    }
                };
            }

            DappsStakingFunc::UnbondAndUnstake => {
                let args: DappsStakingValueInput<BalanceOf<T>> = env.read_as()?;
                let contract = Self::decode_smart_contract(args.contract)?;
                let value: BalanceOf<T> = args.value;

                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::unbond_and_unstake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::unbond_and_unstake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension UnbondAndUnstake failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension UnbondAndUnstake failed to write result")
                        })?;
                        Ok(())
                    }
                };
            }

            DappsStakingFunc::WithdrawUnbonded => {
                let caller = env.ext().caller().clone();

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::bond_and_stake();
                env.charge_weight(base_weight)?;

                let call_result = pallet_dapps_staking::Pallet::<T>::withdraw_unbonded(
                    RawOrigin::Signed(caller).into(),
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension WithdrawUnbonded failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension WithdrawUnbonded failed to write result")
                        })?;
                        Ok(())
                    }
                };
            }

            DappsStakingFunc::ClaimStaker => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let base_weight = T::WeightInfo::claim_staker_with_restake()
                    .max(T::WeightInfo::claim_staker_without_restake());
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_staker(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension ClaimStaker failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension ClaimStaker failed to write result")
                        })?;
                        Ok(())
                    }
                };
            }

            DappsStakingFunc::ClaimDapp => {
                let args: DappsStakingEraInput = env.read_as()?;
                let contract = Self::decode_smart_contract(args.contract)?;
                let era: u32 = args.era;

                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::claim_dapp();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_dapp(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    era,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension ClaimDapp failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension ClaimDapp failed to write result")
                        })?;
                        Ok(())
                    }
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
                    let res =
                        Result::<(), DSError>::Err(DSError::RewardDestinationValueOutOfBounds);
                    env.write(&res.encode(), false, None).map_err(|_| {
                        DispatchError::Other("[ChainExtension] DappsStakingExtension SetRewardDestination failed to write result")
                    })?;
                    return Err(DispatchError::Other(
                        "[ChainExtension] DappsStakingExtension SetRewardDestination unexpected reward destination value",
                    ));
                };

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::set_reward_destination(
                    RawOrigin::Signed(caller).into(),
                    reward_destination,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension SetRewardDestination failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension SetRewardDestination failed to write result")
                        })?;
                        Ok(())
                    }
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

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::nomination_transfer(
                    RawOrigin::Signed(caller).into(),
                    origin_smart_contract,
                    value,
                    target_smart_contract,
                );
                return match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension NominationTransfer failed to write result")
                        })?;
                        Err(e.error)
                    }
                    _ => {
                        let res = Result::<(), DSError>::Ok(());
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other("[ChainExtension] DappsStakingExtension NominationTransfer failed to write result")
                        })?;
                        Ok(())
                    }
                };
            }
        }

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
