use crate::{
    mock::{
        advance_to_era, default_context, evm_call, initialize_first_block, precompile_address,
        Call, DappsStaking, EraIndex, ExternalityBuilder, Origin, TestAccount, AST,
        UNBONDING_PERIOD, *,
    },
    *,
};
use codec::{Decode, Encode};
use fp_evm::{PrecompileFailure, PrecompileOutput};
use frame_support::{assert_ok, dispatch::Dispatchable};
use pallet_dapps_staking::RewardDestination;
use pallet_evm::{ExitSucceed, PrecompileSet};
use precompile_utils::testing::*;
use sha3::{Digest, Keccak256};
use sp_core::H160;
use sp_runtime::{traits::Zero, AccountId32, Perbill};
use std::assert_matches::assert_matches;

const ARG_SIZE_BYTES: usize = 32;

fn precompiles() -> DappPrecompile<TestRuntime> {
    PrecompilesValue::get()
}

#[test]
fn current_era_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let current_era = DappsStaking::current_era();

        precompiles()
            .prepare_test(
                TestAccount::Alex,
                precompile_address(),
                EvmDataWriter::new_with_selector(Action::ReadCurrentEra).build(),
            )
            .expect_cost(READ_WEIGHT)
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(current_era).build());

        // advance to era 5 and check output
        advance_to_era(5);
        let current_era = DappsStaking::current_era();

        precompiles()
            .prepare_test(
                TestAccount::Alex,
                precompile_address(),
                EvmDataWriter::new_with_selector(Action::ReadCurrentEra).build(),
            )
            .expect_cost(READ_WEIGHT)
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(current_era).build());
    });
}

#[test]
fn read_unbonding_period_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        precompiles()
            .prepare_test(
                TestAccount::Alex,
                precompile_address(),
                EvmDataWriter::new_with_selector(Action::ReadUnbondingPeriod).build(),
            )
            .expect_cost(READ_WEIGHT)
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(UNBONDING_PERIOD).build());
    });
}

#[test]
fn read_era_reward_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        advance_to_era(3);
        let era_reward = joint_block_reward() * BLOCKS_PER_ERA as u128;
        let second_era: EraIndex = 2;

        precompiles()
            .prepare_test(
                TestAccount::Alex,
                precompile_address(),
                EvmDataWriter::new_with_selector(Action::ReadEraReward)
                    .write(second_era)
                    .build(),
            )
            .expect_cost(READ_WEIGHT)
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(era_reward).build());
    });
}

#[test]
fn read_era_staked_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let zero_era = EraIndex::zero();
        let staked = Balance::zero();

        precompiles()
            .prepare_test(
                TestAccount::Alex,
                precompile_address(),
                EvmDataWriter::new_with_selector(Action::ReadEraStaked)
                    .write(zero_era)
                    .build(),
            )
            .expect_cost(READ_WEIGHT)
            .expect_no_logs()
            .execute_returns(EvmDataWriter::new().write(staked).build());
    });
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![(TestAccount::Alex.into(), 200 * AST)])
        .build()
        .execute_with(|| {
            initialize_first_block();

            register_and_verify(TestAccount::Alex, TEST_CONTRACT);
        });
}

#[test]
fn bond_and_stake_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex.into(), 200 * AST),
            (TestAccount::Bobo.into(), 200 * AST),
            (TestAccount::Dino.into(), 100 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            register_and_verify(TestAccount::Alex, TEST_CONTRACT);

            let amount_staked_bobo = 100 * AST;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = 50 * AST;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            contract_era_stake_verify(TEST_CONTRACT, amount_staked_bobo + amount_staked_dino);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Bobo.into(), amount_staked_bobo);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);
        });
}

// #[test]
// fn unbond_and_unstake_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 100 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex.into();
//             register_and_verify(developer, TEST_CONTRACT);

//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);
//             let amount_staked_dino = 50 * AST;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             // Bobo unstakes all
//             let era = 2;
//             advance_to_era(era);
//             unbond_unstake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             contract_era_stake_verify(TEST_CONTRACT, amount_staked_dino);
//             verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);

//             // withdraw unbonded funds
//             advance_to_era(era + UNBONDING_PERIOD + 1);
//             withdraw_unbonded_verify(TestAccount::Bobo.into());
//         });
// }

// #[test]
// fn claim_dapp_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex;
//             register_and_verify(developer.into(), TEST_CONTRACT);

