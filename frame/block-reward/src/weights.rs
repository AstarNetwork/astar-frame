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

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet-reward-distribution.
pub trait WeightInfo {
    fn set_configuration() -> Weight;
}

/// Weights for pallet-reward-distribution using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: RewardDistribution RewardDistributionConfigStorage (r:0 w:1)
	fn set_configuration() -> Weight {
		T::DbWeight::get().writes(1 as u64)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: RewardDistribution RewardDistributionConfigStorage (r:0 w:1)
	fn set_configuration() -> Weight {
		RocksDbWeight::get().writes(1 as u64)
	}
}
