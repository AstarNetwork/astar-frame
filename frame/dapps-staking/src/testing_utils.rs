use super::{Event, *};
use frame_support::assert_ok;
use mock::{EraIndex, *};
use sp_runtime::{traits::AccountIdConversion, Perbill};

/// Helper struct used to store information relevant to era/contract/staker combination.
pub(crate) struct MemorySnapshot {
    era_info: EraInfo<Balance>,
    dapp_info: DAppInfo<AccountId>,
    staker_info: StakerInfo<Balance>,
    contract_info: EraStakingPoints<Balance>,
    free_balance: Balance,
    ledger: AccountLedger<Balance>,
}

impl MemorySnapshot {
    /// Prepares a new `MemorySnapshot` struct based on the given arguments.
    pub(crate) fn all(
        era: EraIndex,
        contract_id: &MockSmartContract<AccountId>,
        account: AccountId,
    ) -> Self {
        Self {
            era_info: DappsStaking::general_era_info(era).unwrap(),
            dapp_info: RegisteredDapps::<TestRuntime>::get(contract_id).unwrap(),
            staker_info: StakersInfo::<TestRuntime>::get(&account, contract_id),
            contract_info: DappsStaking::staking_info(contract_id, era),
            ledger: DappsStaking::ledger(&account),
            free_balance: <TestRuntime as Config>::Currency::free_balance(&account),
        }
    }

    /// Prepares a new `MemorySnapshot` struct but only with contract-related info
    /// (no info specific for individual staker).
    pub(crate) fn contract(era: EraIndex, contract_id: &MockSmartContract<AccountId>) -> Self {
        Self {
            era_info: DappsStaking::general_era_info(era).unwrap(),
            dapp_info: RegisteredDapps::<TestRuntime>::get(contract_id).unwrap(),
            staker_info: Default::default(),
            contract_info: DappsStaking::staking_info(contract_id, era),
            ledger: Default::default(),
            free_balance: Default::default(),
        }
    }
}

/// Used to fetch the free balance of dapps staking account
pub(crate) fn free_balance_of_dapps_staking_account() -> Balance {
    <TestRuntime as Config>::Currency::free_balance(
        &<TestRuntime as Config>::PalletId::get().into_account(),
    )
}

/// Used to get total dapps reward for an era.
pub(crate) fn get_total_reward_per_era() -> Balance {
    BLOCK_REWARD * BLOCKS_PER_ERA as Balance
}

/// Used to register contract for staking and assert success.
pub(crate) fn assert_register(developer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let init_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);

    // Contract shouldn't exist.
    assert!(!RegisteredDapps::<TestRuntime>::contains_key(contract_id));
    assert!(!RegisteredDevelopers::<TestRuntime>::contains_key(
        developer
    ));

    // Verify op is successfull
    assert_ok!(DappsStaking::enable_developer_pre_approval(
        Origin::root(),
        false
    ));
    assert_ok!(DappsStaking::register(
        Origin::signed(developer),
        contract_id.clone()
    ));

    let dapp_info = RegisteredDapps::<TestRuntime>::get(contract_id).unwrap();
    assert_eq!(dapp_info.state, DAppState::Registered);
    assert_eq!(dapp_info.developer, developer);
    assert_eq!(
        *contract_id,
        RegisteredDevelopers::<TestRuntime>::get(developer).unwrap()
    );

    let final_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);
    assert_eq!(
        final_reserved_balance,
        init_reserved_balance + <TestRuntime as Config>::RegisterDeposit::get()
    );
}

/// Perform `unregister` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_unregister(developer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::contract(current_era, contract_id);
    let init_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);

    // dApp should be registered prior to unregistering it
    assert_eq!(init_state.dapp_info.state, DAppState::Registered);

    // Ensure that contract can be unregistered
    assert_ok!(DappsStaking::unregister(
        Origin::root(),
        contract_id.clone()
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::ContractRemoved(
        developer,
        contract_id.clone(),
    )));

    let final_state = MemorySnapshot::contract(current_era, contract_id);
    let final_reserved_balance = <TestRuntime as Config>::Currency::reserved_balance(&developer);
    assert_eq!(
        final_reserved_balance,
        init_reserved_balance - <TestRuntime as Config>::RegisterDeposit::get()
    );

    assert_eq!(final_state.era_info.staked, init_state.era_info.staked);

    assert_eq!(
        final_state.contract_info.total,
        init_state.contract_info.total
    );
    assert_eq!(
        final_state.contract_info.number_of_stakers,
        init_state.contract_info.number_of_stakers
    );

    assert_eq!(
        final_state.dapp_info.state,
        DAppState::Unregistered(current_era)
    );
    assert_eq!(final_state.dapp_info.developer, developer);
}

