#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use sp_runtime::{DispatchError, ModuleError};

pub enum UniquesFunc {
    // getters
    CollectionDetails,

    // extrinsics
    Create,
    Mint,
    // SetCollectionMetadata,
    // SetItemMetadata,
    // SetCollectionMaxSupply,
    // Transfer,
}

impl TryFrom<u32> for UniquesFunc {
    type Error = DispatchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        return match value {
            // getters
            0x0002 => Ok(UniquesFunc::CollectionDetails),
            // 0x0003 => Ok(UniquesFunc::NextResourceId),
            // 0x0004 => Ok(UniquesFunc::Collections),
            // 0x0005 => Ok(UniquesFunc::Nfts),
            // 0x0006 => Ok(UniquesFunc::Priorities),
            // 0x0007 => Ok(UniquesFunc::Children),
            // 0x0008 => Ok(UniquesFunc::Resources),
            // 0x0009 => Ok(UniquesFunc::EquippableBases),
            // 0x000A => Ok(UniquesFunc::EquippableSlots),
            // 0x000B => Ok(UniquesFunc::Properties),

            // extrinsics
            0x00A0 => Ok(UniquesFunc::Create),
            0x00A1 => Ok(UniquesFunc::Mint),
            // 0x00A2 => Ok(UniquesFunc::SetCollectionMetadata),
            // 0x00A3 => Ok(UniquesFunc::SetItemMetadata),
            // 0x00A4 => Ok(UniquesFunc::SetCollectionMaxSupply),
            // 0x0012 => Ok(UniquesFunc::Send),
            // 0x0013 => Ok(UniquesFunc::AcceptNft),
            // 0x0014 => Ok(UniquesFunc::RejectNft),
            // 0x0015 => Ok(UniquesFunc::ChangeCollectionIssuer),
            // 0x0016 => Ok(UniquesFunc::SetProperty),
            // 0x0017 => Ok(UniquesFunc::LockCollection),
            // 0x0018 => Ok(UniquesFunc::AddBasicResource),
            // 0x0019 => Ok(UniquesFunc::AddComposableResource),
            // 0x001A => Ok(UniquesFunc::AddSlotResource),
            // 0x001B => Ok(UniquesFunc::AcceptResource),
            // 0x001C => Ok(UniquesFunc::RemoveResource),
            // 0x001D => Ok(UniquesFunc::AcceptResourceRemoval),
            // 0x001E => Ok(UniquesFunc::SetPriority),
            _ => Err(DispatchError::Other(
                "UniquesExtension: Unimplemented func_id",
            )),
        };
    }
}

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum UniquesError {
    /// Success
    Success = 0,
    /// The signing account has no permission to do the operation.
    NoPermission,
    /// The given item ID is unknown.
    UnknownCollection,
    /// The item ID has already been used for an item.
    AlreadyExists,
    /// The owner turned out to be different to what was expected.
    WrongOwner,
    /// Invalid witness data given.
    BadWitness,
    /// The item ID is already taken.
    InUse,
    /// The item or collection is frozen.
    Frozen,
    /// The delegate turned out to be different to what was expected.
    WrongDelegate,
    /// There is no delegate approved.
    NoDelegate,
    /// No approval exists that would allow the transfer.
    Unapproved,
    /// The named owner has not signed ownership of the collection is acceptable.
    Unaccepted,
    /// The item is locked.
    Locked,
    /// All items have been minted.
    MaxSupplyReached,
    /// The max supply has already been set.
    MaxSupplyAlreadySet,
    /// The provided max supply is less to the amount of items a collection already has.
    MaxSupplyTooSmall,
    /// The `CollectionId` in `NextCollectionId` is not being used.
    ///
    /// This means that you can directly proceed to call `create`.
    NextIdNotUsed,
    /// The given item ID is unknown.
    UnknownItem,
    /// Item is not for sale.
    NotForSale,
    /// The provided bid is too low.
    BidTooLow,
    /// Unknown error
    UnknownError = 99,
    UnImplemented = 100,
}

impl TryFrom<DispatchError> for UniquesError {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("Uniques-CE: No module error Info"),
        };
        return match error_text {
            Some("NoPermission") => Ok(UniquesError::NoPermission),
            Some("UnknownCollection") => Ok(UniquesError::UnknownCollection),
            _ => Ok(UniquesError::UnknownError),
        };
    }
}
