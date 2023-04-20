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

use super::*;
use assert_matches::assert_matches;
use frame_support::assert_ok;
use mock::*;

#[test]
pub fn new_origin_works() {
    ExternalityBuilder::build().execute_with(|| {
        // Create native origin
        assert_ok!(Account::new_origin(
            RuntimeOrigin::signed(ALICE).into(),
            NativeAndEVMKind::Native,
        ));
        assert_eq!(
            AccountOrigin::<TestRuntime>::get(ALICE, 0),
            Some(NativeAndEVM::Native(ALICE_D1_NATIVE.into())),
        );
        assert_eq!(AccountOrigin::<TestRuntime>::get(ALICE, 1), None,);
        // Create EVM origin
        assert_ok!(Account::new_origin(
            RuntimeOrigin::signed(ALICE).into(),
            NativeAndEVMKind::H160,
        ));
        assert_eq!(
            AccountOrigin::<TestRuntime>::get(ALICE, 1),
            Some(NativeAndEVM::H160(ALICE_D2_H160.into())),
        );
    })
}

#[test]
pub fn proxy_call_works() {
    ExternalityBuilder::build().execute_with(|| {
        let call: RuntimeCall = pallet_balances::Call::transfer {
            dest: BOB,
            value: 10,
        }
        .into();

        // Create native origin
        assert_ok!(Account::new_origin(
            RuntimeOrigin::signed(ALICE).into(),
            NativeAndEVMKind::Native,
        ));

        // Make call with native origin
        assert_ok!(Account::proxy_call(
            RuntimeOrigin::signed(ALICE).into(),
            0,
            Box::new(call),
        ));
        assert_eq!(System::account(BOB).data.free, 810);
        assert_matches!(
            System::events()
                .last()
                .expect("events expected")
                .event
                .clone(),
            RuntimeEvent::Account(Event::ProxyCall{origin, ..})
            if origin == NativeAndEVM::Native(ALICE_D1_NATIVE.into())
        );
    })
}

#[test]
pub fn proxy_call_fails() {
    ExternalityBuilder::build().execute_with(|| {
        let call: RuntimeCall = pallet_balances::Call::transfer {
            dest: BOB,
            value: 10,
        }
        .into();

        // Make call with unknown origin
        assert_eq!(
            Account::proxy_call(
                RuntimeOrigin::signed(ALICE).into(),
                0,
                Box::new(call.clone()),
            ),
            Err(Error::<TestRuntime>::UnregisteredOrigin.into())
        );

        // Create native origin
        assert_ok!(Account::new_origin(
            RuntimeOrigin::signed(ALICE).into(),
            NativeAndEVMKind::Native,
        ));

        // Make call with native origin
        assert_eq!(
            Account::proxy_call(RuntimeOrigin::signed(ALICE).into(), 1, Box::new(call),),
            Err(Error::<TestRuntime>::UnregisteredOrigin.into())
        );
    })
}