/// Perform `withdraw_from_unregistered` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_withdraw_from_unregistered(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, contract_id, staker);

    // Initial checks
    let unregistered_era = if let DAppState::Unregistered(era) = init_state.dapp_info.state {
        assert!(era <= DappsStaking::current_era());
        era
    } else {
        panic!("Contract should be unregistered.")
    };

    let staked_value = init_state.staker_info.latest_staked_value();
    assert!(staked_value > 0);

    // Op with verification
    assert_ok!(DappsStaking::withdraw_from_unregistered(
        Origin::signed(staker.clone()),
        contract_id.clone()
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::WithdrawFromUnregistered(
        staker,
        contract_id.clone(),
        staked_value,
    )));

    let final_state = MemorySnapshot::all(current_era, contract_id, staker);

    // Verify that all final states are as expected
    assert_eq!(
        init_state.era_info.staked,
        final_state.era_info.staked + staked_value
    );
    assert_eq!(
        init_state.era_info.locked,
        final_state.era_info.locked + staked_value
    );
    assert_eq!(init_state.dapp_info, final_state.dapp_info);
    assert_eq!(
        init_state.ledger.locked,
        final_state.ledger.locked + staked_value
    );
    assert_eq!(
        init_state.ledger.unbonding_info,
        final_state.ledger.unbonding_info
    );
    assert!(final_state.staker_info.latest_staked_value().is_zero());

    if init_state.staker_info.clone().claim().0 >= unregistered_era {
        assert!(!StakersInfo::<TestRuntime>::contains_key(
            &staker,
            contract_id
        ));
    }
}

/// Perform `bond_and_stake` with all the accompanied checks including before/after storage comparison.
pub(crate) fn assert_bond_and_stake(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // Calculate the expected value that will be staked.
    let available_for_staking = init_state.free_balance
        - init_state.ledger.locked
        - <TestRuntime as Config>::MinimumRemainingAmount::get();
    let staking_value = available_for_staking.min(value);

    // Perform op and verify everything is as expected
    assert_ok!(DappsStaking::bond_and_stake(
        Origin::signed(staker),
        contract_id.clone(),
        value,
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::BondAndStake(
        staker,
        contract_id.clone(),
        staking_value,
    )));

    let final_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // In case staker hasn't been staking this contract until now
    if init_state.staker_info.latest_staked_value() == 0 {
        assert!(StakersInfo::<TestRuntime>::contains_key(
            &staker,
            contract_id
        ));
        assert_eq!(
            final_state.contract_info.number_of_stakers,
            init_state.contract_info.number_of_stakers + 1
        );
    }

    // Verify the remaining states
    assert_eq!(
        final_state.era_info.staked,
        init_state.era_info.staked + staking_value
    );
    assert_eq!(
        final_state.era_info.locked,
        init_state.era_info.locked + staking_value
    );
    assert_eq!(
        final_state.contract_info.total,
        init_state.contract_info.total + staking_value
    );
    assert_eq!(
        final_state.staker_info.latest_staked_value(),
        init_state.staker_info.latest_staked_value() + staking_value
    );
    assert_eq!(
        final_state.ledger.locked,
        init_state.ledger.locked + staking_value
    );
}

