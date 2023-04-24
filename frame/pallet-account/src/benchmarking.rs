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

use hex_literal::hex;
use parity_scale_codec::{Decode, Encode};

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::{Pallet as System, RawOrigin};

const WHITELISTED_D1_NATIVE: [u8; 32] =
    hex!["53ebafb5200b910e56654c867e08747f7dbd3695d6c133af24084b23c3253a67"];

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    System::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
    new_origin {
        let caller = whitelisted_caller::<T::AccountId>();
        let origin_kind = NativeAndEVMKind::Native.encode();
        let origin = NativeAndEVM::Native(WHITELISTED_D1_NATIVE.into()).encode();
    }: _(RawOrigin::Signed(caller.clone()), Decode::decode(&mut &origin_kind[..]).unwrap())
    verify {
        assert_last_event::<T>(
            Event::<T>::NewOrigin {
                account: caller,
                origin: Decode::decode(&mut &origin[..]).unwrap(),
            }.into()
        );
    }

    proxy_call {
        let caller = whitelisted_caller::<T::AccountId>();
        let origin_kind = NativeAndEVMKind::Native.encode();
        assert_ok!(Pallet::<T>::new_origin(
            RawOrigin::Signed(caller.clone()).into(),
            Decode::decode(&mut &origin_kind[..]).unwrap()
        ));

        let call = Box::new(frame_system::Call::remark { remark: vec![42u8] }.into());
        let origin = NativeAndEVM::Native(WHITELISTED_D1_NATIVE.into()).encode();
    }: _(RawOrigin::Signed(caller.clone()), 0, call)
    verify {
        assert_last_event::<T>(
            Event::<T>::ProxyCall {
                origin: Decode::decode(&mut &origin[..]).unwrap(),
                result: Ok(()),
            }.into()
        );
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::ExternalityBuilder::build(),
        crate::mock::TestRuntime,
    );
}
