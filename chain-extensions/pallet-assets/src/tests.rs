use crate::mock::*;
use frame_support::assert_ok;
use frame_support::traits::fungibles::roles::Inspect;
use frame_support::traits::Currency;
use pallet_contracts::Determinism;
use pallet_contracts_primitives::{Code, ExecReturnValue};
use parity_scale_codec::{Decode, Encode};
use sp_core::crypto::AccountId32;
use sp_runtime::DispatchError;
use std::fs;

#[test]
fn create_works() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            assert_ok!(create(addr.clone(), 1, 1));

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

            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));

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

            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));
            assert_eq!(Assets::balance(1, ALICE), 1000);

            assert_ok!(burn(addr.clone(), 1, ALICE, 1000));
            assert_eq!(Assets::balance(1, ALICE), 0);
        });
}

#[test]
fn balance_of_and_total_supply() {
    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let addr = instantiate();

            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, ALICE, 1000));

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

            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, addr.clone(), 1000));
            assert_ok!(approve_transfer(addr.clone(), 1, BOB, 100));

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

            assert_ok!(Assets::create(RuntimeOrigin::signed(ALICE), 1, ALICE, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), 1, ALICE, 1000));
            assert_ok!(Assets::approve_transfer(
                RuntimeOrigin::signed(ALICE),
                1,
                addr.clone(),
                100
            ));

            // assert_ok!(create(addr.clone(), 1, 1));
            // assert_ok!(mint(addr.clone(), 1, addr.clone(), 1000));
            // assert_ok!(approve_transfer(addr.clone(), 1, ALICE, 100));

            assert_eq!(
                allowance(addr.clone(), 1, ALICE, addr.clone()).data[1..],
                100u128.encode()
            );

            assert_ok!(transfer_approved(addr.clone(), 1, ALICE, BOB, 100));

            assert_eq!(balance_of(addr.clone(), 1, BOB).data[1..], 100u128.encode());
            assert_eq!(
                balance_of(addr.clone(), 1, ALICE).data[1..],
                900u128.encode()
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

            assert_ok!(create(addr.clone(), 1, 1));
            assert_ok!(mint(addr.clone(), 1, addr.clone(), 1000));
            assert_ok!(approve_transfer(addr.clone(), 1, BOB, 100));

            assert_ok!(cancel_approval(addr.clone(), 1, BOB));

            assert_eq!(
                allowance(addr.clone(), 1, addr.clone(), BOB).data[1..],
                0u128.encode()
            );
        });
}

// ________________________________
fn instantiate() -> AccountId32 {
    let code = fs::read("./test-contract/asset_wrapper.wasm").expect("could not read .wasm file");
    let _ = Balances::deposit_creating(&ALICE, ONE * 1000);
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
        [0xb1, 0xef, 0xc1, 0x7b].to_vec(),
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
