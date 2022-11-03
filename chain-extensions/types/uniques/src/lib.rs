#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use sp_runtime::{DispatchError, ModuleError};

pub enum UniquesFunc {
    // getters
    // NextCollectionId,
    CollectionDetails,

    // extrinsics
    Create,
    Mint,
    Transfer,
    ApproveTransfer,
    CancelApproval,
}

impl TryFrom<u32> for UniquesFunc {
    type Error = DispatchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        return match value {
            // getters
            // 0x0001 => Ok(UniquesFunc::NextCollectionId),
            0x0002 => Ok(UniquesFunc::CollectionDetails),

            // extrinsics
            0x00A0 => Ok(UniquesFunc::Create),
            0x00A1 => Ok(UniquesFunc::Mint),
            0x00A2 => Ok(UniquesFunc::Transfer),
            0x00A3 => Ok(UniquesFunc::ApproveTransfer),
            0x00A4 => Ok(UniquesFunc::CancelApproval),

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
            Some("AlreadyExists") => Ok(UniquesError::AlreadyExists),
            Some("WrongOwner") => Ok(UniquesError::WrongOwner),
            Some("BadWitness") => Ok(UniquesError::UnknownCollection),
            Some("InUse") => Ok(UniquesError::InUse),
            Some("Frozen") => Ok(UniquesError::Frozen),
            Some("WrongDelegate") => Ok(UniquesError::WrongDelegate),
            Some("NoDelegate") => Ok(UniquesError::NoDelegate),
            Some("Unapproved") => Ok(UniquesError::Unapproved),
            Some("Unaccepted") => Ok(UniquesError::Unaccepted),
            Some("Locked") => Ok(UniquesError::Locked),
            Some("MaxSupplyReached") => Ok(UniquesError::MaxSupplyReached),
            Some("MaxSupplyAlreadySet") => Ok(UniquesError::MaxSupplyAlreadySet),
            Some("MaxSupplyTooSmall") => Ok(UniquesError::MaxSupplyTooSmall),
            Some("NextIdNotUsed") => Ok(UniquesError::NextIdNotUsed),
            Some("NotForSale") => Ok(UniquesError::NotForSale),
            Some("BidTooLow") => Ok(UniquesError::BidTooLow),
            Some("UnImplemented") => Ok(UniquesError::UnImplemented),
            _ => Ok(UniquesError::UnknownError),
        };
    }
}
