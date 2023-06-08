// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use crate::test::mock::*;
use crate::test::testing_utils::*;
use crate::{pallet as pallet_dapp_staking, DAppId, Error, IntegratedDApps, NextDAppId};
use frame_support::{assert_noop, error::BadOrigin, traits::Get};

#[test]
fn register_is_ok() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        // Basic test
        assert_register(5, &MockSmartContract::Wasm(1));

        // Register two contracts using the same owner
        assert_register(7, &MockSmartContract::Wasm(2));
        assert_register(7, &MockSmartContract::Wasm(3));
    })
}

#[test]
fn register_with_incorrect_origin_fails() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        assert_noop!(
            DappStaking::register(RuntimeOrigin::signed(1), 3, MockSmartContract::Wasm(2)),
            BadOrigin
        );
    })
}

#[test]
fn register_already_registered_contract_fails() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        let smart_contract = MockSmartContract::Wasm(1);
        assert_register(2, &smart_contract);
        assert_noop!(
            DappStaking::register(RuntimeOrigin::root(), 2, smart_contract),
            Error::<Test>::ContractAlreadyExists
        );
    })
}

#[test]
fn register_past_max_number_of_contracts_fails() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        let limit = <Test as pallet_dapp_staking::Config>::MaxNumberOfContracts::get();
        for id in 1..=limit {
            assert_register(1, &MockSmartContract::Wasm(id.into()));
        }

        assert_noop!(
            DappStaking::register(
                RuntimeOrigin::root(),
                2,
                MockSmartContract::Wasm((limit + 1).into())
            ),
            Error::<Test>::ExcededMaxNumberOfContracts
        );
    })
}

#[test]
fn register_past_sentinel_value_of_id_fails() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        // hacky approach, but good enough for test
        NextDAppId::<Test>::put(DAppId::MAX - 1);

        // First register should pass since sentinel value hasn't been reached yet
        assert_register(1, &MockSmartContract::Wasm(3));

        // Second one should fail since we've reached the sentine value and cannot add more contracts
        assert_eq!(NextDAppId::<Test>::get(), DAppId::MAX);
        assert_noop!(
            DappStaking::register(RuntimeOrigin::root(), 1, MockSmartContract::Wasm(5)),
            Error::<Test>::NewDAppIdUnavailable
        );
    })
}

#[test]
fn set_dapp_reward_destination_for_contract_is_ok() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        // Prepare & register smart contract
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Update beneficiary
        assert!(IntegratedDApps::<Test>::get(&smart_contract)
            .unwrap()
            .reward_destination
            .is_none());
        assert_set_dapp_reward_destination(owner, &smart_contract, Some(3));
        assert_set_dapp_reward_destination(owner, &smart_contract, Some(5));
        assert_set_dapp_reward_destination(owner, &smart_contract, None);
    })
}

#[test]
fn set_dapp_reward_destination_fails() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);

        // Contract doesn't exist yet
        assert_noop!(
            DappStaking::set_dapp_reward_destination(
                RuntimeOrigin::signed(owner),
                smart_contract,
                Some(5)
            ),
            Error::<Test>::ContractNotFound
        );

        // Non-owner cannnot change reward destination
        assert_register(owner, &smart_contract);
        assert_noop!(
            DappStaking::set_dapp_reward_destination(
                RuntimeOrigin::signed(owner + 1),
                smart_contract,
                Some(5)
            ),
            Error::<Test>::OriginNotOwner
        );
    })
}

#[test]
fn set_dapp_owner_is_ok() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        // Prepare & register smart contract
        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);
        assert_register(owner, &smart_contract);

        // Update owner
        let new_owner = 7;
        assert_set_dapp_owner(Some(owner), &smart_contract, new_owner);
        assert_set_dapp_owner(Some(new_owner), &smart_contract, 1337);

        // Ensure manager can bypass owner
        assert_set_dapp_owner(None, &smart_contract, owner);
    })
}

#[test]
fn set_dapp_owner_fails() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        let owner = 1;
        let smart_contract = MockSmartContract::Wasm(3);

        // Contract doesn't exist yet
        assert_noop!(
            DappStaking::set_dapp_owner(RuntimeOrigin::signed(owner), smart_contract, 5),
            Error::<Test>::ContractNotFound
        );

        // Ensure non-owner cannot steal ownership
        assert_register(owner, &smart_contract);
        assert_noop!(
            DappStaking::set_dapp_owner(
                RuntimeOrigin::signed(owner + 1),
                smart_contract,
                owner + 1
            ),
            Error::<Test>::OriginNotOwner
        );
    })
}
