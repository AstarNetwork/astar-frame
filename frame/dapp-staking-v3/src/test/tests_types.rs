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

// Helper to generate custom `Get` types for testing the `AccountLedger` struct.
macro_rules! get_u32_type {
    ($struct_name:ident, $value:expr) => {
        struct $struct_name;
        impl Get<u32> for $struct_name {
            fn get() -> u32 {
                $value
            }
        }
    };
}

#[test]
fn protocol_state_default() {
    let protoc_state = ProtocolState::<BlockNumber>::default();

    assert_eq!(protoc_state.era, 0);
    assert_eq!(
        protoc_state.next_era_start, 1,
        "Era should start immediately on the first block"
    );
}

#[test]
fn account_ledger_default() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let acc_ledger = AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    assert!(acc_ledger.is_empty());
    assert!(acc_ledger.active_locked_amount().is_zero());
    assert!(acc_ledger.lock_era().is_zero());
    assert!(acc_ledger.latest_locked_chunk().is_none());
}

#[test]
fn account_ledger_add_lock_amount_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // First step, sanity checks
    let first_era = 1;
    assert!(acc_ledger.active_locked_amount().is_zero());
    assert!(acc_ledger.total_locked_amount().is_zero());
    assert!(acc_ledger.add_lock_amount(0, first_era).is_ok());
    assert!(acc_ledger.active_locked_amount().is_zero());

    // Adding lock value works as expected
    let init_amount = 20;
    assert!(acc_ledger.add_lock_amount(init_amount, first_era).is_ok());
    assert_eq!(acc_ledger.active_locked_amount(), init_amount);
    assert_eq!(acc_ledger.total_locked_amount(), init_amount);
    assert_eq!(acc_ledger.lock_era(), first_era);
    assert!(!acc_ledger.is_empty());
    assert_eq!(acc_ledger.locked.len(), 1);
    assert_eq!(
        acc_ledger.latest_locked_chunk(),
        Some(&LockedChunk::<Balance> {
            amount: init_amount,
            era: first_era,
        })
    );

    // Add to the same era
    let addition = 7;
    assert!(acc_ledger.add_lock_amount(addition, first_era).is_ok());
    assert_eq!(acc_ledger.active_locked_amount(), init_amount + addition);
    assert_eq!(acc_ledger.total_locked_amount(), init_amount + addition);
    assert_eq!(acc_ledger.lock_era(), first_era);
    assert_eq!(acc_ledger.locked.len(), 1);

    // Add up to storage limit
    for i in 2..=LockedDummy::get() {
        assert!(acc_ledger.add_lock_amount(addition, first_era + i).is_ok());
        assert_eq!(
            acc_ledger.active_locked_amount(),
            init_amount + addition * i as u128
        );
        assert_eq!(acc_ledger.lock_era(), first_era + i);
        assert_eq!(acc_ledger.locked.len(), i as usize);
    }

    // Any further additions should fail due to exhausting bounded storage capacity
    assert!(acc_ledger
        .add_lock_amount(addition, acc_ledger.lock_era() + 1)
        .is_err());
    assert!(!acc_ledger.is_empty());
    assert_eq!(acc_ledger.locked.len(), LockedDummy::get() as usize);
}

