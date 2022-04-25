use crate::mock::{
    advance_to_era, default_context, evm_call, initialize_first_block, precompile_address, Call,
    DappsStaking, EraIndex, ExternalityBuilder, Origin, TestAccount, AST, UNBONDING_PERIOD, *,
};
use codec::{Decode, Encode};
use fp_evm::{PrecompileFailure, PrecompileOutput};
use frame_support::{assert_ok, dispatch::Dispatchable};
use pallet_dapps_staking::RewardDestination;
use pallet_evm::{ExitSucceed, PrecompileSet};
use sha3::{Digest, Keccak256};
use sp_core::H160;
use sp_runtime::{AccountId32, Perbill};
use std::assert_matches::assert_matches;

const ARG_SIZE_BYTES: usize = 32;

fn precompiles() -> DappPrecompile<TestRuntime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_argument_count_reverts() {
    ExternalityBuilder::default().build().execute_with(|| {
        // This selector is only three bytes long when four are required.
        let short_selector = vec![1u8, 2u8, 3u8];

        assert_matches!(
            precompiles().execute(
                precompile_address(),
                &short_selector,
                None,
                &default_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert { output, ..}))
            if output == b"tried to parse selector out of bounds",
        );
    });
}

#[test]
fn no_selector_exists_but_length_is_right() {
    ExternalityBuilder::default().build().execute_with(|| {
        let bad_selector = vec![1u8, 2u8, 3u8, 4u8];

        assert_matches!(
            precompiles().execute(
                precompile_address(),
                &bad_selector,
                None,
                &default_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert { output, ..}))
            if &output == b"unknown selector"
        );
    });
}

#[test]
fn current_era_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let selector = &Keccak256::digest(b"read_current_era()")[0..4];
        let mut expected_era = vec![0u8; 32];
        expected_era[31] = 1;

        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_era.clone(),
            cost: Default::default(),
            logs: Default::default(),
        }));

        assert_eq!(
            precompiles().execute(
                precompile_address(),
                &selector,
                None,
                &default_context(),
                false
            ),
            expected
        );

        // advance to era 5 and check output
        expected_era[31] = 5;
        advance_to_era(5);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_era,
            cost: Default::default(),
            logs: Default::default(),
        }));
        assert_eq!(
            precompiles().execute(
                precompile_address(),
                &selector,
                None,
                &default_context(),
                false
            ),
            expected
        );
    });
}

#[test]
fn read_unbonding_period_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        let selector = &Keccak256::digest(b"read_unbonding_period()")[0..4];
        let mut expected_unbonding_period = vec![0u8; 32];
        expected_unbonding_period[31] = UNBONDING_PERIOD as u8;

        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_unbonding_period,
            cost: Default::default(),
            logs: Default::default(),
        }));
        assert_eq!(
            precompiles().execute(
                precompile_address(),
                &selector,
                None,
                &default_context(),
                false
            ),
            expected
        );
    });
}

#[test]
fn read_era_reward_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"read_era_reward(uint32)")[0..4];
        let mut input_data = Vec::<u8>::from([0u8; 36]);
        input_data[0..4].copy_from_slice(&selector);
        let era = [0u8; 32];
        input_data[4..36].copy_from_slice(&era);

        // build expected outcome
        let reward = joint_block_reward();
        let expected_output = argument_from_u128(reward);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_output,
            cost: Default::default(),
            logs: Default::default(),
        }));

        // execute and verify read_era_reward() query
        assert_eq!(
            precompiles().execute(
                precompile_address(),
                &input_data,
                None,
                &default_context(),
                false
            ),
            expected
        );
    });
}

#[test]
fn read_era_staked_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        initialize_first_block();

        // build input for the call
        let selector = &Keccak256::digest(b"read_era_staked(uint32)")[0..4];
        let mut input_data = Vec::<u8>::from([0u8; 36]);
        input_data[0..4].copy_from_slice(&selector);
        let era = [0u8; 32];
        input_data[4..36].copy_from_slice(&era);

        // build expected outcome
        let staked = 0;
        let expected_output = argument_from_u128(staked);
        let expected = Some(Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: expected_output,
            cost: Default::default(),
            logs: Default::default(),
        }));

        // execute and verify read_era_staked() query
        assert_eq!(
            precompiles().execute(
                precompile_address(),
                &input_data,
                None,
                &default_context(),
                false
            ),
            expected
        );
    });
}

