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
use parity_scale_codec::MaxEncodedLen;
use parity_scale_codec::{Decode, Encode};
use scale_info::prelude::vec::Vec;
use sp_runtime::{DispatchError, ModuleError};

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Outcome {
    /// Success
    Success = 0,
    /// Failed to schedule a call
    FailedToSchedule = 1,
    /// Cannot find the scheduled call.
    NotFound = 2,
    /// Given target block number is in the past.
    TargetBlockNumberInPast = 3,
    /// Reschedule failed because it does not change scheduled time.
    RescheduleNoChange = 4,
    /// Attempt to use a non-named function on a named task.
    Named = 5,
    /// Origin Caller is not supported
    OriginCannotBeCaller = 98,
    /// Unknown error
    RuntimeError = 99,
}

impl From<DispatchError> for Outcome {
    fn from(input: DispatchError) -> Self {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };
        return match error_text {
            Some("FailedToSchedule") => Outcome::FailedToSchedule,
            Some("NotFound") => Outcome::NotFound,
            Some("TargetBlockNumberInPast") => Outcome::TargetBlockNumberInPast,
            Some("RescheduleNoChange") => Outcome::RescheduleNoChange,
            Some("Named") => Outcome::Named,
            _ => Outcome::RuntimeError,
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub struct ContractCallInput<AccountId, Balance> {
    pub dest: AccountId,
    pub data: Vec<u8>,
    pub gas_limit: (u64, u64),
    pub storage_deposit_limit: Option<Balance>,
    pub value: Balance,
    pub max_weight: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Origin {
    Caller,
    Address,
}

impl Default for Origin {
    fn default() -> Self {
        Self::Address
    }
}

#[macro_export]
macro_rules! select_origin {
    ($origin:expr, $account:expr) => {
        match $origin {
            Origin::Caller => return Ok(RetVal::Converging(Outcome::OriginCannotBeCaller as u32)),
            Origin::Address => RawOrigin::Signed($account),
        }
    };
}