/// Used to perform start_unbonding with sucess and storage assertions.
pub(crate) fn assert_unbond_and_unstake(
    staker: AccountId,
    contract_id: &MockSmartContract<AccountId>,
    value: Balance,
) {
    // Get latest staking info
    let current_era = DappsStaking::current_era();
    let init_state = MemorySnapshot::all(current_era, &contract_id, staker);

    // Calculate the expected resulting unbonding amount
    let remaining_staked = init_state
        .staker_info
        .latest_staked_value()
        .saturating_sub(value);
    let expected_unbond_amount = if remaining_staked < MINIMUM_STAKING_AMOUNT {
        init_state.staker_info.latest_staked_value()
    } else {
        value
    };
    let remaining_staked = init_state.staker_info.latest_staked_value() - expected_unbond_amount;

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::unbond_and_unstake(
        Origin::signed(staker),
        contract_id.clone(),
        value
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::UnbondAndUnstake(
        staker,
        contract_id.clone(),
        expected_unbond_amount,
    )));

    // Fetch the latest unbonding info so we can compare it to initial unbonding info
    let final_state = MemorySnapshot::all(current_era, &contract_id, staker);
    let expected_unlock_era = current_era + UNBONDING_PERIOD;
    match init_state
        .ledger
        .unbonding_info
        .vec()
        .binary_search_by(|x| x.unlock_era.cmp(&expected_unlock_era))
    {
        Ok(_) => assert_eq!(
            init_state.ledger.unbonding_info.len(),
            final_state.ledger.unbonding_info.len()
        ),
        Err(_) => assert_eq!(
            init_state.ledger.unbonding_info.len() + 1,
            final_state.ledger.unbonding_info.len()
        ),
    }
    assert_eq!(
        init_state.ledger.unbonding_info.sum() + expected_unbond_amount,
        final_state.ledger.unbonding_info.sum()
    );

    // Push the unlocking chunk we expect to have at the end and compare two structs
    let mut unbonding_info = init_state.ledger.unbonding_info.clone();
    unbonding_info.add(UnlockingChunk {
        amount: expected_unbond_amount,
        unlock_era: current_era + UNBONDING_PERIOD,
    });
    assert_eq!(unbonding_info, final_state.ledger.unbonding_info);

    // Ensure that total locked value for staker hasn't been changed.
    assert_eq!(init_state.ledger.locked, final_state.ledger.locked);
    if final_state.ledger.is_empty() {
        assert!(!Ledger::<TestRuntime>::contains_key(&staker));
    }

    // Ensure that total staked amount has been decreased for contract and staking points are updated
    assert_eq!(
        init_state.contract_info.total - expected_unbond_amount,
        final_state.contract_info.total
    );
    assert_eq!(
        init_state.staker_info.latest_staked_value() - expected_unbond_amount,
        final_state.staker_info.latest_staked_value()
    );

    // Ensure that the number of stakers is as expected
    let delta = if remaining_staked > 0 { 0 } else { 1 };
    assert_eq!(
        init_state.contract_info.number_of_stakers - delta,
        final_state.contract_info.number_of_stakers
    );

    // Ensure that total staked value has been decreased
    assert_eq!(
        init_state.era_info.staked - expected_unbond_amount,
        final_state.era_info.staked
    );
    // Ensure that locked amount is the same since this will only start the unbonding period
    assert_eq!(init_state.era_info.locked, final_state.era_info.locked);
}

/// Used to perform start_unbonding with sucess and storage assertions.
pub(crate) fn assert_withdraw_unbonded(staker: AccountId) {
    let current_era = DappsStaking::current_era();

    let init_era_info = GeneralEraInfo::<TestRuntime>::get(current_era).unwrap();
    let init_ledger = Ledger::<TestRuntime>::get(&staker);

    // Get the current unlocking chunks
    let (valid_info, remaining_info) = init_ledger.unbonding_info.partition(current_era);
    let expected_unbond_amount = valid_info.sum();

    // Ensure op is successful and event is emitted
    assert_ok!(DappsStaking::withdraw_unbonded(Origin::signed(staker),));
    System::assert_last_event(mock::Event::DappsStaking(Event::Withdrawn(
        staker,
        expected_unbond_amount,
    )));

    // Fetch the latest unbonding info so we can compare it to expected remainder
    let final_ledger = Ledger::<TestRuntime>::get(&staker);
    assert_eq!(remaining_info, final_ledger.unbonding_info);
    if final_ledger.unbonding_info.is_empty() && final_ledger.locked == 0 {
        assert!(!Ledger::<TestRuntime>::contains_key(&staker));
    }

    // Compare the ledger and total staked value
    let final_rewards_and_stakes = GeneralEraInfo::<TestRuntime>::get(current_era).unwrap();
    assert_eq!(final_rewards_and_stakes.staked, init_era_info.staked);
    assert_eq!(
        final_rewards_and_stakes.locked,
        init_era_info.locked - expected_unbond_amount
    );
    assert_eq!(
        final_ledger.locked,
        init_ledger.locked - expected_unbond_amount
    );
}