#[test]
fn register_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![(TestAccount::Alex.into(), 200 * AST)])
        .build()
        .execute_with(|| {
            initialize_first_block();
            let developer = TestAccount::Alex.into();
            register_and_verify(developer, TEST_CONTRACT);
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

            // register new contract by Alex
            let developer = TestAccount::Alex.into();
            register_and_verify(developer, TEST_CONTRACT);

            let amount_staked_bobo = 100 * AST;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = 50 * AST;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            contract_era_stake_verify(TEST_CONTRACT, amount_staked_bobo + amount_staked_dino);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Bobo.into(), amount_staked_bobo);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);
        });
}

#[test]
fn unbond_and_unstake_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex.into(), 200 * AST),
            (TestAccount::Bobo.into(), 200 * AST),
            (TestAccount::Dino.into(), 100 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex.into();
            register_and_verify(developer, TEST_CONTRACT);

            let amount_staked_bobo = 100 * AST;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);
            let amount_staked_dino = 50 * AST;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            // Bobo unstakes all
            let era = 2;
            advance_to_era(era);
            unbond_unstake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            contract_era_stake_verify(TEST_CONTRACT, amount_staked_dino);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);

            // withdraw unbonded funds
            advance_to_era(era + UNBONDING_PERIOD + 1);
            withdraw_unbonded_verify(TestAccount::Bobo.into());
        });
}

#[test]
fn claim_dapp_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex.into(), 200 * AST),
            (TestAccount::Bobo.into(), 200 * AST),
            (TestAccount::Dino.into(), 200 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex;
            register_and_verify(developer.into(), TEST_CONTRACT);

            let stake_amount_total = 300 * AST;
            let ratio_bobo = Perbill::from_rational(3u32, 5u32);
            let ratio_dino = Perbill::from_rational(2u32, 5u32);
            let amount_staked_bobo = ratio_bobo * stake_amount_total;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = ratio_dino * stake_amount_total;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            // advance era and claim reward
            let era = 5;
            advance_to_era(era);
            claim_dapp_and_verify(TEST_CONTRACT, era - 1);

            //check that the reward is payed out to the developer
            let developer_reward = DAPP_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;
            assert_eq!(
                <TestRuntime as pallet_evm::Config>::Currency::free_balance(
                    &TestAccount::Alex.into()
                ),
                (200 * AST) + developer_reward - REGISTER_DEPOSIT
            );
        });
}

#[test]
fn claim_staker_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex.into(), 200 * AST),
            (TestAccount::Bobo.into(), 200 * AST),
            (TestAccount::Dino.into(), 200 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex;
            register_and_verify(developer.into(), TEST_CONTRACT);

            let stake_amount_total = 300 * AST;
            let ratio_bobo = Perbill::from_rational(3u32, 5u32);
            let ratio_dino = Perbill::from_rational(2u32, 5u32);
            let amount_staked_bobo = ratio_bobo * stake_amount_total;
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = ratio_dino * stake_amount_total;
            bond_stake_and_verify(TestAccount::Dino, TEST_CONTRACT, amount_staked_dino);

            // advance era and claim reward
            advance_to_era(5);

            let stakers_reward = STAKER_BLOCK_REWARD * BLOCKS_PER_ERA as Balance;

            // Ensure that all rewards can be claimed for the first staker
            for era in 1..DappsStaking::current_era() as Balance {
                claim_staker_and_verify(TestAccount::Bobo, TEST_CONTRACT);
                assert_eq!(
                    <TestRuntime as pallet_evm::Config>::Currency::free_balance(
                        &TestAccount::Bobo.into()
                    ),
                    (200 * AST) + ratio_bobo * stakers_reward * era
                );
            }

            // Repeat the same thing for the second staker
            for era in 1..DappsStaking::current_era() as Balance {
                claim_staker_and_verify(TestAccount::Dino, TEST_CONTRACT);
                assert_eq!(
                    <TestRuntime as pallet_evm::Config>::Currency::free_balance(
                        &TestAccount::Dino.into()
                    ),
                    (200 * AST) + ratio_dino * stakers_reward * era
                );
            }
        });
}

