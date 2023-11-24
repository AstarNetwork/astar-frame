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

use crate::{weights::{WeightInfo, SubstrateWeight}, Config};
use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_xcm_transactor chain extension.
pub trait CEWeightInfo {
    fn account_id() -> Weight;
	fn prepare_execute(len: u32) -> Weight;
	fn execute() -> Weight;
	fn validate_send(len: u32) -> Weight;
	fn send() -> Weight;
	fn take_response() -> Weight;
	fn new_query() -> Weight;
    fn read_as_unbounded(n: u32) -> Weight;
}

/// Weights for pallet_xcm_transactor chain extension.
pub struct ChainExtensionWeight<T: Config>(PhantomData<T>);
impl<T: Config> CEWeightInfo for ChainExtensionWeight<T> {
    fn account_id() -> Weight {
        <T as Config>::WeightInfo::account_id()
    }

    fn prepare_execute(len: u32) -> Weight {
        <T as Config>::WeightInfo::prepare_execute().saturating_add(Self::read_as_unbounded(len))
    }

    fn execute() -> Weight {
        <T as Config>::WeightInfo::execute()
    }

    fn validate_send(len: u32) -> Weight {
        <T as Config>::WeightInfo::validate_send().saturating_add(Self::read_as_unbounded(len))
    }

    fn send() -> Weight {
        Weight::from_ref_time(100_000)
    }

    fn take_response() -> Weight {
        <T as Config>::WeightInfo::take_response()
    }

    fn new_query() -> Weight {
        <T as Config>::WeightInfo::new_query().saturating_add(<T as Config>::WeightInfo::on_callback_recieved())
    }

    fn read_as_unbounded(n: u32) -> Weight {
        Weight::from_ref_time(1_000).saturating_mul(n.into())
    }
}
