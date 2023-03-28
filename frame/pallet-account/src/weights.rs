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

use frame_support::weights::Weight;

pub trait WeightInfo {
    fn new_origin() -> Weight;
    fn proxy_call() -> Weight;
    fn meta_call() -> Weight;
}

impl WeightInfo for () {
    fn new_origin() -> Weight {
        Default::default()
    }
    fn proxy_call() -> Weight {
        Default::default()
    }
    fn meta_call() -> Weight {
        Default::default()
    }
}