#[test]
fn bond_and_stake_ss58_is_ok() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex.into(), 200 * AST),
            (TestAccount::Bobo.into(), 200 * AST),
            (TestAccount::Dino.into(), 100 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register new contract by Alex
            let developer = TestAccount::Alex.into();
            register_and_verify(developer, TEST_CONTRACT);

            let amount_staked_bobo = 100 * AST;

            bond_stake_ss58_and_verify(TestAccount::Bobo.into(), TEST_CONTRACT, amount_staked_bobo);

            let amount_staked_dino = 50 * AST;
            bond_stake_ss58_and_verify(TestAccount::Dino.into(), TEST_CONTRACT, amount_staked_dino);

            contract_era_stake_verify(TEST_CONTRACT, amount_staked_bobo + amount_staked_dino);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Bobo.into(), amount_staked_bobo);
            verify_staked_amount(TEST_CONTRACT, TestAccount::Dino.into(), amount_staked_dino);
        });
}

#[test]
fn set_reward_destination() {
    ExternalityBuilder::default()
        .with_balances(vec![
            (TestAccount::Alex.into(), 200 * AST),
            (TestAccount::Bobo.into(), 200 * AST),
        ])
        .build()
        .execute_with(|| {
            initialize_first_block();

            // register contract and stake it
            register_and_verify(TestAccount::Alex.into(), TEST_CONTRACT);

            // bond & stake the origin contract
            bond_stake_and_verify(TestAccount::Bobo, TEST_CONTRACT, 100 * AST);

            // transfer nomination and ensure it was successful
            set_reward_destination_verify(TestAccount::Bobo.into(), RewardDestination::FreeBalance);
            set_reward_destination_verify(
                TestAccount::Bobo.into(),
                RewardDestination::StakeBalance,
            );
        });
}

// ****************************************************************************************************
// Helper functions
// ****************************************************************************************************

/// helper function to register and verify if registration is valid
fn register_and_verify(developer: AccountId32, contract_array: [u8; 20]) {
    let selector = &Keccak256::digest(b"register(address)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 36]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);

    // verify that argument check is done in register()
    assert_ok!(Call::Evm(evm_call(developer.clone(), selector.to_vec())).dispatch(Origin::root()));

    // call register()
    assert_ok!(Call::Evm(evm_call(developer.clone(), input_data)).dispatch(Origin::root()));

    // check the storage after the register
    let smart_contract_bytes =
        (DappsStaking::registered_contract(developer).unwrap_or_default()).encode();
    assert_eq!(
        smart_contract_bytes,
        to_smart_contract_bytes(contract_array)
    );

    // check_register_event(developer, contract_h160);
}

/// transform 20 byte array (h160) to smart contract encoded 21 bytes
pub fn to_smart_contract_bytes(input: [u8; 20]) -> [u8; 21] {
    let mut smart_contract_bytes = [0u8; 21];
    // prepend enum byte to the H160
    // enum for SmartContract::H160 is 0
    smart_contract_bytes[0] = 0;
    smart_contract_bytes[1..21].copy_from_slice(&input[0..20]);

    smart_contract_bytes
}

/// helper function to read ledger storage item
fn read_staked_amount_h160_verify(staker: TestAccount, amount: u128) {
    let selector = &Keccak256::digest(b"read_staked_amount(bytes)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 100]);
    input_data[0..4].copy_from_slice(&selector);

    input_data[35] = 32; // call data starting from position [4..36]
    input_data[67] = 20; // size of call data in bytes [36..68]

    let staker_arg = argument_from_h160(staker.to_h160());
    input_data[68..100].copy_from_slice(&staker_arg);

    let expected = Some(Ok(PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: argument_from_u128(amount),
        cost: Default::default(),
        logs: Default::default(),
    }));

    assert_eq!(
        precompiles().execute(
            precompile_address(),
            &input_data,
            None,
            &default_context(),
            false
        ),
        expected
    );
}

/// helper function to read ledger storage item for ss58 account
fn read_staked_amount_ss58_verify(staker: AccountId32, amount: u128) {
    let selector = &Keccak256::digest(b"read_staked_amount(bytes)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 100]);
    input_data[0..4].copy_from_slice(&selector);

    input_data[35] = 32; // call data starting from position [4..36]
    input_data[67] = 32; // size of call data in bytes [36..68]

    let staker_bytes = staker.encode();
    input_data[68..100].copy_from_slice(&staker_bytes);

    let expected = Some(Ok(PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: argument_from_u128(amount),
        cost: Default::default(),
        logs: Default::default(),
    }));

    assert_eq!(
        precompiles().execute(
            precompile_address(),
            &input_data,
            None,
            &default_context(),
            false
        ),
        expected
    );
}

/// helper function to bond, stake and verify if resulet is OK
fn bond_stake_and_verify(staker: TestAccount, contract_array: [u8; 20], amount: u128) {
    let selector = &Keccak256::digest(b"bond_and_stake(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let staking_amount = amount.to_be_bytes();
    input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

    // verify that argument check is done in bond_and_stake()
    assert_ok!(
        Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
    );

    // call bond_and_stake()
    assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));

    read_staked_amount_h160_verify(staker.clone(), amount.clone());
}

