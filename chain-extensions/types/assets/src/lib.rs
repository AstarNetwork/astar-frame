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

use scale_info::scale;
#[cfg(feature = "substrate")]
use sp_runtime::{DispatchError, ModuleError};

#[derive(PartialEq, Eq, Copy, Clone, scale::Encode, scale::Decode, Debug)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AssetsError {
    /// Success
    Success = 0,
    /// Error
    IsError = 1,
}

// #[pallet::error]
// pub enum Error<T, I = ()> {
//     /// Account balance must be greater than or equal to the transfer amount.
//     BalanceLow,
//     /// The account to alter does not exist.
//     NoAccount,
//     /// The signing account has no permission to do the operation.
//     NoPermission,
//     /// The given asset ID is unknown.
//     Unknown,
//     /// The origin account is frozen.
//     Frozen,
//     /// The asset ID is already taken.
//     InUse,
//     /// Invalid witness data given.
//     BadWitness,
//     /// Minimum balance should be non-zero.
//     MinBalanceZero,
//     /// Unable to increment the consumer reference counters on the account. Either no provider
//     /// reference exists to allow a non-zero balance of a non-self-sufficient asset, or the
//     /// maximum number of consumers has been reached.
//     NoProvider,
//     /// Invalid metadata given.
//     BadMetadata,
//     /// No approval exists that would allow the transfer.
//     Unapproved,
//     /// The source account would not survive the transfer and it needs to stay alive.
//     WouldDie,
//     /// The asset-account already exists.
//     AlreadyExists,
//     /// The asset-account doesn't have an associated deposit.
//     NoDeposit,
//     /// The operation would result in funds being burned.
//     WouldBurn,
// }

#[cfg(feature = "substrate")]
impl TryFrom<DispatchError> for AssetsError {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };
        return match error_text {
            Some("BalanceLow") => Ok(AssetsError::IsError),
            Some("NoAccount") => Ok(AssetsError::IsError),
            Some("NoPermission") => Ok(AssetsError::IsError),
            Some("Unknown") => Ok(AssetsError::IsError),
            Some("Frozen") => Ok(AssetsError::IsError),
            Some("InUse") => Ok(AssetsError::IsError),
            Some("BadWitness") => Ok(AssetsError::IsError),
            Some("MinBalanceZero") => Ok(AssetsError::IsError),
            Some("NoProvider") => Ok(AssetsError::IsError),
            Some("BadMetadata") => Ok(AssetsError::IsError),
            Some("Unapproved") => Ok(AssetsError::IsError),
            Some("WouldDie") => Ok(AssetsError::IsError),
            Some("AlreadyExists") => Ok(AssetsError::IsError),
            Some("NoDeposit") => Ok(AssetsError::IsError),
            Some("WouldBurn") => Ok(AssetsError::IsError),
            _ => Ok(AssetsError::IsError),
        };
    }
}

#[cfg(feature = "ink")]
impl ink_env::chain_extension::FromStatusCode for AssetsError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::IsError),
            _ => panic!("encountered unknown status code"),
        }
    }
}

#[cfg(feature = "ink")]
impl From<scale::Error> for AssetsError {
    fn from(_: scale::Error) -> Self {
        panic!("encountered unexpected invalid SCALE encoding")
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode, scale::MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[cfg_attr(
feature = "ink",
derive(ink_storage::traits::SpreadLayout, ink_storage::traits::PackedLayout,)
)]
#[cfg_attr(all(feature = "ink", feature = "std"), derive(ink_storage::traits::StorageLayout))]
pub enum Origin {
    Caller,
    Address,
}

impl Default for Origin {
    fn default() -> Self {
        Self::Address
    }
}

#[cfg(feature = "ink")]
impl ink_storage::traits::SpreadAllocate for Origin {
    fn allocate_spread(_ptr: &mut ink_primitives::KeyPtr) -> Self {
        Self::Address
    }
}