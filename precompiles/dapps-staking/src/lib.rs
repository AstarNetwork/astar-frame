//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use codec::{Decode, Encode};
use fp_evm::{Context, ExitError, ExitSucceed, PrecompileFailure, PrecompileOutput};

use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::{Currency, Get},
};
use pallet_dapps_staking::RewardDestination;
use pallet_evm::{AddressMapping, Precompile};
use precompile_utils::{
    Address, Bytes, EvmData, EvmDataReader, EvmDataWriter, EvmResult, FunctionModifier, Gasometer,
    RuntimeHelper,
};
use sp_core::H160;
use sp_runtime::traits::{Saturating, Zero};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;
extern crate alloc;

type BalanceOf<Runtime> = <<Runtime as pallet_dapps_staking::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Smart contract enum. TODO move this to Astar primitives.
/// This is only used to encode SmartContract enum
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<A> {
    /// EVM smart contract instance.
    Evm(H160),
    /// Wasm smart contract instance. Not used in this precompile
    Wasm(A),
}

pub struct DappsStakingWrapper<R>(PhantomData<R>);

impl<R> DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    BalanceOf<R>: EvmData,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    R::Call: From<pallet_dapps_staking::Call<R>>,
    R::AccountId: From<[u8; 32]>,
{
    /// Fetch current era from CurrentEra storage map
    fn read_current_era(gasometer: &mut Gasometer) -> EvmResult<PrecompileOutput> {
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();

        let output = EvmDataWriter::new().write(current_era).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Fetch unbonding period
    fn read_unbonding_period(gasometer: &mut Gasometer) -> EvmResult<PrecompileOutput> {
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        let unbonding_period = R::UnbondingPeriod::get();

        let output = EvmDataWriter::new().write(unbonding_period).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Fetch reward from EraRewardsAndStakes storage map
    fn read_era_reward(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
    ) -> EvmResult<PrecompileOutput> {
        input.expect_arguments(gasometer, 1)?;

        // parse input parameters for pallet-dapps-staking call
        let era: u32 = input.read::<u32>(gasometer)?.into();

        // call pallet-dapps-staking
        let read_reward = pallet_dapps_staking::GeneralEraInfo::<R>::get(era);
        let reward = read_reward.map_or(Zero::zero(), |r| {
            r.rewards.stakers.saturating_add(r.rewards.dapps)
        });
        // compose output
        let output = EvmDataWriter::new().write(reward).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Fetch total staked amount from EraRewardsAndStakes storage map
    fn read_era_staked(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
    ) -> EvmResult<PrecompileOutput> {
        input.expect_arguments(gasometer, 1)?;

        // parse input parameters for pallet-dapps-staking call
        let era: u32 = input.read::<u32>(gasometer)?.into();

        // call pallet-dapps-staking
        let reward_and_stake = pallet_dapps_staking::GeneralEraInfo::<R>::get(era);
        // compose output
        let staked = reward_and_stake.map_or(Zero::zero(), |r| r.staked);
        let staked = TryInto::<u128>::try_into(staked).unwrap_or(0);
        let output = EvmDataWriter::new().write(staked).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Fetch Ledger storage map for an account
    fn read_staked_amount(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
    ) -> EvmResult<PrecompileOutput> {
        input.expect_arguments(gasometer, 1)?;

        // parse input parameters for pallet-dapps-staking call
        let staker_vec: Vec<u8> = input.read::<Bytes>(gasometer)?.into();
        let staker: R::AccountId = match staker_vec.len() {
            // public address of the ss58 account has 32 bytes
            32 => {
                let mut staker_bytes = [0_u8; 32];
                staker_bytes[..].clone_from_slice(&staker_vec[0..32]);

                staker_bytes.into()
            }
            // public address of the H160 account has 20 bytes
            20 => {
                let mut staker_bytes = [0_u8; 20];
                staker_bytes[..].clone_from_slice(&staker_vec[0..20]);

                R::AddressMapping::into_account_id(staker_bytes.into())
            }
            _ => {
                // Return `false` if account length is wrong
                return Err(PrecompileFailure::Error {
                    exit_status: ExitError::Other("Bad input account, Use H160 or 32 bytes".into()),
                });
            }
        };

        // call pallet-dapps-staking
        let ledger = pallet_dapps_staking::Ledger::<R>::get(&staker);
        log::trace!(target: "ds-precompile", "read_staked_amount for account:{:?}, ledger.locked:{:?}", staker, ledger.locked);

        // compose output
        let output = EvmDataWriter::new().write(ledger.locked).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Read the amount staked on contract in the given era
    fn read_contract_stake(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
    ) -> EvmResult<PrecompileOutput> {
        input.expect_arguments(gasometer, 1)?;

        // parse input parameters for pallet-dapps-staking call
        let contract_h160 = input.read::<Address>(gasometer)?.0;
        let contract_id = Self::decode_smart_contract(gasometer, contract_h160)?;
        let current_era = pallet_dapps_staking::CurrentEra::<R>::get();

        // call pallet-dapps-staking
        let staking_info =
            pallet_dapps_staking::Pallet::<R>::contract_stake_info(&contract_id, current_era)
                .unwrap_or_default();

        // encode output with total
        let total = TryInto::<u128>::try_into(staking_info.total).unwrap_or(0);
        let output = EvmDataWriter::new().write(total).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Register contract with the dapp-staking pallet
    fn register(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        input.expect_arguments(gasometer, 1)?;
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // parse contract's address
        let contract_h160 = input.read::<Address>(gasometer)?.0;
        let contract_id = Self::decode_smart_contract(gasometer, contract_h160)?;
        log::trace!(target: "ds-precompile", "register {:?}", contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call = pallet_dapps_staking::Call::<R>::register { contract_id }.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Lock up and stake balance of the origin account.
    fn bond_and_stake(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        input.expect_arguments(gasometer, 1)?;
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // parse contract's address
        let contract_h160 = input.read::<Address>(gasometer)?.0;
        let contract_id = Self::decode_smart_contract(gasometer, contract_h160)?;

        // parse balance to be staked
        let value: BalanceOf<R> = input.read(gasometer)?;

        log::trace!(target: "ds-precompile", "bond_and_stake {:?}, {:?}", contract_id, value);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call = pallet_dapps_staking::Call::<R>::bond_and_stake { contract_id, value }.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Start unbonding process and unstake balance from the contract.
    fn unbond_and_unstake(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        input.expect_arguments(gasometer, 1)?;
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // parse contract's address
        let contract_h160 = input.read::<Address>(gasometer)?.0;
        let contract_id = Self::decode_smart_contract(gasometer, contract_h160)?;

        // parse balance to be unstaked
        let value: BalanceOf<R> = input.read(gasometer)?;
        log::trace!(target: "ds-precompile", "unbond_and_unstake {:?}, {:?}", contract_id, value);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call =
            pallet_dapps_staking::Call::<R>::unbond_and_unstake { contract_id, value }.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Start unbonding process and unstake balance from the contract.
    fn withdraw_unbonded(
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call = pallet_dapps_staking::Call::<R>::withdraw_unbonded {}.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    fn claim_dapp(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        input.expect_arguments(gasometer, 1)?;
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // parse contract's address
        let contract_h160 = input.read::<Address>(gasometer)?.0;
        let contract_id = Self::decode_smart_contract(gasometer, contract_h160)?;

        // parse era
        let era: u32 = input.read::<u32>(gasometer)?.into();
        log::trace!(target: "ds-precompile", "claim_dapp {:?}, era {:?}", contract_id, era);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call = pallet_dapps_staking::Call::<R>::claim_dapp { contract_id, era }.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Claim rewards for the contract in the dapps-staking pallet
    fn claim_staker(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        input.expect_arguments(gasometer, 1)?;
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // parse contract's address
        let contract_h160 = input.read::<Address>(gasometer)?.0;
        let contract_id = Self::decode_smart_contract(gasometer, contract_h160)?;
        log::trace!(target: "ds-precompile", "claim_staker {:?}", contract_id);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call = pallet_dapps_staking::Call::<R>::claim_staker { contract_id }.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Set claim reward destination for the caller
    fn set_reward_destination(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<(
        <R::Call as Dispatchable>::Origin,
        pallet_dapps_staking::Call<R>,
    )> {
        input.expect_arguments(gasometer, 1)?;
        gasometer.record_cost(RuntimeHelper::<R>::db_read_gas_cost())?;

        // raw solidity representation of enum
        let reward_destination_raw = input.read::<u8>(gasometer)?;

        // Transform raw value into dapps staking enum
        let reward_destination = if reward_destination_raw == 0 {
            RewardDestination::FreeBalance
        } else if reward_destination_raw == 1 {
            RewardDestination::StakeBalance
        } else {
            return Err(precompile_utils::error(
                "Unexpected reward destination value.",
            ));
        };

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        log::trace!(target: "ds-precompile", "set_reward_destination {:?} {:?}", origin, reward_destination);

        let call =
            pallet_dapps_staking::Call::<R>::set_reward_destination { reward_destination }.into();

        // Return call information
        Ok((Some(origin).into(), call))
    }

    /// Helper method to decode type SmartContract enum
    pub fn decode_smart_contract(
        gasometer: &mut Gasometer,
        contract_h160: H160,
    ) -> EvmResult<<R as pallet_dapps_staking::Config>::SmartContract> {
        // Encode contract address to fit SmartContract enum.
        // Since the SmartContract enum type can't be accessed from this pecompile,
        // use locally defined enum clone (see Contract enum)
        let contract_enum_encoded = Contract::<H160>::Evm(contract_h160).encode();

        // encoded enum will add one byte before the contract's address
        // therefore we need to decode len(H160) + 1 byte = 21
        let smart_contract = <R as pallet_dapps_staking::Config>::SmartContract::decode(
            &mut &contract_enum_encoded[..21],
        )
        .map_err(|_| gasometer.revert("Error while decoding SmartContract"))?;

        Ok(smart_contract)
    }
}

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    ReadCurrentEra = "read_current_era()",
    ReadUnbondingPeriod = "read_unbonding_period()",
    ReadEraReward = "read_era_reward(uint32)",
    ReadEraStaked = "read_era_staked(uint32)",
    ReadStakedAmount = "read_staked_amount(bytes)",
    ReadContractStake = "read_contract_stake(address)",
    Register = "register(address)",
    BondAndStake = "bond_and_stake(address,uint128)",
    UnbondAndUnstake = "unbond_and_unstake(address,uint128)",
    WithdrawUnbounded = "withdraw_unbonded()",
    ClaimDapp = "claim_dapp(address,uint128)",
    ClaimStaker = "claim_staker(address)",
    SetRewardDestination = "set_reward_destination(RewardDestination)",
}

impl<R> Precompile for DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    BalanceOf<R>: EvmData,
    R::AccountId: From<[u8; 32]>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "ds-precompile", "Execute input = {:?}", input);
        let mut gasometer = Gasometer::new(target_gas);
        let gasometer = &mut gasometer;

        let (mut input, selector) = EvmDataReader::new_with_selector(gasometer, input)?;
        let input = &mut input;

        gasometer.check_function_modifier(
            context,
            is_static,
            match selector {
                Action::ReadCurrentEra
                | Action::ReadUnbondingPeriod
                | Action::ReadEraReward
                | Action::ReadEraStaked
                | Action::ReadStakedAmount
                | Action::ReadContractStake => FunctionModifier::View,
                _ => FunctionModifier::NonPayable,
            },
        )?;

        let (origin, call) = match selector {
            // read storage
            Action::ReadCurrentEra => return Self::read_current_era(gasometer),
            Action::ReadUnbondingPeriod => return Self::read_unbonding_period(gasometer),
            Action::ReadEraReward => return Self::read_era_reward(input, gasometer),
            Action::ReadEraStaked => return Self::read_era_staked(input, gasometer),
            Action::ReadStakedAmount => return Self::read_staked_amount(input, gasometer),
            Action::ReadContractStake => return Self::read_contract_stake(input, gasometer),
            // Dispatchables
            Action::Register => Self::register(input, gasometer, context)?,
            Action::BondAndStake => Self::bond_and_stake(input, gasometer, context)?,
            Action::UnbondAndUnstake => Self::unbond_and_unstake(input, gasometer, context)?,
            Action::WithdrawUnbounded => Self::withdraw_unbonded(gasometer, context)?,
            Action::ClaimDapp => Self::claim_dapp(input, gasometer, context)?,
            Action::ClaimStaker => Self::claim_staker(input, gasometer, context)?,
            Action::SetRewardDestination => {
                Self::set_reward_destination(input, gasometer, context)?
            }
        };

        // Dispatch call (if enough gas).
        RuntimeHelper::<R>::try_dispatch(origin, call, gasometer)?;

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: vec![],
            logs: vec![],
        })
    }
}
