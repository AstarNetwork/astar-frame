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

use super::{pallet::Error, Event, *};
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
pub fn is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        let call: RuntimeCall = pallet_balances::Call::transfer {
            dest: BOB,
            value: 10,
        }
        .into();

        assert_ok!(Account::call_as(
            RuntimeOrigin::signed(ALICE).into(),
            SimpleSalt(1),
            Box::new(call),
        ));
    })
}
