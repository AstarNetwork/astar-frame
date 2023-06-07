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
use crate::test::testing_utils::*;

#[test]
fn register_is_ok() {
    ExtBuilder::build().execute_with(|| {
        initialize_first_block();

        assert_register(5, &MockSmartContract::Wasm(1));
        assert_register(7, &MockSmartContract::Wasm(2));
        assert_register(13, &MockSmartContract::Wasm(3));
    })
}