/// helper function to bond, stake and verify if resulet is OK
fn bond_stake_ss58_and_verify(staker: AccountId32, contract_array: [u8; 20], amount: u128) {
    let selector = &Keccak256::digest(b"bond_and_stake(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let staking_amount = amount.to_be_bytes();
    input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

    // verify that argument check is done in bond_and_stake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), selector.to_vec())).dispatch(Origin::root()));

    // call bond_and_stake()
    assert_ok!(Call::Evm(evm_call(staker.clone(), input_data)).dispatch(Origin::root()));

    read_staked_amount_ss58_verify(staker.clone(), amount.clone());
}

/// helper function to unbond, unstake and verify if resulet is OK
fn unbond_unstake_and_verify(staker: TestAccount, contract_array: [u8; 20], amount: u128) {
    let selector = &Keccak256::digest(b"unbond_and_unstake(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let staking_amount = amount.to_be_bytes();
    input_data[(68 - staking_amount.len())..68].copy_from_slice(&staking_amount);

    // verify that argument check is done in unbond_unstake()
    assert_ok!(
        Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
    );

    // call unbond_and_unstake()
    assert_ok!(
        Call::Evm(evm_call(staker.clone().into(), input_data.clone())).dispatch(Origin::root())
    );

    read_staked_amount_h160_verify(staker.clone(), amount.clone());
}

/// helper function to withdraw unstaked funds and verify if resulet is OK
fn withdraw_unbonded_verify(staker: AccountId32) {
    let selector = &Keccak256::digest(b"withdraw_unbonded()")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 4]);
    input_data[0..4].copy_from_slice(&selector);

    // call unbond_and_unstake(). Check usable_balance before and after the call
    assert_ne!(
        <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker),
        <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker)
    );
    assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));
    assert_eq!(
        <TestRuntime as pallet_evm::Config>::Currency::free_balance(&staker),
        <TestRuntime as pallet_evm::Config>::Currency::usable_balance(&staker)
    );
}

/// helper function to verify change of reward destination for a staker
fn set_reward_destination_verify(staker: AccountId32, reward_destination: RewardDestination) {
    let input_data = match reward_destination {
        RewardDestination::FreeBalance => Keccak256::digest(b"free_balance_reward_destination()"),
        RewardDestination::StakeBalance => Keccak256::digest(b"stake_balance_reward_destination()"),
    };
    let input_data = &input_data[0..4];

    // Read staker's ledger
    let init_ledger = DappsStaking::ledger(&staker);
    // Ensure that something is staked or being unbonded
    assert!(!init_ledger.is_empty());

    assert_ok!(
        Call::Evm(evm_call(staker.clone().into(), input_data.to_vec())).dispatch(Origin::root())
    );

    let final_ledger = DappsStaking::ledger(&staker);
    assert_eq!(final_ledger.reward_destination(), reward_destination);
}