#[test]
fn account_ledger_subtract_lock_amount_basic_usage_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Sanity check scenario
    // Cannot reduce if there is nothing locked, should be a noop
    assert!(acc_ledger.subtract_lock_amount(0, 1).is_ok());
    assert!(acc_ledger.subtract_lock_amount(10, 1).is_ok());
    assert!(acc_ledger.locked.len().is_zero());
    assert!(acc_ledger.is_empty());

    // First basic scenario
    // Add some lock amount, then reduce it for the same era
    let first_era = 1;
    let first_lock_amount = 19;
    let unlock_amount = 7;
    assert!(acc_ledger
        .add_lock_amount(first_lock_amount, first_era)
        .is_ok());
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, first_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 1);
    assert_eq!(
        acc_ledger.total_locked_amount(),
        first_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.active_locked_amount(),
        first_lock_amount - unlock_amount
    );
    assert_eq!(acc_ledger.unlocking_amount(), 0);

    // Second basic scenario
    // Reduce the lock from the era which isn't latest in the vector
    let first_lock_amount = first_lock_amount - unlock_amount;
    let second_lock_amount = 31;
    let second_era = 2;
    assert!(acc_ledger
        .add_lock_amount(second_lock_amount - first_lock_amount, second_era)
        .is_ok());
    assert_eq!(acc_ledger.active_locked_amount(), second_lock_amount);
    assert_eq!(acc_ledger.locked.len(), 2);

    // Subtract from the first era and verify state is as expected
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, first_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 2);
    assert_eq!(
        acc_ledger.active_locked_amount(),
        second_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.locked[0].amount,
        first_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.locked[1].amount,
        second_lock_amount - unlock_amount
    );

    // Third basic scenario
    // Reduce the the latest era, don't expect the first one to change
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, second_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 2);
    assert_eq!(
        acc_ledger.active_locked_amount(),
        second_lock_amount - unlock_amount * 2
    );
    assert_eq!(
        acc_ledger.locked[0].amount,
        first_lock_amount - unlock_amount
    );
    assert_eq!(
        acc_ledger.locked[1].amount,
        second_lock_amount - unlock_amount * 2
    );
}

#[test]
fn account_ledger_subtract_lock_amount_overflow_fails() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    let first_lock_amount = 17 * 19;
    let era = 1;
    let unlock_amount = 5;
    assert!(acc_ledger.add_lock_amount(first_lock_amount, era).is_ok());
    for idx in 1..=LockedDummy::get() {
        assert!(acc_ledger.subtract_lock_amount(unlock_amount, idx).is_ok());
        assert_eq!(acc_ledger.locked.len(), idx as usize);
        assert_eq!(
            acc_ledger.active_locked_amount(),
            first_lock_amount - unlock_amount * idx as u128
        );
    }

    // Updating existing lock should still work
    for _ in 1..10 {
        assert!(acc_ledger
            .subtract_lock_amount(unlock_amount, LockedDummy::get())
            .is_ok());
    }

    // Attempt to add additional chunks should fail.
    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, LockedDummy::get() + 1)
        .is_err());
}

#[test]
fn account_ledger_subtract_lock_amount_advanced_example_works() {
    get_u32_type!(LockedDummy, 5);
    get_u32_type!(UnlockingDummy, 5);
    let mut acc_ledger =
        AccountLedger::<Balance, BlockNumber, LockedDummy, UnlockingDummy>::default();

    // Prepare an example where we have two non-consecutive entries, and we unlock in the era right before the second entry.
    // This covers a scenario where user has locked in the current era,
    // creating an entry for the next era, and then decides to immediately unlock.
    let first_lock_amount = 17;
    let second_lock_amount = 23;
    let first_era = 1;
    let second_era = 5;
    let unlock_era = second_era - 1;
    let unlock_amount = 5;
    assert!(acc_ledger
        .add_lock_amount(first_lock_amount, first_era)
        .is_ok());
    assert!(acc_ledger
        .add_lock_amount(second_lock_amount, second_era)
        .is_ok());
    assert_eq!(acc_ledger.locked.len(), 2);

    assert!(acc_ledger
        .subtract_lock_amount(unlock_amount, unlock_era)
        .is_ok());
    assert_eq!(
        acc_ledger.active_locked_amount(),
        first_lock_amount + second_lock_amount - unlock_amount
    );

    // Check entries in more detail
    assert_eq!(acc_ledger.locked.len(), 3);
    assert_eq!(acc_ledger.locked[0].amount, first_lock_amount,);
    assert_eq!(
        acc_ledger.locked[2].amount,
        first_lock_amount + second_lock_amount - unlock_amount
    );
    // Verify the new entry
    assert_eq!(
        acc_ledger.locked[1].amount,
        first_lock_amount - unlock_amount
    );
    assert_eq!(acc_ledger.locked[1].era, unlock_era);
}