//             let stake_amount_total = 300 * AST;
//             let ratio_bobo = Perbill::from_rational(3u32, 5u32);
//             let ratio_dino = Perbill::from_rational(2u32, 5u32);
//             let amount_staked_bobo = ratio_bobo * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let amount_staked_dino = ratio_dino * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             // advance era and claim reward
//             let era = 5;
//             advance_to_era(era);
//             claim_dapp_and_verify(TEST_CONTRACT, era - 1);

//             //check that the reward is payed out to the developer
//             let developer_reward = DAPP_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;
//             assert_eq!(
//                 <TestRuntime as pallet_evm::Config>::Currency::free_balance(
//                     &TestAccount::Alex.into()
//                 ),
//                 (200 * AST) + developer_reward - REGISTER_DEPOSIT
//             );
//         });
// }

// #[test]
// fn claim_staker_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex;
//             register_and_verify(developer.into(), TEST_CONTRACT);

//             let stake_amount_total = 300 * AST;
//             let ratio_bobo = Perbill::from_rational(3u32, 5u32);
//             let ratio_dino = Perbill::from_rational(2u32, 5u32);
//             let amount_staked_bobo = ratio_bobo * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let amount_staked_dino = ratio_dino * stake_amount_total;
//             bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

//             // advance era and claim reward
//             advance_to_era(5);

//             let stakers_reward = STAKER_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;

//             // Ensure that all rewards can be claimed for the first staker
//             for era in 1..DappsStaking::current_era() as Balance {
//                 claim_staker_and_verify(TestAccount::Bobo, TEST_CONTRACT);
//                 assert_eq!(
//                     <TestRuntime as pallet_evm::Config>::Currency::free_balance(
//                         &TestAccount::Bobo.into()
//                     ),
//                     (200 * AST) + ratio_bobo * stakers_reward * era
//                 );
//             }

//             // Repeat the same thing for the second staker
//             for era in 1..DappsStaking::current_era() as Balance {
//                 claim_staker_and_verify(TestAccount::Dino, TEST_CONTRACT);
//                 assert_eq!(
//                     <TestRuntime as pallet_evm::Config>::Currency::free_balance(
//                         &TestAccount::Dino.into()
//                     ),
//                     (200 * AST) + ratio_dino * stakers_reward * era
//                 );
//             }
//         });
// }

// #[test]
// fn bond_and_stake_ss58_is_ok() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//             (TestAccount::Dino.into(), 100 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex.into();
//             register_and_verify(developer, TEST_CONTRACT);

//             let amount_staked_bobo = 100 * AST;

//             bond_stake_ss58_and_verify(TestAccount::Bobo.into(), TEST_CONTRACT, amount_staked_bobo);

//             let amount_staked_dino = 50 * AST;
//             bond_stake_ss58_and_verify(TestAccount::Dino.into(), TEST_CONTRACT, amount_staked_dino);

//             contract_era_stake_verify(TEST_CONTRACT, amount_staked_bobo + amount_staked_dino);
//             verify_staked_amount(TEST_CONTRACT, TestAccount::Bobo.into(), amount_staked_bobo);
//             verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);
//         });
// }

// #[test]
// fn set_reward_destination() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();
//             // register contract and stake it
//             register_and_verify(TestAccount::Alex.into(), TEST_CONTRACT);

//             // bond & stake the origin contract
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, 100 * AST);

//             // change destinations and verfiy it was successful
//             set_reward_destination_verify(TestAccount::Bobo.into(), RewardDestination::FreeBalance);
//             set_reward_destination_verify(
//                 TestAccount::Bobo.into(),
//                 RewardDestination::StakeBalance,
//             );
//             set_reward_destination_verify(TestAccount::Bobo.into(), RewardDestination::FreeBalance);
//         });
// }

// #[test]
// fn withdraw_from_unregistered() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register new contract by Alex
//             let developer = TestAccount::Alex.into();
//             register_and_verify(developer, TEST_CONTRACT);

//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             let contract_id = decode_smart_contract_from_array(TEST_CONTRACT).unwrap();
//             assert_ok!(DappsStaking::unregister(Origin::root(), contract_id));

//             withdraw_from_unregistered_verify(TestAccount::Bobo.into(), TEST_CONTRACT);
//         });
// }

// #[test]
// fn nomination_transfer() {
//     ExternalityBuilder::default()
//         .with_balances(vec![
//             (TestAccount::Alex.into(), 200 * AST),
//             (TestAccount::Dino.into(), 200 * AST),
//             (TestAccount::Bobo.into(), 200 * AST),
//         ])
//         .build()
//         .execute_with(|| {
//             initialize_first_block();

