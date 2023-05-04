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

#![cfg_attr(not(feature = "std"), no_std)]

use num_enum::{IntoPrimitive, TryFromPrimitive};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{RuntimeDebug, H160};
use xcm::{latest::Weight, prelude::*};

pub const XCM_EXTENSION_ID: u16 = 04;

#[repr(u16)]
#[derive(TryFromPrimitive, IntoPrimitive)]
pub enum Command {
    PrepareExecute = 0,
    Execute = 1,
    ValidateSend = 2,
    Send = 3,
    NewQuery = 4,
    TakeResponse = 5,
    PalletAccountId = 6,
}

/// Type of XCM Response Query
#[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
// #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum QueryType<AccountId> {
    // No callback, store the response for manual polling
    NoCallback,
    // Call Wasm contract's method on recieving response
    // It expects the contract method to have following signature
    //     -  (query_id: QueryId, responder: Multilocation, response: Response)
    WASMContractCallback {
        contract_id: AccountId,
        selector: [u8; 4],
    },
    // Call Evm contract's method on recieving response
    // It expects the contract method to have following signature
    //     -  (query_id: QueryId, responder: Multilocation, response: Response)
    EVMContractCallback {
        contract_id: H160,
        selector: [u8; 4],
    },
}

/// Query config
#[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo)]
// #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct QueryConfig<AccountId, BlockNumber> {
    // query type
    pub query_type: QueryType<AccountId>,
    // blocknumber after which query will be expire
    pub timeout: BlockNumber,
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ValidateSendInput {
    pub dest: VersionedMultiLocation,
    pub xcm: VersionedXcm<()>,
}

pub struct PreparedExecution<Call> {
    pub xcm: Xcm<Call>,
    pub weight: Weight,
}

pub struct ValidatedSend {
    pub dest: MultiLocation,
    pub xcm: Xcm<()>,
}

#[macro_export]
macro_rules! create_error_enum {
    ($vis:vis $type_name:ident) => {
        #[repr(u32)]
        #[derive(
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::marker::Copy,
            ::core::clone::Clone,
            // crate name mismatch, 'parity-scale-codec' is crate name but in ink! contract
            // it is usually renamed to `scale`
            Encode,
            Decode,
            ::core::fmt::Debug,
            ::num_enum::IntoPrimitive,
            ::num_enum::FromPrimitive,
        )]
        #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
        $vis enum $type_name {
            /// Success
            Success = 0,
            /// CE command not supported
            InvalidCommand = 1,
            /// The version of the Versioned value used is not able to be interpreted.
            BadVersion = 2,
            /// Origin not allow for registering queries
            InvalidOrigin = 3,
            /// Does not support the given query type
            NotSupported = 4,
            /// XCM execute preparation missing
            PreparationMissing = 5,
            /// Some of the XCM instructions failed to execute
            ExecutionFailed = 6,
            /// Failed to validate the XCM for sending
            SendValidateFailed = 7,
            /// Failed to send the XCM to destination
            SendFailed = 8,
            /// No response recieved for given query
            NoResponse = 9,
            /// Failed to weigh the XCM message
            CannotWeigh = 10,
            /// Unknown runtime error
            #[num_enum(default)]
            RuntimeError = 99,
        }
    };
}

create_error_enum!(pub XcmCeError);
