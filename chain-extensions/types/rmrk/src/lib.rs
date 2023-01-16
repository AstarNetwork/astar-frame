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
use codec::{Decode, Encode};
use sp_runtime::{DispatchError, ModuleError};

pub enum RmrkFunc {
    // getters
    // NextNftId,
    CollectionIndex,
    // NextResourceId,
    Collections,
    Nfts,
    Priorities,
    Children,
    Resources,
    EquippableBases,
    EquippableSlots,
    Properties,
    Lock,

    // extrinsics
    MintNft,
    MintNftDirectlyToNft,
    CreateCollection,
    BurnNft,
    DestroyCollection,
    Send,
    AcceptNft,
    RejectNft,
    ChangeCollectionIssuer,
    SetProperty,
    LockCollection,
    AddBasicResource,
    AddComposableResource,
    AddSlotResource,
    AcceptResource,
    RemoveResource,
    AcceptResourceRemoval,
    SetPriority,
}

impl TryFrom<u16> for RmrkFunc {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        return match value {
            // getters
            // 0x0001 => Ok(RmrkFunc::NextNftId),
            0x0002 => Ok(RmrkFunc::CollectionIndex),
            // 0x0003 => Ok(RmrkFunc::NextResourceId),
            0x0004 => Ok(RmrkFunc::Collections),
            0x0005 => Ok(RmrkFunc::Nfts),
            0x0006 => Ok(RmrkFunc::Priorities),
            0x0007 => Ok(RmrkFunc::Children),
            0x0008 => Ok(RmrkFunc::Resources),
            0x0009 => Ok(RmrkFunc::EquippableBases),
            0x000A => Ok(RmrkFunc::EquippableSlots),
            0x000B => Ok(RmrkFunc::Properties),
            0x000C => Ok(RmrkFunc::Lock),

            // extrinsics
            0x000D => Ok(RmrkFunc::MintNft),
            0x000E => Ok(RmrkFunc::MintNftDirectlyToNft),
            0x000F => Ok(RmrkFunc::CreateCollection),
            0x0010 => Ok(RmrkFunc::BurnNft),
            0x0011 => Ok(RmrkFunc::DestroyCollection),
            0x0012 => Ok(RmrkFunc::Send),
            0x0013 => Ok(RmrkFunc::AcceptNft),
            0x0014 => Ok(RmrkFunc::RejectNft),
            0x0015 => Ok(RmrkFunc::ChangeCollectionIssuer),
            0x0016 => Ok(RmrkFunc::SetProperty),
            0x0017 => Ok(RmrkFunc::LockCollection),
            0x0018 => Ok(RmrkFunc::AddBasicResource),
            0x0019 => Ok(RmrkFunc::AddComposableResource),
            0x001A => Ok(RmrkFunc::AddSlotResource),
            0x001B => Ok(RmrkFunc::AcceptResource),
            0x001C => Ok(RmrkFunc::RemoveResource),
            0x001D => Ok(RmrkFunc::AcceptResourceRemoval),
            0x001E => Ok(RmrkFunc::SetPriority),
            _ => Err(DispatchError::Other("RmrkExtension: Unimplemented func_id")),
        };
    }
}

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum RmrkError {
    /// Error names should be descriptive.
    Success,
    /// Errors should have helpful documentation associated with them.
    StorageOverflow,
    TooLong,
    NoAvailableCollectionId,
    NoAvailableResourceId,
    MetadataNotSet,
    RecipientNotSet,
    NoAvailableNftId,
    NotInRange,
    RoyaltyNotSet,
    CollectionUnknown,
    NoPermission,
    NoWitness,
    CollectionNotEmpty,
    CollectionFullOrLocked,
    CannotSendToDescendentOrSelf,
    ResourceAlreadyExists,
    NftAlreadyExists,
    EmptyResource,
    TooManyRecursions,
    NftIsLocked,
    CannotAcceptNonOwnedNft,
    CannotRejectNonOwnedNft,
    CannotRejectNonPendingNft,
    ResourceDoesntExist,
    /// Accepting a resource that is not pending should fail
    ResourceNotPending,
    NonTransferable,
    // Must unequip an item before sending (this only applies to the
    // rmrk-equip pallet but the send operation lives in rmrk-core)
    CannotSendEquippedItem,
}

impl TryFrom<DispatchError> for RmrkError {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };
        match error_text {
            Some("NoneValue") => Ok(RmrkError::Success),
            Some("StorageOverflow") => Ok(RmrkError::StorageOverflow),
            Some("TooLong") => Ok(RmrkError::TooLong),
            Some("NoAvailableCollectionId") => Ok(RmrkError::NoAvailableCollectionId),
            Some("NoAvailableResourceId") => Ok(RmrkError::NoAvailableResourceId),
            Some("MetadataNotSet") => Ok(RmrkError::MetadataNotSet),
            Some("RecipientNotSet") => Ok(RmrkError::RecipientNotSet),
            Some("NoAvailableNftId") => Ok(RmrkError::NoAvailableNftId),
            Some("NotInRange") => Ok(RmrkError::NotInRange),
            Some("RoyaltyNotSet") => Ok(RmrkError::RoyaltyNotSet),
            Some("CollectionUnknown") => Ok(RmrkError::CollectionUnknown),
            Some("NoPermission") => Ok(RmrkError::NoPermission),
            Some("NoWitness") => Ok(RmrkError::NoWitness),
            Some("CollectionNotEmpty") => Ok(RmrkError::CollectionNotEmpty),
            Some("CollectionFullOrLocked") => Ok(RmrkError::CollectionFullOrLocked),
            Some("CannotSendToDescendentOrSelf") => Ok(RmrkError::CannotSendToDescendentOrSelf),
            Some("ResourceAlreadyExists") => Ok(RmrkError::ResourceAlreadyExists),
            Some("NftAlreadyExists") => Ok(RmrkError::NftAlreadyExists),
            Some("EmptyResource") => Ok(RmrkError::EmptyResource),
            Some("TooManyRecursions") => Ok(RmrkError::TooManyRecursions),
            Some("NftIsLocked") => Ok(RmrkError::NftIsLocked),
            Some("CannotAcceptNonOwnedNft") => Ok(RmrkError::CannotAcceptNonOwnedNft),
            Some("CannotRejectNonOwnedNft") => Ok(RmrkError::CannotRejectNonOwnedNft),
            Some("CannotRejectNonPendingNft") => Ok(RmrkError::CannotRejectNonPendingNft),
            Some("ResourceDoesntExist") => Ok(RmrkError::ResourceDoesntExist),
            Some("ResourceNotPending") => Ok(RmrkError::ResourceNotPending),
            Some("NonTransferable") => Ok(RmrkError::NonTransferable),
            Some("CannotSendEquippedItem") => Ok(RmrkError::CannotSendEquippedItem),
            _ => Err(DispatchError::Other("RmrkExtension: Unknown error")),
        }
    }
}
