use crate::mock::{Test, *};
use frame_support::assert_ok;
use frame_support::traits::{Contains, Currency};
use pallet_contracts::Determinism;
use pallet_contracts_primitives::Code;
use parity_scale_codec::{Encode , Decode};
use std::fs;
use frame_support::weights::Weight;

use frame_support::traits::schedule::DispatchTime;
use pallet_scheduler::Agenda;

#[test]
fn set_value_return_value() {
    let code =
        fs::read("./test-contract/scheduler_example.wasm").expect("could not read .wasm file");

    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let min_balance = <Test as pallet_contracts::Config>::Currency::minimum_balance();
            let _ = Balances::deposit_creating(&ALICE, 1000 * min_balance);

            let input: Vec<u8> = [0x9b, 0xae, 0x9d, 0x5e].to_vec();
            let addr = Contracts::bare_instantiate(
                ALICE,
                0,
                GAS_LIMIT,
                None,
                Code::Upload(code),
                input,
                vec![],
                false,
            )
            .result
            .unwrap()
            .account_id;

            let data: Vec<u8> = [0xf8, 0xaf, 0x07, 0x9d].to_vec();
            let result = Contracts::bare_call(
                ALICE,
                addr.clone(),
                0,
                GAS_LIMIT,
                None,
                data,
                false,
                Determinism::Deterministic,
            );
            assert_ok!(result.result);

            let selector = [0xca, 0x6f, 0x21, 0x70].to_vec();
            let result = Contracts::bare_call(
                ALICE,
                addr.clone(),
                0,
                GAS_LIMIT,
                None,
                selector,
                false,
                Determinism::Deterministic,
            )
            .result
            .unwrap();
            assert_eq!(result.data[1], 10);
        });
}

#[test]
fn chain_extension_works() {
    let code =
        fs::read("./test-contract/scheduler_example.wasm").expect("could not read .wasm file");

    ExtBuilder::default()
        .existential_deposit(50)
        .build()
        .execute_with(|| {
            let min_balance = <Test as pallet_contracts::Config>::Currency::minimum_balance();
            let _ = Balances::deposit_creating(&ALICE, 1000 * min_balance);
            let input: Vec<u8> = [0x9b, 0xae, 0x9d, 0x5e].to_vec();
            let addr = Contracts::bare_instantiate(
                ALICE,
                0,
                GAS_LIMIT,
                None,
                Code::Upload(code),
                input,
                vec![],
                false,
            )
            .result
            .unwrap()
            .account_id;

            run_to_block(1);

            // Call schedule - selector: 0x9db83191
            // #[ink(message)]
            // pub fn schedule(
            //      &mut self,
            //      when: BlockNumber,
            //      maybe_periodic: Option<(BlockNumber, u32)>,
            //  ) -> Result<(), SchedulerError>
            //
            // With args:
            // when: 3
            // maybe_periodic: None
            // let input = (selector, 5).encode();

            let mut data = Vec::new();
            data.append(&mut [0x9d, 0xb8, 0x31, 0x96].to_vec());
            data.append(&mut 5u64.encode());

            let selector: Vec<u8> = [0x9d, 0xb8, 0x31, 0x96].to_vec();
            let input = (selector, 5u64).encode();

            let result = Contracts::call(
                RuntimeOrigin::signed(ALICE),
                addr.clone().into(),
                0,
                GAS_LIMIT,
                None,
                data.clone(),
            );
            assert_ok!(result);


            let result = Contracts::bare_call(
                ALICE,
                addr.clone().into(),
                0,
                GAS_LIMIT,
                None,
                input,
                false,
                Determinism::Deterministic,
            );
            assert_ok!(result.result);

            // println!("{:?}", Agenda::<Test>::get(5));
            assert!(Agenda::<Test>::get(5).len() == 1);

            run_to_block(5);

            let selector = [0xca, 0x6f, 0x21, 0x70].to_vec();
            let result = Contracts::bare_call(
                ALICE,
                addr.clone(),
                0,
                GAS_LIMIT,
                None,
                selector,
                false,
                Determinism::Deterministic,
            )
            .result
            .unwrap();
            assert_eq!(result.data[1], 10);
        });
}

#[test]
fn basic_scheduling_works() {
    new_test_ext().execute_with(|| {
        let call =
            RuntimeCall::Logger(LoggerCall::log { i: 42, weight: Weight::from_ref_time(10) });
        assert_ok!(Scheduler::schedule(
            RuntimeOrigin::signed(ALICE),
			4,
			None,
			127,
			Box::new(call)
		));
        println!("{:?}", Agenda::<Test>::get(4));
        assert!(Agenda::<Test>::get(4).len() == 1);

        run_to_block(3);
        assert!(logger::log().is_empty());
        run_to_block(4);
        assert_eq!(logger::log(), vec![(frame_system::RawOrigin::Signed(ALICE).into(), 42u32)]);
        run_to_block(100);
        assert_eq!(logger::log(), vec![(frame_system::RawOrigin::Signed(ALICE).into(), 42u32)]);
    });
}
