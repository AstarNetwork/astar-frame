#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{
    traits::{Saturating, Zero},
    DispatchError, ModuleError,
};

use chain_extension_traits::ChainExtensionExec;
use chain_extension_types::{DSError, DappsStakingAccountInput, SmartContract};
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
    CurrentEra = 1,
    UnbondingPeriod = 2,
    EraRewards = 3,
    EraStaked = 4,
    StakedAmount = 5,
    StakedAmountOnContract = 6,
    ReadContractStake = 7,
    Register = 8,
    BondAndStake = 9,
    UnbondAndStake = 10,
    WithdrawUnbonded = 11,
    ClaimStaker = 12,
    ClaimDapp = 13,
    SetRewardDestination = 14,
    WithdrawFromUnregistered = 15,
    NominationTransfer = 16,
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
            8 => return Ok(DappsStakingFunc::Register),
            9 => return Ok(DappsStakingFunc::BondAndStake),
            10 => return Ok(DappsStakingFunc::UnbondAndStake),
            11 => return Ok(DappsStakingFunc::WithdrawUnbonded),
            12 => return Ok(DappsStakingFunc::ClaimStaker),
            13 => return Ok(DappsStakingFunc::ClaimDapp),
            14 => return Ok(DappsStakingFunc::SetRewardDestination),
            15 => return Ok(DappsStakingFunc::WithdrawFromUnregistered),
            16 => return Ok(DappsStakingFunc::NominationTransfer),
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
    fn execute_func<E>(
        func_id: u32,
        environment: Environment<E, InitState>,
    ) -> Result<(), DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId:
            UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]> + From<[u8; 32]>,
    {
        let func_id = DappsStakingFunc::try_from(func_id)?;
        let mut env = environment.buf_in_buf_out();

        match func_id {
            // DappsStaking - read_current_era()
            DappsStakingFunc::CurrentEra => {
                let result_to_encode = pallet_dapps_staking::CurrentEra::<T>::get();
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - read_unbonding_period()
            DappsStakingFunc::UnbondingPeriod => {
                let result_to_encode = T::UnbondingPeriod::get();
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - read_era_reward()
            DappsStakingFunc::EraRewards => {
                let arg: u32 = env.read_as()?;
                let read_reward = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("general_era_info call failed"));
                let result_to_encode = read_reward.map_or(Zero::zero(), |r| {
                    r.rewards.stakers.saturating_add(r.rewards.dapps)
                });
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - read_era_staked()
            DappsStakingFunc::EraStaked => {
                let arg: u32 = env.read_as()?;
                let read_staked = pallet_dapps_staking::GeneralEraInfo::<T>::get(arg)
                    .ok_or(DispatchError::Other("general_era_info call failed"));
                let result_to_encode = read_staked.map_or(Zero::zero(), |r| r.staked);
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - read_staked_amount()
            DappsStakingFunc::StakedAmount => {
                let staker: T::AccountId = env.read_as()?;
                let result_to_encode = pallet_dapps_staking::Ledger::<T>::get(&staker);
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - read_staked_amount_on_contract()
            DappsStakingFunc::StakedAmountOnContract => {
                let args: DappsStakingAccountInput<T::AccountId> = env.read_as()?;
                let staker: T::AccountId = args.staker;
                let contract = Self::decode_smart_contract2(args.contract)?;
                let staking_info =
                    pallet_dapps_staking::GeneralStakerInfo::<T>::get(&staker, &contract);
                let staked_amount = staking_info.latest_staked_value();
                env.write(&staked_amount.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - read_contract_stake()
            DappsStakingFunc::ReadContractStake => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let current_era = pallet_dapps_staking::CurrentEra::<T>::get();
                let result_to_encode =
                    pallet_dapps_staking::Pallet::<T>::contract_stake_info(&contract, current_era)
                        .unwrap_or_default();
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - register()
            DappsStakingFunc::Register => {
                sp_std::if_std! {println!(
                    "[ChainExtension] DappsStakingExtension Register entered"
                );}
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::register();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                sp_std::if_std! {println!(
                    "[ChainExtension] DappsStakingExtension Register contract {:?}, caller{:?}, weight {:?}",
                    contract, caller, base_weight
                );}
                let call_result = pallet_dapps_staking::Pallet::<T>::register(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );
                sp_std::if_std! {println!(
                    "[ChainExtension] DappsStakingExtension Register call_result {:?}",
                    call_result
                );}
                let _result_to_encode = match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other(
                                "[ChainExtension] DappsStakingExtension failed to write result",
                            )
                        })?;
                        return Err(DispatchError::from(e.error));
                    }
                    _ => Result::<(), DispatchError>::Ok(()),
                };

                sp_std::if_std! {println!(
                    "[ChainExtension] DappsStakingExtension Register result_to_encode {:?}",
                    _result_to_encode
                );}
            }

            // DappsStaking - bond_and_stake()
            DappsStakingFunc::BondAndStake => {
                // let args: DappsStakingContractAmount<T::AccountId> = env.read_as()?;
                // let contract = SmartContract::Wasm(args.contract);
                // let value: BalanceOf<T> = args.amount.into();
                let contract_bytes: [u8; 32] = env.read_as()?;
                let value: BalanceOf<T> = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::bond_and_stake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                sp_std::if_std! {println!(
                    "[ChainExtension] DappsStakingExtension BondAndStake contract {:?}, caller{:?}, weight {:?}, value {:?}",
                    contract, caller, base_weight, value
                );}
                let call_result = pallet_dapps_staking::Pallet::<T>::bond_and_stake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );

                // let result = Self::store_result_in_env::<E>(&env, call_result)?;
                // result

                let _result_to_encode = match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other(
                                "[ChainExtension] DappsStakingExtension failed to write result",
                            )
                        })?;
                        return Err(DispatchError::from(e.error));
                    }
                    _ => Result::<(), DispatchError>::Ok(()),
                };
            }

            // DappsStaking - unbond_and_unstake()
            DappsStakingFunc::UnbondAndStake => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let value: BalanceOf<T> = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::unbond_and_unstake();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::unbond_and_unstake(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    value,
                );
                let _result_to_encode = match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other(
                                "[ChainExtension] DappsStakingExtension failed to write result",
                            )
                        })?;
                        return Err(DispatchError::from(e.error));
                    }
                    _ => Result::<(), DispatchError>::Ok(()),
                };
            }

            // DappsStaking - withdraw_unbonded()
            DappsStakingFunc::WithdrawUnbonded => {
                let caller = env.ext().caller().clone();
                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::bond_and_stake();
                env.charge_weight(base_weight)?;

                let call_result = pallet_dapps_staking::Pallet::<T>::withdraw_unbonded(
                    RawOrigin::Signed(caller).into(),
                );
                let _result_to_encode = match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other(
                                "[ChainExtension] DappsStakingExtension failed to write result",
                            )
                        })?;
                        return Err(DispatchError::from(e.error));
                    }
                    _ => Result::<(), DispatchError>::Ok(()),
                };
            }

            // DappsStaking - claim_staker()
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
                let _result_to_encode = match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other(
                                "[ChainExtension] DappsStakingExtension failed to write result",
                            )
                        })?;
                        return Err(DispatchError::from(e.error));
                    }
                    _ => Result::<(), DispatchError>::Ok(()),
                };
            }

            // DappsStaking - claim_dapp()
            DappsStakingFunc::ClaimDapp => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let era: u32 = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let base_weight = <T as pallet_dapps_staking::Config>::WeightInfo::claim_dapp();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::claim_dapp(
                    RawOrigin::Signed(caller).into(),
                    contract,
                    era,
                );
                let _result_to_encode = match call_result {
                    Err(e) => {
                        let mapped_error = DSError::try_from(e.error)?;
                        let res = Result::<(), DSError>::Err(mapped_error);
                        env.write(&res.encode(), false, None).map_err(|_| {
                            DispatchError::Other(
                                "[ChainExtension] DappsStakingExtension failed to write result",
                            )
                        })?;
                        return Err(DispatchError::from(e.error));
                    }
                    _ => Result::<(), DispatchError>::Ok(()),
                };
            }

            // DappsStaking - set_reward_destination()
            DappsStakingFunc::SetRewardDestination => {
                let reward_destination_raw: u32 = env.read_as()?;
                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::set_reward_destination();
                env.charge_weight(base_weight)?;

                // Transform raw value into dapps staking enum
                let reward_destination = if reward_destination_raw == 0 {
                    RewardDestination::FreeBalance
                } else if reward_destination_raw == 1 {
                    RewardDestination::StakeBalance
                } else {
                    return Err(DispatchError::Other(
                        "[ChainExtension] Unexpected reward destination value",
                    ));
                };

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::set_reward_destination(
                    RawOrigin::Signed(caller).into(),
                    reward_destination,
                );
                let result_to_encode = match call_result {
                    Err(e) => Result::<(), DispatchError>::Err(DispatchError::from(e.error)),
                    _ => Result::<(), DispatchError>::Ok(()),
                };
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - withdraw_from_unregistered()
            DappsStakingFunc::WithdrawFromUnregistered => {
                let contract_bytes: [u8; 32] = env.read_as()?;
                let contract = Self::decode_smart_contract(contract_bytes)?;
                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::withdraw_from_unregistered();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::withdraw_from_unregistered(
                    RawOrigin::Signed(caller).into(),
                    contract,
                );
                let result_to_encode = match call_result {
                    Err(e) => Result::<(), DispatchError>::Err(DispatchError::from(e.error)),
                    _ => Result::<(), DispatchError>::Ok(()),
                };
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }

            // DappsStaking - nomination_transfer()
            DappsStakingFunc::NominationTransfer => {
                let origin_smart_contract_bytes: [u8; 32] = env.read_as()?;
                let amount: BalanceOf<T> = env.read_as()?;
                let target_smart_contract_bytes: [u8; 32] = env.read_as()?;
                let origin_smart_contract =
                    Self::decode_smart_contract(origin_smart_contract_bytes)?;
                let target_smart_contract =
                    Self::decode_smart_contract(target_smart_contract_bytes)?;
                let base_weight =
                    <T as pallet_dapps_staking::Config>::WeightInfo::nomination_transfer();
                env.charge_weight(base_weight)?;

                let caller = env.ext().caller().clone();
                let call_result = pallet_dapps_staking::Pallet::<T>::nomination_transfer(
                    RawOrigin::Signed(caller).into(),
                    origin_smart_contract,
                    amount,
                    target_smart_contract,
                );
                let result_to_encode = match call_result {
                    Err(e) => Result::<(), DispatchError>::Err(DispatchError::from(e.error)),
                    _ => Result::<(), DispatchError>::Ok(()),
                };
                env.write(&result_to_encode.encode(), false, None)
                    .map_err(|_| {
                        DispatchError::Other(
                            "[ChainExtension] DappsStakingExtension failed to write result",
                        )
                    })?;
            }
        }

        // env.write(&result_to_encode.encode(), false, None).map_err(|_| {
        //     DispatchError::Other("[ChainExtension] DappsStakingExtension failed to write result")
        // })?;
        // log::info!(
        //     "[ChainExtension] DappsStakingExtension result_to_encode={:?}",
        //     &result_to_encode
        // );

        // match result_to_encode {
        //     Ok() =>
        // }

        Ok(())
    }
}