//             // register two contracts for nomination transfer test
//             let origin_contract: [u8; 20] = H160::repeat_byte(0x09).to_fixed_bytes();
//             let target_contract: [u8; 20] = H160::repeat_byte(0x0A).to_fixed_bytes();
//             register_and_verify(TestAccount::Alex.into(), origin_contract);
//             register_and_verify(TestAccount::Dino.into(), target_contract);

//             // bond & stake the origin contract
//             let amount_staked_bobo = 100 * AST;
//             bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

//             // transfer nomination and ensure it was successful
//             nomination_transfer_verify(
//                 TestAccount::Bobo.into(),
//                 origin_contract,
//                 10 * AST,
//                 target_contract,
//             );
//         });
// }

// ****************************************************************************************************
// Helper functions
// ****************************************************************************************************

/// helper function to register and verify if registration is valid
fn register_and_verify(developer: TestAccount, contract: H160) {
    precompiles()
        .prepare_test(
            developer.clone(),
            precompile_address(),
            EvmDataWriter::new_with_selector(Action::Register)
                .write(Address(contract.clone()))
                .build(),
        )
        .expect_no_logs()
        .execute_returns(EvmDataWriter::new().write(true).build());

    // check the storage after the register
    let dev_account_id: AccountId32 = developer.into();
    let smart_contract_bytes =
        (DappsStaking::registered_contract(dev_account_id).unwrap_or_default()).encode();

    assert_eq!(
        // 0-th byte is enum value discriminator
        smart_contract_bytes[1..21],
        contract.to_fixed_bytes()
    );
}

/// helper function to read ledger storage item
fn read_staked_amount_h160_verify(staker: TestAccount, amount: u128) {
    precompiles()
        .prepare_test(
            staker.clone(),
            precompile_address(),
            EvmDataWriter::new_with_selector(Action::ReadStakedAmount)
                .write(AccountId32::from(staker))
                .build(),
        )
        .expect_no_logs()
        .execute_returns(EvmDataWriter::new().write(amount).build());
}

// /// helper function to read ledger storage item for ss58 account
// fn read_staked_amount_ss58_verify(staker: AccountId32, amount: u128) {
//     let selector = &Keccak256::digest(b"read_staked_amount(bytes)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 100]);
//     input_data[0..4].copy_from_slice(&selector);

//     input_data[35] = 32; // call data starting from position [4..36]
//     input_data[67] = 32; // size of call data in bytes [36..68]

//     let staker_bytes = staker.encode();
//     input_data[68..100].copy_from_slice(&staker_bytes);

//     let expected = Some(Ok(PrecompileOutput {
//         exit_status: ExitSucceed::Returned,
//         output: argument_from_u128(amount),
//     }));

//     assert_eq!(
//         precompiles().execute(
//             precompile_address(),
//             &input_data,
//             None,
//             &default_context(),
//             false
//         ),
//         expected
//     );
// }

/// helper function to bond, stake and verify if resulet is OK
fn bond_stake_and_verify(staker: TestAccount, contract: H160, amount: u128) {
    precompiles()
        .prepare_test(
            staker.clone(),
            precompile_address(),
            EvmDataWriter::new_with_selector(Action::BondAndStake)
                .write(Address(contract.clone()))
                .write(amount)
                .build(),
        )
        .expect_no_logs()
        .execute_returns(EvmDataWriter::new().write(true).build());

    read_staked_amount_h160_verify(staker, amount);
}

// /// helper function to bond, stake and verify if resulet is OK
// fn bond_stake_ss58_and_verify(staker: AccountId32, contract_array: [u8; 20], amount: u128) {
//     let selector = &Keccak256::digest(b"bond_and_stake(address,uint128)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 68]);
//     input_data[0..4].copy_from_slice(&selector);
//     input_data[16..36].copy_from_slice(&contract_array);
//     let staking_amount = amount.to_be_bytes();
//     input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

//     // verify that argument check is done in bond_and_stake()
//     assert_ok!(Call::Evm(evm_call(staker.clone(), selector.to_vec())).dispatch(Origin::root()));

//     // call bond_and_stake()
//     assert_ok!(Call::Evm(evm_call(staker.clone(), input_data)).dispatch(Origin::root()));

//     read_staked_amount_ss58_verify(staker.clone(), amount.clone());
// }