/// helper function to bond, stake and verify if result is OK
fn claim_dapp_and_verify(contract_array: [u8; 20], era: EraIndex) {
    let staker = TestAccount::Bobo;
    let selector = &Keccak256::digest(b"claim_dapp(address,uint128)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 68]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);
    let era_array = era.to_be_bytes();
    input_data[(68 - era_array.len())..68].copy_from_slice(&era_array);

    // verify that argument check is done in claim()
    assert_ok!(
        Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
    );

    assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));
}

/// helper function to bond, stake and verify if the result is OK
fn claim_staker_and_verify(staker: TestAccount, contract_array: [u8; 20]) {
    let selector = &Keccak256::digest(b"claim_staker(address)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 36]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);

    assert_ok!(
        Call::Evm(evm_call(staker.clone().into(), selector.to_vec())).dispatch(Origin::root())
    );
    assert_ok!(Call::Evm(evm_call(staker.clone().into(), input_data)).dispatch(Origin::root()));
}

fn contract_era_stake_verify(contract_array: [u8; 20], amount: u128) {
    // prepare input to read staked amount on the contract
    let selector = &Keccak256::digest(b"read_contract_stake(address)")[0..4];
    let mut input_data = Vec::<u8>::from([0u8; 36]);
    input_data[0..4].copy_from_slice(&selector);
    input_data[16..36].copy_from_slice(&contract_array);

    // Compose expected outcome: add total stake on contract
    let expected_output = argument_from_u128(amount);
    let expected = Some(Ok(PrecompileOutput {
        exit_status: ExitSucceed::Returned,
        output: expected_output,
        cost: Default::default(),
        logs: Default::default(),
    }));

    // execute and verify read_contract_stake() query
    assert_eq!(
        precompiles().execute(
            precompile_address(),
            &input_data,
            None,
            &default_context(),
            false
        ),
        expected
    );
}

/// helper function to verify latest staked amount
fn verify_staked_amount(contract_array: [u8; 20], staker: AccountId32, amount: Balance) {
    // check the storage
    let smart_contract = decode_smart_contract_from_array(contract_array).unwrap();
    let staker_info = DappsStaking::staker_info(staker, &smart_contract);
    assert_eq!(staker_info.latest_staked_value(), amount);
}

/// Helper method to decode type SmartContract enum from [u8; 20]
fn decode_smart_contract_from_array(
    contract_array: [u8; 20],
) -> Result<<TestRuntime as pallet_dapps_staking::Config>::SmartContract, String> {
    // Encode contract address to fit SmartContract enum.
    let mut contract_enum_encoded: [u8; 21] = [0; 21];
    contract_enum_encoded[0] = 0; // enum for EVM H160 address is 0
    contract_enum_encoded[1..21].copy_from_slice(&contract_array);

    let smart_contract = <TestRuntime as pallet_dapps_staking::Config>::SmartContract::decode(
        &mut &contract_enum_encoded[..21],
    )
    .map_err(|_| "Error while decoding SmartContract")?;

    Ok(smart_contract)
}

/// Store u128 value in the 32 bytes vector as big endian
pub fn argument_from_u128(value: u128) -> Vec<u8> {
    let mut buffer = [0u8; ARG_SIZE_BYTES];
    buffer[ARG_SIZE_BYTES - core::mem::size_of::<u128>()..].copy_from_slice(&value.to_be_bytes());
    buffer.to_vec()
}

/// Store H160 value in the 32 bytes vector as big endian
pub fn argument_from_h160(value: H160) -> Vec<u8> {
    let mut buffer = [0u8; ARG_SIZE_BYTES];
    buffer[0..core::mem::size_of::<H160>()].copy_from_slice(&value.to_fixed_bytes());
    buffer.to_vec()
}