/// Used to perform claim for stakers with success assertion
pub(crate) fn assert_claim_staker(claimer: AccountId, contract_id: &MockSmartContract<AccountId>) {
    let (claim_era, _) = DappsStaking::staker_info(&claimer, contract_id).claim();
    let init_state = MemorySnapshot::all(claim_era, contract_id, claimer);

    // Calculate contract portion of the reward
    let (_, stakers_joint_reward) =
        DappsStaking::dev_stakers_split(&init_state.contract_info, &init_state.era_info);

    let (claim_era, staked) = init_state.staker_info.clone().claim();
    assert!(claim_era > 0); // Sanity check - if this fails, method is being used incorrectly

    // Cannot claim rewards post unregister era, this indicates a bug!
    if let DAppState::Unregistered(unregistered_era) = init_state.dapp_info.state {
        assert!(unregistered_era > claim_era);
    }

    let calculated_reward =
        Perbill::from_rational(staked, init_state.contract_info.total) * stakers_joint_reward;
    let issuance_before_claim = <TestRuntime as Config>::Currency::total_issuance();

    assert_ok!(DappsStaking::claim_staker(
        Origin::signed(claimer),
        contract_id.clone(),
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::Reward(
        claimer,
        contract_id.clone(),
        claim_era,
        calculated_reward,
    )));

    let final_state = MemorySnapshot::all(claim_era, &contract_id, claimer);
    assert_eq!(
        init_state.free_balance + calculated_reward,
        final_state.free_balance
    );

    let (new_era, _) = final_state.staker_info.clone().claim();
    if final_state.staker_info.is_empty() {
        assert!(new_era.is_zero());
        assert!(!StakersInfo::<TestRuntime>::contains_key(
            &claimer,
            contract_id
        ));
    } else {
        assert!(new_era > claim_era);
    }
    assert!(new_era.is_zero() || new_era > claim_era);

    // Claim shouldn't mint new tokens, instead it should just transfer from the dapps staking pallet account
    let issuance_after_claim = <TestRuntime as Config>::Currency::total_issuance();
    assert_eq!(issuance_before_claim, issuance_after_claim);
}

/// Used to perform claim for dApp reward with success assertion
pub(crate) fn assert_claim_dapp(contract_id: &MockSmartContract<AccountId>, claim_era: EraIndex) {
    let developer = DappsStaking::dapp_info(contract_id).unwrap().developer;
    let init_state = MemorySnapshot::all(claim_era, contract_id, developer);
    assert!(!init_state.contract_info.contract_reward_claimed);

    // Cannot claim rewards post unregister era
    if let DAppState::Unregistered(unregistered_era) = init_state.dapp_info.state {
        assert!(unregistered_era > claim_era);
    }

    // Calculate contract portion of the reward
    let (calculated_reward, _) =
        DappsStaking::dev_stakers_split(&init_state.contract_info, &init_state.era_info);

    assert_ok!(DappsStaking::claim_dapp(
        Origin::signed(developer),
        contract_id.clone(),
        claim_era,
    ));
    System::assert_last_event(mock::Event::DappsStaking(Event::Reward(
        developer,
        contract_id.clone(),
        claim_era,
        calculated_reward,
    )));

    let final_state = MemorySnapshot::all(claim_era, &contract_id, developer);
    assert_eq!(
        init_state.free_balance + calculated_reward,
        final_state.free_balance
    );

    assert!(final_state.contract_info.contract_reward_claimed);

    // Just in case dev is also a staker - this shouldn't cause any change in StakerInfo or Ledger
    assert_eq!(init_state.staker_info, final_state.staker_info);
    assert_eq!(init_state.ledger, final_state.ledger);
}
