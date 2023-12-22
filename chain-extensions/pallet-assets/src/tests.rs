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

use crate::mock::*;
use frame_support::assert_ok;
use frame_support::traits::fungibles::roles::Inspect;
use frame_support::traits::Currency;
use pallet_contracts::Determinism;
use pallet_contracts_primitives::{Code, ExecReturnValue, ReturnFlags};
use parity_scale_codec::Encode;
use sp_core::crypto::AccountId32;
use sp_runtime::DispatchError;
use std::fs;

// Those tests use the contract scheduler_example avilable here:
// https://github.com/AstarNetwork/chain-extension-contracts/blob/main/examples/assets
// It maps chain extension functions to ink! callable messages
// ex:
// #[ink(message)]
// pub fn burn(&mut self, asset_id: u128, who: AccountId, amount: Balance) -> Result<(), AssetsError> {
//    AssetsExtension::burn(Origin::Caller, asset_id, who, amount)?;
//     Ok(())
// }

#[test]
fn create_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Act - Create asset
            assert_ok!(create(addr.clone(), 1, 1));

            // Assert - Contract is the owner
            assert_eq!(Assets::owner(1), Some(addr.into()));
        });
}

#[test]
fn mint_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create asset
            assert_ok!(create(addr.clone(), 1, 1));

            // Act - Mint 1000 assets to Alice
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));

            // Assert - Alice balance is 1000
            assert_eq!(Assets::balance(1, ALICE), 1000);
        });
}

#[test]
fn burn_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - Create & mint 1000 to Alice
            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));

            // Act - Burn 1000 of Alice tokens
            assert_ok!(burn(addr.clone(), 1, ALICE, 1000));

            // Assert - Balance of Alice is then 0
            assert_eq!(Assets::balance(1, ALICE), 0);
        });
}

#[test]
fn transfer_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Assert - Create & mint 1000 to contract
            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, addr.clone(), 1000));

            // Act - Tranfer 1000 from contract to Alice
            assert_ok!(transfer(addr.clone(), 1, ALICE, 1000));

            // Assert - Alice balance is 1000 and contract is zero
            assert_eq!(Assets::balance(1, ALICE), 1000);
            assert_eq!(Assets::balance(1, addr.clone()), 0);
        });
}

#[test]
fn balance_of_and_total_supply() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create & mint 1000 to Alice
            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));

            // Assert - Balance and total supply is 1000
            assert_eq!(
                balance_of(addr.clone(), 1, ALICE).data[1..],
                1000u128.encode()
            );
            assert_eq!(total_supply(addr.clone(), 1).data[1..], 1000u128.encode());
        });
}

#[test]
fn approve_transfer_and_check_allowance() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - Create and mint 1000 to contract
            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, addr.clone(), 1000));

            // Act - approve transfer To BOB for 100
            assert_ok!(approve_transfer(addr.clone(), 1, BOB, 100));

            // Assert - Bob has 100 allowance
            assert_eq!(
                allowance(addr.clone(), 1, addr.clone(), BOB).data[1..],
                100u128.encode()
            );
        });
}

#[test]
fn approve_transfer_and_transfer_balance() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange
            // As transfer_approved() can only be called on behalf of the contract
            // Bob creates & mint token to himself
            // and approve the contract to spend his assets
            assert_ok!(Assets::create(RuntimeOrigin::signed(BOB), 1, BOB, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(BOB), 1, BOB, 1000));
            assert_ok!(Assets::approve_transfer(
                RuntimeOrigin::signed(BOB),
                1,
                addr.clone(),
                100
            ));

            // Act - The contract transfer 100 from Alice to Bob
            assert_ok!(transfer_approved(addr.clone(), 1, BOB, ALICE, 100));

            // Assert - Bob has 900 and Alice 100
            assert_eq!(balance_of(addr.clone(), 1, BOB).data[1..], 900u128.encode());
            assert_eq!(
                balance_of(addr.clone(), 1, ALICE).data[1..],
                100u128.encode()
            );
        });
}

#[test]
fn cancel_approval_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - Create and mint 1000 to contract
            // and approve Bob to spend 100
            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, addr.clone(), 1000));
            assert_ok!(approve_transfer(addr.clone(), 1, BOB, 100));

            // Act - cancel approval
            assert_ok!(cancel_approval(addr.clone(), 1, BOB));

            // Assert - Bob allowance is 0
            assert_eq!(
                allowance(addr.clone(), 1, addr.clone(), BOB).data[1..],
                0u128.encode()
            );
        });
}

#[test]
fn set_metadata_and_checks() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create
            assert_ok!(create(addr.clone(), 1, 1));

            // Act - set metadata
            assert_ok!(set_metadata(
                addr.clone(),
                1,
                "Name".as_bytes().to_vec(),
                "SYMB".as_bytes().to_vec(),
                18
            ));

            // Assert - metadata Name, Symbol & decimal is correct
            assert_eq!(metadata_name(addr.clone(), 1).data[1..], "Name".encode());
            assert_eq!(metadata_symbol(addr.clone(), 1).data[1..], "SYMB".encode());
            assert_eq!(metadata_decimals(addr.clone(), 1).data[1..], 18u8.encode());
        });
}

