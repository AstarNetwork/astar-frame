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

use crate::mock::{Test, *};
use frame_support::assert_ok;
use frame_support::traits::Currency;
use pallet_contracts::Determinism;
use pallet_contracts_primitives::{Code, ExecReturnValue};
use pallet_scheduler::Agenda;
use parity_scale_codec::Encode;
use sp_runtime::{AccountId32, DispatchError};
use std::fs;

// Those tests use the contract scheduler_example avilable here:
// https://github.com/swanky-dapps/chain-extension-contracts/blob/feature/scheduler/examples/scheduler/lib.rs
// It stores a u32 value in storage (default value: 0):
// #[ink(storage)]
// #[derive(Default)]
// pub struct Scheduler {
//    value: u32,
// }
//
// and `schedule()` will schedule a call to `increase_value()` that will increment value by 10
// #[ink(message)]
// pub fn increase_value(&mut self) {
//     self.value += 10;
// }

#[test]
fn schedule_call_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Schedule a call to `increase_value()` that will increment value by 10
            // params
            // when: BlockNumber = 5
            // maybe_periodic: None
            let result = schedule(addr.clone(), 5, None);
            assert_ok!(result);

            // Assert that the scheduled call is part of Scheduler Agenda
            assert!(Agenda::<Test>::get(5).len() == 1);

            // Run tu block 5 add assert value has been incremented
            run_to_block(5);
            let result = get_value(addr);
            assert_eq!(result.data[1], 10);
        });
}

#[test]
fn schedule_periodic_call() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Schedule a call to `increase_value()` that will increment value by 10
            // params
            // when: BlockNumber = 5
            // maybe_periodic: every 2 blocks, 3 times
            let result = schedule(addr.clone(), 5, Some((2, 3)));
            assert_ok!(result);

            // Assert that the scheduled call is part of Scheduler Agenda
            assert!(Agenda::<Test>::get(5).len() == 1);

            // Run tu block 5 add assert value has been incremented
            run_to_block(5);
            let result = get_value(addr.clone());
            assert_eq!(result.data[1], 10);

            // Run tu block 7 add assert value has been incremented
            run_to_block(7);
            let result = get_value(addr.clone());
            assert_eq!(result.data[1], 20);

            // Run tu block 7 add assert value has been incremented
            run_to_block(9);
            let result = get_value(addr.clone());
            assert_eq!(result.data[1], 30);
        });
}

#[test]
fn cancel_call() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Schedule a call to `increase_value()` that will increment value by 10
            // params
            // when: BlockNumber = 5
            // maybe_periodic: None
            let result = schedule(addr.clone(), 5, None);
            assert_ok!(result);

            // Assert that the scheduled call is part of Scheduler Agenda
            assert!(Agenda::<Test>::get(5).len() == 1);

            // Cancel the call
            // params
            // when: BlockNumber = 5
            // index: 0
            let result = cancel(addr.clone(), 5, 0);
            assert_ok!(result);

            // Run tu block 5 add assert value has not been incremented
            run_to_block(5);
            let result = get_value(addr.clone());
            assert_eq!(result.data[1], 0);
        });
}

fn instantiate() -> AccountId32 {
    let code =
        fs::read("./test-contract/scheduler_example.wasm").expect("could not read .wasm file");
    let min_balance = <Test as pallet_contracts::Config>::Currency::minimum_balance();
    let _ = Balances::deposit_creating(&ALICE, 1000 * min_balance);
    let instance_selector: Vec<u8> = [0x9b, 0xae, 0x9d, 0x5e].to_vec();
    Contracts::bare_instantiate(
        ALICE,
        0,
        GAS_LIMIT,
        None,
        Code::Upload(code),
        instance_selector,
        vec![],
        false,
    )
    .result
    .unwrap()
    .account_id
}

fn schedule(
    addr: AccountId32,
    when: u32,
    maybe_periodic: Option<(u32, u32)>,
) -> Result<ExecReturnValue, DispatchError> {
    let mut data = Vec::new();
    data.append(&mut [0x9d, 0xb8, 0x31, 0x96].to_vec());
    data.append(&mut when.encode());
    data.append(&mut maybe_periodic.encode());

    do_bare_call(addr, data)
}

fn cancel(addr: AccountId32, when: u32, index: u32) -> Result<ExecReturnValue, DispatchError> {
    let mut data = Vec::new();
    data.append(&mut [0x97, 0x96, 0xe9, 0xa7].to_vec());
    data.append(&mut when.encode());
    data.append(&mut index.encode());

    do_bare_call(addr, data)
}

fn get_value(addr: AccountId32) -> ExecReturnValue {
    let selector = [0xca, 0x6f, 0x21, 0x70].to_vec();
    do_bare_call(addr, selector).unwrap()
}

fn do_bare_call(addr: AccountId32, input: Vec<u8>) -> Result<ExecReturnValue, DispatchError> {
    Contracts::bare_call(
        ALICE,
        addr.into(),
        0,
        GAS_LIMIT,
        None,
        input,
        false,
        Determinism::Deterministic,
    )
    .result
}
