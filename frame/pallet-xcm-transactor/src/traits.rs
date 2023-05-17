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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use frame_support::weights::Weight;
use sp_runtime::DispatchError;
use xcm::prelude::*;
use xcm_ce_primitives::QueryType;

use crate::{Config, ResponseInfo};

/// Handle the incoming xcm notify callback from ResponseHandler (pallet_xcm)
pub trait OnCallback {
    /// error type, that can be converted to dispatch error
    type Error: Into<DispatchError>;
    /// account id type
    type AccountId;
    /// blocknumber type
    type BlockNumber;

    // TODO: Query type itself should be generic like
    //
    // type QueryType: Member + Parameter + MaybeSerializeDeserialize + MaxEncodedLen + Convert<Self, Weight>
    // type CallbackHandler: OnResponse<QueryType = T::QueryType>
    //
    // #[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen)]
    // enum MyQueryType {}
    //
    // impl Convert<Self, Weight> for MyQueryType {}

    /// Check whether query type is supported or not
    fn can_handle(query_type: &QueryType<Self::AccountId>) -> bool;

    /// handle the xcm response
    fn on_callback(
        responder: impl Into<MultiLocation>,
        response_info: ResponseInfo<Self::AccountId>,
    ) -> Result<Weight, Self::Error>;
}


/// OnCallback implementation that does not supports any callback
/// Use this to disable callbacks
pub struct NoCallback<T: Config>(PhantomData<T>);
impl<T: Config> OnCallback for NoCallback<T> {
    type Error = crate::Error<T>;
    type AccountId = T::AccountId;
    type BlockNumber = T::BlockNumber;

    fn can_handle(_: &QueryType<Self::AccountId>) -> bool {
        false
    }

    fn on_callback(
        _: impl Into<MultiLocation>,
        _: ResponseInfo<Self::AccountId>,
    ) -> Result<Weight, Self::Error> {
        Ok(Weight::zero())
    }
}