impl<R> DappsStakingExtension<R> {
    // TODO: remove only temporarily - when all function are reworked
    /// Helper method to decode type SmartContract enum
    pub fn decode_smart_contract2(
        account: R::AccountId,
    ) -> Result<<R as pallet_dapps_staking::Config>::SmartContract, DispatchError>
    where
        R: pallet_dapps_staking::Config,
    {
        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this chain extension,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = SmartContract::<R::AccountId>::Wasm(account).encode();

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
        let contract_enum_encoded = SmartContract::<R::AccountId>::Wasm(account).encode();

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

    // Strip module error text
    pub fn map_module_error(error: DispatchError) -> &'static str {
        let error_text = match error {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };

        error_text.unwrap_or_default()
    }

    // pub fn store_result_in_env<E>(env: &Environment<E, BufInBufOut>, call_result: DispatchResultWithPostInfo) -> Result<(), DispatchError>
    // where
    //     E: Ext,
    //     <E::T as SysConfig>::AccountId:
    //     UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]> + From<[u8; 32]>,
    //     {

    //     sp_std::if_std! {println!(
    //         "[ChainExtension] DappsStakingExtension BondAndStake call_result {:?}",
    //         call_result
    //     );}

    //     // let env = env.buf_in_buf_out();
    //     let result_to_encode = match call_result {
    //         Err(e) => {
    //             let mapped_error = DSError::try_from(e.error)?;
    //             let res = Result::<(), DSError>::Err(mapped_error);
    //             env.write(&res.encode(), false, None).map_err(|_| {
    //                 DispatchError::Other(
    //                     "[ChainExtension] DappsStakingExtension failed to write result",
    //                 )
    //             })?;
    //             return Err(DispatchError::from(e.error));
    //         }
    //         _ => Result::<(), DispatchError>::Ok(()),
    //     };

    //     sp_std::if_std! {println!(
    //         "[ChainExtension] DappsStakingExtension BondAndStake result_to_encode {:?}",
    //         result_to_encode
    //     );}

    //     Ok(())
    // }
}