#[test]
fn transfer_ownership_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Arrange - create token - owner is contract
            assert_ok!(create(addr.clone(), 1, 1));
            assert_eq!(Assets::owner(1), Some(addr.clone()));

            // Act - transfer ownership to Alice
            assert_ok!(transfer_ownership(addr.clone(), 1, ALICE));

            // Assert - Alice is the owner
            assert_eq!(Assets::owner(1), Some(ALICE));
        });
}

#[test]
fn cannot_make_tx_on_behalf_of_caller() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            // Assert
            // When calling chan extensio with Orgin::Caller
            // it reverts
            assert_eq!(
                create_caller(addr.clone(), 1, 1).unwrap(),
                ExecReturnValue {
                    flags: ReturnFlags::REVERT,
                    data: [0, 1, 98].into()
                }
            );
        });
}

fn instantiate() -> AccountId32 {
    let code = fs::read("./test-contract/asset_wrapper.wasm").expect("could not read .wasm file");
    let _ = Balances::deposit_creating(&ALICE, ONE * 1000);
    let _ = Balances::deposit_creating(&BOB, ONE * 1000);
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

fn create(
    addr: AccountId32,
    asset_id: u128,
    min_balance: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0xab, 0x70, 0x0a, 0x1b].to_vec(),
        (asset_id, min_balance).encode(),
    ]
    .concat();
    do_bare_call(addr, data, ONE)
}

fn create_caller(
    addr: AccountId32,
    asset_id: u128,
    min_balance: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x7f, 0xb0, 0xf9, 0xbb].to_vec(),
        (asset_id, min_balance).encode(),
    ]
    .concat();
    do_bare_call(addr, data, ONE)
}

fn mint(
    addr: AccountId32,
    asset_id: u128,
    beneficiary: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0xcf, 0xdd, 0x9a, 0xa2].to_vec(),
        (asset_id, beneficiary, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn burn(
    addr: AccountId32,
    asset_id: u128,
    who: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0xb1, 0xef, 0xc1, 0x7b].to_vec(),
        (asset_id, who, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn transfer(
    addr: AccountId32,
    asset_id: u128,
    target: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x84, 0xa1, 0x5d, 0xa1].to_vec(),
        (asset_id, target, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn transfer_approved(
    addr: AccountId32,
    asset_id: u128,
    owner: AccountId32,
    dest: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x31, 0x05, 0x59, 0x75].to_vec(),
        (asset_id, owner, dest, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn approve_transfer(
    addr: AccountId32,
    asset_id: u128,
    delegate: AccountId32,
    amount: u128,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x8e, 0x7c, 0x3e, 0xe9].to_vec(),
        (asset_id, delegate, amount).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn cancel_approval(
    addr: AccountId32,
    asset_id: u128,
    delegate: AccountId32,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x31, 0x7c, 0x8e, 0x29].to_vec(),
        (asset_id, delegate).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn transfer_ownership(
    addr: AccountId32,
    asset_id: u128,
    owner: AccountId32,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x10, 0x7e, 0x33, 0xea].to_vec(),
        (asset_id, owner).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn set_metadata(
    addr: AccountId32,
    asset_id: u128,
    name: Vec<u8>,
    symbol: Vec<u8>,
    decimals: u8,
) -> Result<ExecReturnValue, DispatchError> {
    let data = [
        [0x0b, 0x78, 0x7b, 0xb5].to_vec(),
        (asset_id, name, symbol, decimals).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0)
}

fn balance_of(addr: AccountId32, asset_id: u128, who: AccountId32) -> ExecReturnValue {
    let data = [[0x0f, 0x75, 0x5a, 0x56].to_vec(), (asset_id, who).encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn total_supply(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0xdb, 0x63, 0x75, 0xa8].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn allowance(
    addr: AccountId32,
    asset_id: u128,
    owner: AccountId32,
    delegate: AccountId32,
) -> ExecReturnValue {
    let data = [
        [0x6a, 0x00, 0x16, 0x5e].to_vec(),
        (asset_id, owner, delegate).encode(),
    ]
    .concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn metadata_name(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0xf5, 0xcd, 0xdb, 0xc1].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn metadata_symbol(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0x7c, 0xdc, 0xaf, 0xc1].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn metadata_decimals(addr: AccountId32, asset_id: u128) -> ExecReturnValue {
    let data = [[0x25, 0x54, 0x47, 0x3b].to_vec(), asset_id.encode()].concat();
    do_bare_call(addr, data, 0).unwrap()
}

fn do_bare_call(
    addr: AccountId32,
    input: Vec<u8>,
    value: u128,
) -> Result<ExecReturnValue, DispatchError> {
    Contracts::bare_call(
        ALICE,
        addr.into(),
        value.into(),
        GAS_LIMIT,
        None,
        input,
        false,
        Determinism::Deterministic,
    )
    .result
}
