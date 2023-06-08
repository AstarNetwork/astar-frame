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
use crate::*;

use frame_support::assert_ok;
use std::collections::HashMap;

/// Helper struct used to store the entire pallet state snapshot.
/// Used when comparison of before/after states is required.
pub(crate) struct MemorySnapshot {
    active_protocol_state: ProtocolState<BlockNumberFor<Test>>,
    next_dapp_id: DAppId,
    current_era_info: EraInfo<BalanceOf<Test>>,
    integrated_dapps: HashMap<
        <Test as pallet::Config>::SmartContract,
        DAppInfo<<Test as frame_system::Config>::AccountId>,
    >,
    ledger: HashMap<<Test as frame_system::Config>::AccountId, AccountLedgerFor<Test>>,
}

impl MemorySnapshot {
    /// Generate a new memory snapshot, capturing entire dApp staking pallet state.
    pub fn new() -> Self {
        Self {
            active_protocol_state: ActiveProtocolState::<Test>::get(),
            next_dapp_id: NextDAppId::<Test>::get(),
            current_era_info: CurrentEraInfo::<Test>::get(),
            integrated_dapps: IntegratedDApps::<Test>::iter().collect(),
            ledger: Ledger::<Test>::iter().collect(),
        }
    }
}

/// Register contract for staking and assert success.
pub(crate) fn assert_register(owner: AccountId, smart_contract: &MockSmartContract) {
    // Init check to ensure smart contract hasn't already been integrated
    assert!(!IntegratedDApps::<Test>::contains_key(smart_contract));
    let pre_snapshot = MemorySnapshot::new();

    // Register smart contract
    assert_ok!(DappStaking::register(
        RuntimeOrigin::root(),
        owner,
        smart_contract.clone()
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppRegistered {
        owner,
        smart_contract: smart_contract.clone(),
        dapp_id: pre_snapshot.next_dapp_id,
    }));

    // Verify post-state
    let dapp_info = IntegratedDApps::<Test>::get(smart_contract).unwrap();
    assert_eq!(dapp_info.state, DAppState::Registered);
    assert_eq!(dapp_info.owner, owner);
    assert_eq!(dapp_info.id, pre_snapshot.next_dapp_id);
    assert!(dapp_info.reward_destination.is_none());

    assert_eq!(pre_snapshot.next_dapp_id + 1, NextDAppId::<Test>::get());
    assert_eq!(
        pre_snapshot.integrated_dapps.len() + 1,
        IntegratedDApps::<Test>::count() as usize
    );
}

/// Update dApp reward destination and assert success
pub(crate) fn assert_set_dapp_reward_destination(
    owner: AccountId,
    smart_contract: &MockSmartContract,
    beneficiary: Option<AccountId>,
) {
    // Change reward destination
    assert_ok!(DappStaking::set_dapp_reward_destination(
        RuntimeOrigin::signed(owner),
        smart_contract.clone(),
        beneficiary,
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppRewardDestination {
        smart_contract: smart_contract.clone(),
        beneficiary: beneficiary,
    }));

    // Sanity check & reward destination update
    assert_eq!(
        IntegratedDApps::<Test>::get(&smart_contract)
            .unwrap()
            .reward_destination,
        beneficiary
    );
}

/// Update dApp owner and assert success.
/// if `caller` is `None`, `Root` origin is used, otherwise standard `Signed` origin is used.
pub(crate) fn assert_set_dapp_owner(
    caller: Option<AccountId>,
    smart_contract: &MockSmartContract,
    new_owner: AccountId,
) {
    let origin = caller.map_or(RuntimeOrigin::root(), |owner| RuntimeOrigin::signed(owner));

    assert_ok!(DappStaking::set_dapp_owner(
        origin,
        smart_contract.clone(),
        new_owner,
    ));
    System::assert_last_event(RuntimeEvent::DappStaking(Event::DAppOwnerChanged {
        smart_contract: smart_contract.clone(),
        new_owner,
    }));

    // Verify post-state
    assert_eq!(
        IntegratedDApps::<Test>::get(&smart_contract).unwrap().owner,
        new_owner
    );
}