// /// helper function to unbond, unstake and verify if resulet is OK
// fn unbond_unstake_and_verify(staker: TestAccount, contract_array: [u8; 20], amount: u128) {
//     let selector = &Keccak256::digest(b"unbond_and_unstake(address,uint128)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 68]);
//     input_data[0..4].copy_from_slice(&selector);
//     input_data[16..36].copy_from_slice(&contract_array);
//     let staking_amount = amount.to_be_bytes();
//     input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

//     // verify that argument check is done in unbond_unstake()
//     assert_ok!(
//         Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
//     );

//     // call unbond_and_unstake()
//     assert_ok!(
//         Call::Evm(evm_call(staker.clone().into(), input_data.clone())).dispatch(Origin::root())
//     );

//     read_staked_amount_h160_verify(staker.clone(), amount.clone());
// }

// /// helper function to withdraw unstaked funds and verify if resulet is OK
// fn withdraw_unbonded_verify(staker: AccountId32) {
//     let selector = &Keccak256::digest(b"withdraw_unbonded()")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 4]);
//     input_data[0..4].copy_from_slice(&selector);

//     // call unbond_and_unstake(). Check usable_balance before and after the call
//     assert_ne!(
//         <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker),
//         <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker)
//     );
//     assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));
//     assert_eq!(
//         <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker),
//         <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker)
//     );
// }

// /// helper function to verify change of reward destination for a staker
// fn set_reward_destination_verify(staker: AccountId32, reward_destination: RewardDestination) {
//     let selector = &Keccak256::digest(b"set_reward_destination(uint8)")[0..4];

//     let mut input_data = Vec::<u8>::from([0u8; 36]);
//     input_data[0..4].copy_from_slice(&selector);

//     let reward_destination_raw: u8 = match reward_destination {
//         RewardDestination::FreeBalance => 0,
//         RewardDestination::StakeBalance => 1,
//     };
//     input_data[35] = reward_destination_raw;

//     // Read staker's ledger
//     let init_ledger = DappsStaking::ledger(&staker);
//     // Ensure that something is staked or being unbonded
//     assert!(!init_ledger.is_empty());

//     assert_ok!(
//         Call::Evm(evm_call(staker.clone().into(), input_data.to_vec())).dispatch(Origin::root())
//     );

//     let final_ledger = DappsStaking::ledger(&staker);
//     assert_eq!(final_ledger.reward_destination(), reward_destination);
// }

// /// helper function to withdraw funds from unregistered contract
// fn withdraw_from_unregistered_verify(staker: AccountId32, contract_array: [u8; 20]) {
//     let selector = &Keccak256::digest(b"withdraw_from_unregistered(address)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 36]);

//     input_data[0..4].copy_from_slice(&selector);
//     input_data[16..36].copy_from_slice(&contract_array);

//     let smart_contract = decode_smart_contract_from_array(contract_array).unwrap();
//     let init_staker_info = DappsStaking::staker_info(&staker, &smart_contract);
//     assert!(!init_staker_info.latest_staked_value().is_zero());

//     // call withdraw_from_unregistered(). Check usable_balance before and after the call
//     assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));

//     let final_staker_info = DappsStaking::staker_info(&staker, &smart_contract);
//     assert!(final_staker_info.latest_staked_value().is_zero());
// }

// /// helper function to verify nomination transfer from origin to target contract
// fn nomination_transfer_verify(
//     staker: AccountId32,
//     origin_contract_array: [u8; 20],
//     amount: Balance,
//     target_contract_array: [u8; 20],
// ) {
//     let selector = &Keccak256::digest(b"nomination_transfer(address,uint128,address)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 100]);

//     input_data[0..4].copy_from_slice(&selector);
//     input_data[16..36].copy_from_slice(&origin_contract_array);
//     input_data[52..68].copy_from_slice(&amount.to_be_bytes());
//     input_data[80..100].copy_from_slice(&target_contract_array);

//     let origin_smart_contract = decode_smart_contract_from_array(origin_contract_array).unwrap();
//     let target_smart_contract = decode_smart_contract_from_array(target_contract_array).unwrap();

//     // Read init data staker info states
//     let init_origin_staker_info = DappsStaking::staker_info(&staker, &origin_smart_contract);
//     let init_target_staker_info = DappsStaking::staker_info(&staker, &target_smart_contract);

//     assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));

//     let final_origin_staker_info = DappsStaking::staker_info(&staker, &origin_smart_contract);
//     let final_target_staker_info = DappsStaking::staker_info(&staker, &target_smart_contract);

