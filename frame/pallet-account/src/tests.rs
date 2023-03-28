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
use frame_support::{assert_err, assert_ok};
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
            AccountOrigin::<TestRuntime>::get(ALICE),
            vec![NativeAndEVM::Native(ALICE_D1_NATIVE.into())],
        );
        // Create EVM origin
        assert_ok!(Account::new_origin(
            RuntimeOrigin::signed(ALICE).into(),
            NativeAndEVMKind::H160,
        ));
        assert_eq!(
            AccountOrigin::<TestRuntime>::get(ALICE),
            vec![
                NativeAndEVM::Native(ALICE_D1_NATIVE.into()),
                NativeAndEVM::H160(ALICE_D2_H160.into())
            ],
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
pub fn meta_call_works() {
    use parity_scale_codec::Encode;
    use sp_core::Pair;
    use sp_runtime::traits::IdentifyAccount;

    ExternalityBuilder::build().execute_with(|| {
        let call: RuntimeCall = pallet_balances::Call::transfer {
            dest: BOB,
            value: 10,
        }
        .into();

        let pair = sp_core::ed25519::Pair::from_string("//Alice", None).unwrap();
        let payload = (ChainMagic::get(), 0u64, call.clone());
        let signer: sp_runtime::MultiSigner = pair.public().into();
        let account_id = signer.into_account();
        let signature = pair.sign(&payload.encode()[..]);

        // Make call with signer as origin
        assert_eq!(System::account(&account_id).nonce, 0);

        assert_ok!(Account::meta_call(
            RuntimeOrigin::signed(ALICE).into(),
            Box::new(call),
            account_id.clone(),
            signature.into(),
        ));

        assert_eq!(System::account(&account_id).nonce, 1);
        assert_eq!(System::account(BOB).data.free, 810);
    })
}

#[test]
pub fn meta_call_bad_signature() {
    ExternalityBuilder::build().execute_with(|| {
        let call: RuntimeCall = pallet_balances::Call::transfer {
            dest: BOB,
            value: 10,
        }
        .into();

        let bad_signature = sp_runtime::MultiSignature::Ecdsa(Default::default());

        assert_err!(
            Account::meta_call(
                RuntimeOrigin::signed(ALICE).into(),
                Box::new(call),
                ALICE,
                bad_signature,
            ),
            Error::<TestRuntime>::BadSignature,
        );
    })
}
