//! Astar dApps staking interface.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use codec::{Decode, Encode};
use fp_evm::{Context, ExitSucceed, PrecompileOutput};

use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    traits::{Currency, Get},
};
use pallet_evm::{AddressMapping, Precompile};
use sp_core::H160;
use sp_runtime::{
    traits::{SaturatedConversion, Zero},
    ModuleError,
};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};
use precompile_utils::{ Address,
    EvmData, EvmDataReader, EvmDataWriter, EvmResult, FunctionModifier, Gasometer,
    RuntimeHelper,
};
use sp_core::{H160, U256};
use sp_runtime::traits::Zero;
use sp_std::{convert::TryInto, marker::PhantomData, vec};
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
    R::AccountId: From<u64>,
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
        let read_reward = pallet_dapps_staking::EraRewardsAndStakes::<R>::get(era);

        // compose output
        let reward = read_reward.map_or(Zero::zero(), |r| r.rewards);
        let reward = TryInto::<u128>::try_into(reward).unwrap_or(0);
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
        let reward_and_stake = pallet_dapps_staking::EraRewardsAndStakes::<R>::get(era);

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

    /// Fetch Ledger storage map
    fn read_staked_amount(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
    ) -> EvmResult<PrecompileOutput> {
        input.expect_arguments(gasometer, 1)?;

        // parse input parameters for pallet-dapps-staking call
        let staker_h160 = input.read::<Address>(gasometer)?.0;
        let staker = R::AddressMapping::into_account_id(staker_h160);

        // call pallet-dapps-staking
        let ledger = pallet_dapps_staking::Ledger::<R>::get(&staker);

        // compose output
        let output = EvmDataWriter::new().write(ledger.locked).build();

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output,
            logs: Default::default(),
        })
    }

    /// Fetch Ledger storage map for ss58 account
    fn read_staked_amount_ss58(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
    ) -> EvmResult<PrecompileOutput> {
        input.expect_arguments(gasometer, 1)?;

        // parse input parameters for pallet-dapps-staking call
        let staker_u256 = input.read::<U256>(gasometer)?;
        let mut staker_bytes = [0_u8; 32];
        sp_core::U256::from(staker_u256).to_big_endian(&mut staker_bytes[..]);
        println!("staker_bytes {:?}", staker_bytes);

        let staker = <R as frame_system::Config>::AccountId::decode(&mut &staker_bytes[..])
            .map_err(|_| gasometer.revert("Error while decoding AccountID"))?;
        println!("staker {:?}", staker);

        // call pallet-dapps-staking
        let ledger = pallet_dapps_staking::Ledger::<R>::get(&staker);

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
            pallet_dapps_staking::Pallet::<R>::staking_info(&contract_id, current_era);

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

    /// Claim rewards for the contract in the dapp-staking pallet
    fn claim(
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
        log::trace!(target: "ds-precompile", "claim {:?}, era {:?}", contract_id, era);

        // Build call with origin.
        let origin = R::AddressMapping::into_account_id(context.caller);
        let call = pallet_dapps_staking::Call::<R>::claim { contract_id, era }.into();

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
    ReadStakedAmount = "read_staked_amount(address)",
    ReadStakedAmountSs58 = "read_staked_amount_ss58(bytes)",
    ReadContractStake = "read_contract_stake(address)",
    Register = "register(address)",
    BondAndStake = "bond_and_stake(address,uint128)",
    UnbondAndUnstake = "unbond_and_unstake(address,uint128)",
    WithdrawUnbounded = "withdraw_unbonded()",
    Claim = "claim(address,uint128)",
}

impl<R> Precompile for DappsStakingWrapper<R>
where
    R: pallet_evm::Config + pallet_dapps_staking::Config,
    R::Call: From<pallet_dapps_staking::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    <R::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    BalanceOf<R>: EvmData,
    <R as frame_system::Config>::AccountId: From<u64>,
{
    fn execute(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "ds-precompile", "In ds precompile");
        let mut gasometer = Gasometer::new(target_gas);
        let gasometer = &mut gasometer;

        let (mut input, selector) = EvmDataReader::new_with_selector(gasometer, input)?;
        let input = &mut input;

        gasometer.check_function_modifier(context, is_static, FunctionModifier::NonPayable)?;

        let (origin, call) = match selector {
            // read storage
            Action::ReadCurrentEra => return Self::read_current_era(gasometer),
            Action::ReadUnbondingPeriod => return Self::read_unbonding_period(gasometer),
            Action::ReadEraReward => return Self::read_era_reward(input, gasometer),
            Action::ReadEraStaked => return Self::read_era_staked(input, gasometer),
            Action::ReadStakedAmount => return Self::read_staked_amount(input, gasometer),
            Action::ReadStakedAmountSs58 => return Self::read_staked_amount_ss58(input, gasometer),
            Action::ReadContractStake => return Self::read_contract_stake(input, gasometer),
            // Dispatchables
            Action::Register => Self::register(input, gasometer, context)?,
            Action::BondAndStake => Self::bond_and_stake(input, gasometer, context)?,
            Action::UnbondAndUnstake => Self::unbond_and_unstake(input, gasometer, context)?,
            Action::WithdrawUnbounded => Self::withdraw_unbonded(gasometer, context)?,
            Action::Claim => Self::claim(input, gasometer, context)?,
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