//     // Verify final state
//     let will_be_unstaked = init_origin_staker_info
//         .latest_staked_value()
//         .saturating_sub(amount)
//         < MINIMUM_STAKING_AMOUNT;
//     let transfer_amount = if will_be_unstaked {
//         init_origin_staker_info.latest_staked_value()
//     } else {
//         amount
//     };

//     assert_eq!(
//         final_origin_staker_info.latest_staked_value() + transfer_amount,
//         init_origin_staker_info.latest_staked_value()
//     );
//     assert_eq!(
//         final_target_staker_info.latest_staked_value() - transfer_amount,
//         init_target_staker_info.latest_staked_value()
//     );
// }

// /// helper function to bond, stake and verify if result is OK
// fn claim_dapp_and_verify(contract_array: [u8; 20], era: EraIndex) {
//     let staker = TestAccount::Bobo;
//     let selector = &Keccak256::digest(b"claim_dapp(address,uint128)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 68]);
//     input_data[0..4].copy_from_slice(&selector);
//     input_data[16..36].copy_from_slice(&contract_array);
//     let era_array = era.to_be_bytes();
//     input_data[(68 - era_array.len())..68].copy_from_slice(&era_array);

//     // verify that argument check is done in claim()
//     assert_ok!(
//         Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
//     );

//     assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));
// }

// /// helper function to bond, stake and verify if the result is OK
// fn claim_staker_and_verify(staker: TestAccount, contract_array: [u8; 20]) {
//     let selector = &Keccak256::digest(b"claim_staker(address)")[0..4];
//     let mut input_data = Vec::<u8>::from([0u8; 36]);
//     input_data[0..4].copy_from_slice(&selector);
//     input_data[16..36].copy_from_slice(&contract_array);

//     assert_ok!(
//         Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
//     );
//     assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));
// }

fn contract_era_stake_verify(contract: H160, amount: Balance) {
    precompiles()
        .prepare_test(
            TestAccount::Alex,
            precompile_address(),
            EvmDataWriter::new_with_selector(Action::ReadContractStake)
                .write(Address(contract.clone()))
                .build(),
        )
        .expect_cost(READ_WEIGHT)
        .expect_no_logs()
        .execute_returns(EvmDataWriter::new().write(amount).build());
}

/// helper function to verify latest staked amount
fn verify_staked_amount(contract: H160, staker: TestAccount, amount: Balance) {
    precompiles()
        .prepare_test(
            staker.clone(),
            precompile_address(),
            EvmDataWriter::new_with_selector(Action::ReadStakedAmountOnContract)
                .write(Address(contract.clone()))
                .write(Address(H160::from(staker)))
                .build(),
        )
        .expect_cost(READ_WEIGHT)
        .expect_no_logs()
        .execute_returns(EvmDataWriter::new().write(amount).build());
}

// /// Store u128 value in the 32 bytes vector as big endian
// pub fn argument_from_u128(value: u128) -> Vec<u8> {
//     let mut buffer = [0u8; ARG_SIZE_BYTES];
//     buffer[ARG_SIZE_BYTES - core::mem::size_of::<u128>()..].copy_from_slice(&value.to_be_bytes());
//     buffer.to_vec()
// }

// /// Store H160 value in the 32 bytes vector as big endian
// pub fn argument_from_h160(value: H160) -> Vec<u8> {
//     let mut buffer = [0u8; ARG_SIZE_BYTES];
//     buffer[0..core::mem::size_of::<H160>()].copy_from_slice(&value.to_fixed_bytes());
//     buffer.to_vec()
// }

// /// Helper method to decode type SmartContract enum from [u8; 20]
// fn decode_smart_contract_from_array(
//     contract_array: [u8; 20],
// ) -> Result<<TestRuntime as pallet_dapps_staking::Config>::SmartContract, String> {
//     // Encode contract address to fit SmartContract enum.
//     let mut contract_enum_encoded: [u8; 21] = [0; 21];
//     contract_enum_encoded[0] = 0; // enum for EVM H160 address is 0
//     contract_enum_encoded[1..21].copy_from_slice(&contract_array);

//     let smart_contract = <TestRuntime as pallet_dapps_staking::Config>::SmartContract::decode(
//         &mut &contract_enum_encoded[..21],
//     )
//     .map_err(|_| "Error while decoding SmartContract")?;

//     Ok(smart_contract)
// }
