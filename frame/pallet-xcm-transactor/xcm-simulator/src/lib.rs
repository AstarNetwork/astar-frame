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

#[cfg(test)]
mod mocks;

#[cfg(test)]
mod tests {
    use crate::mocks::{parachain, *};

    use frame_support::{assert_ok, traits::fungible::Inspect, weights::Weight};
    use pallet_contracts::Determinism;
    use pallet_xcm_transactor::{
        chain_extension::{ValidateSendInput, XcmCeError as XcmCEError},
        QueryConfig, QueryType,
    };
    use parity_scale_codec::{Decode, Encode};
    use sp_runtime::traits::Bounded;
    use xcm::prelude::*;
    use xcm_simulator::TestExt;

    type AccoundIdOf<T> = <T as frame_system::Config>::AccountId;
    type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

    const GAS_LIMIT: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);

    const SELECTOR_CONSTRUCTOR: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
    const SELECTOR_EXECUTE: [u8; 4] = [0x11, 0x11, 0x11, 0x11];
    const SELECTOR_SEND: [u8; 4] = [0x22, 0x22, 0x22, 0x22];
    const SELECTOR_QUERY: [u8; 4] = [0x33, 0x33, 0x33, 0x33];
    const SELECTOR_HANDLE_RESPONSE: [u8; 4] = [0x55, 0x55, 0x55, 0x55];
    const SELECTOR_GET: [u8; 4] = [0x66, 0x66, 0x66, 0x66];

    struct Fixture {
        pub basic_flip_id: parachain::AccountId,
    }

    /// Deploy the xcm flipper contract in ParaA and fund it's derive account
    /// in ParaB
    fn xcm_flipper_fixture() -> Fixture {
        // deploy and initialize xcm flipper contract with `false` in ParaA
        let mut basic_flip_id = [0u8; 32].into();
        ParaA::execute_with(|| {
            (basic_flip_id, _) = deploy_contract::<parachain::Runtime>(
                "basic_flip",
                ALICE.into(),
                0,
                GAS_LIMIT,
                None,
                // selector + true
                SELECTOR_CONSTRUCTOR.to_vec(),
            );

            // check for flip status, should be false
            let outcome = ParachainContracts::bare_call(
                ALICE.into(),
                basic_flip_id.clone(),
                0,
                GAS_LIMIT,
                None,
                SELECTOR_GET.to_vec(),
                true,
                Determinism::Deterministic,
            );
            let res = outcome.result.unwrap();
            // check for revert
            assert!(!res.did_revert());
            // decode the return value
            let flag = Result::<bool, ()>::decode(&mut res.data.as_ref()).unwrap();
            assert_eq!(flag, Ok(false));
        });

        // transfer funds to contract derieve account in ParaB
        ParaB::execute_with(|| {
            use parachain::System;

            let account = sibling_para_account_account_id(1, basic_flip_id.clone());
            assert_ok!(ParachainBalances::transfer(
                parachain::RuntimeOrigin::signed(ALICE),
                account,
                INITIAL_BALANCE / 2,
            ));

            System::reset_events();
        });

        Fixture { basic_flip_id }
    }

    /// Execute XCM from contract via CE
    #[test]
    fn test_ce_execute() {
        MockNet::reset();

        let Fixture {
            basic_flip_id: contract_id,
        } = xcm_flipper_fixture();

        //
        //  check the execute
        //
        ParaA::execute_with(|| {
            let transfer_amount = 100_000;
            // transfer some native to contract
            assert_ok!(ParachainBalances::transfer(
                parachain::RuntimeOrigin::signed(ALICE),
                contract_id.clone(),
                transfer_amount,
            ));

            let xcm: Xcm<()> = Xcm(vec![
                WithdrawAsset((Here, transfer_amount).into()),
                BuyExecution {
                    fees: (Here, transfer_amount).into(),
                    weight_limit: Unlimited,
                },
                DepositAsset {
                    assets: All.into(),
                    beneficiary: AccountId32 {
                        network: None,
                        id: ALICE.into(),
                    }
                    .into(),
                },
            ]);

            // run execute in contract
            let alice_balance_before = ParachainBalances::balance(&ALICE.into());
            let (res, _, _) =
                call_contract_method::<parachain::Runtime, Result<Result<Weight, XcmCEError>, ()>>(
                    ALICE.into(),
                    contract_id.clone(),
                    0,
                    GAS_LIMIT,
                    None,
                    [SELECTOR_EXECUTE.to_vec(), VersionedXcm::V3(xcm).encode()].concat(),
                    true,
                );

            assert_eq!(res, Ok(Ok(Weight::from_parts(30, 0))));
            assert!(
                // TODO: since bare_call doesn't charge, use call
                ParachainBalances::balance(&ALICE.into()) == alice_balance_before + transfer_amount
            );
        });
    }

    /// Send the XCM and handle response callback via CE
    #[test]
    fn test_ce_wasm_callback() {
        MockNet::reset();

        let Fixture {
            basic_flip_id: contract_id,
        } = xcm_flipper_fixture();

        //
        // Check send & callback query
        //
        ParaA::execute_with(|| {
            use parachain::{Runtime, RuntimeCall};

            let remark_call = RuntimeCall::System(frame_system::Call::remark_with_event {
                remark: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 0],
            });

            let config = QueryConfig::<AccoundIdOf<Runtime>, BlockNumberOf<Runtime>> {
                query_type: QueryType::WASMContractCallback {
                    contract_id: contract_id.clone(),
                    selector: SELECTOR_HANDLE_RESPONSE,
                },
                timeout: Bounded::max_value(),
            };
            let dest: VersionedMultiLocation = (Parent, Parachain(2)).into();

            // register the callback query
            let (res, _, _) = call_contract_method::<
                parachain::Runtime,
                Result<Result<QueryId, XcmCEError>, ()>,
            >(
                ALICE.into(),
                contract_id.clone(),
                0,
                GAS_LIMIT,
                None,
                [
                    SELECTOR_QUERY.to_vec(),
                    (config.clone(), dest.clone()).encode(),
                ]
                .concat(),
                true,
            );
            assert_eq!(res, Ok(Ok(0)));
            let query_id = res.unwrap().unwrap();

            let xcm: Xcm<()> = Xcm(vec![
                WithdrawAsset((Here, INITIAL_BALANCE / 2).into()),
                BuyExecution {
                    fees: (Here, INITIAL_BALANCE / 2).into(),
                    weight_limit: Unlimited,
                },
                SetAppendix(Xcm(vec![ReportTransactStatus(QueryResponseInfo {
                    destination: (Parent, Parachain(1)).into(),
                    query_id,
                    max_weight: parachain::CallbackGasLimit::get(),
                })])),
                Transact {
                    origin_kind: OriginKind::SovereignAccount,
                    require_weight_at_most: Weight::from_parts(
                        100_000_000_000_000,
                        1024 * 1024 * 1024,
                    ),
                    call: remark_call.encode().into(),
                },
            ]);

            // send xcm
            let (_res, _, _) = call_contract_method::<
                parachain::Runtime,
                Result<Result<VersionedMultiAssets, XcmCEError>, ()>,
            >(
                ALICE.into(),
                contract_id.clone(),
                0,
                GAS_LIMIT,
                None,
                [
                    SELECTOR_SEND.to_vec(),
                    ValidateSendInput {
                        dest,
                        xcm: VersionedXcm::V3(xcm),
                    }
                    .encode(),
                ]
                .concat(),
                true,
            );

            // dbg!(res);
        });

        // check if remark was executed in ParaB
        ParaB::execute_with(|| {
            use parachain::{RuntimeEvent, System};
            // check remark events
            assert!(System::events().iter().any(|r| matches!(
                r.event,
                RuntimeEvent::System(frame_system::Event::Remarked { .. })
            )));

            // clear the events
            System::reset_events();
        });

        // check for callback, if callback success then flip=true
        ParaA::execute_with(|| {
            // check for flip status
            let (res, _, _) = call_contract_method::<parachain::Runtime, Result<bool, ()>>(
                ALICE.into(),
                contract_id.clone(),
                0,
                GAS_LIMIT,
                None,
                SELECTOR_GET.to_vec(),
                true,
            );
            assert_eq!(res, Ok(true));
        });
    }
}
