#![cfg_attr(not(feature = "std"), no_std)]
use sp_runtime::{
    DispatchError, ModuleError,
};
use codec::{Decode, Encode};


#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum DSError {
    /// Disabled
    Disabled,
    /// No change in maintenance mode
    NoMaintenanceModeChange,
    /// Upgrade is too heavy, reduce the weight parameter.
    UpgradeTooHeavy,
    /// Can not stake with zero value.
    StakingWithNoValue,
    /// Can not stake with value less than minimum staking value
    InsufficientValue,
    /// Number of stakers per contract exceeded.
    MaxNumberOfStakersExceeded,
    /// Targets must be operated contracts
    NotOperatedContract,
    /// Contract isn't staked.
    NotStakedContract,
    /// Contract isn't unregistered.
    NotUnregisteredContract,
    /// Unclaimed rewards should be claimed before withdrawing stake.
    UnclaimedRewardsRemaining,
    /// Unstaking a contract with zero value
    UnstakingWithNoValue,
    /// There are no previously unbonded funds that can be unstaked and withdrawn.
    NothingToWithdraw,
    /// The contract is already registered by other account
    AlreadyRegisteredContract,
    /// User attempts to register with address which is not contract
    ContractIsNotValid,
    /// This account was already used to register contract
    AlreadyUsedDeveloperAccount,
    /// Smart contract not owned by the account id.
    NotOwnedContract,
    /// Report issue on github if this is ever emitted
    UnknownEraReward,
    /// Report issue on github if this is ever emitted
    UnexpectedStakeInfoEra,
    /// Contract has too many unlocking chunks. Withdraw the existing chunks if possible
    /// or wait for current chunks to complete unlocking process to withdraw them.
    TooManyUnlockingChunks,
    /// Contract already claimed in this era and reward is distributed
    AlreadyClaimedInThisEra,
    /// Era parameter is out of bounds
    EraOutOfBounds,
    /// Too many active `EraStake` values for (staker, contract) pairing.
    /// Claim existing rewards to fix this problem.
    TooManyEraStakeValues,
    /// To register a contract, pre-approval is needed for this address
    RequiredContractPreApproval,
    /// Developer's account is already part of pre-approved list
    AlreadyPreApprovedDeveloper,
    /// Account is not actively staking
    NotActiveStaker,
    /// Transfering nomination to the same contract
    NominationTransferToSameContract,
}

impl TryFrom<DispatchError> for DSError {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };
        match error_text {
            Some("Disabled") => return Ok(DSError::Disabled),
            Some("NoMaintenanceModeChange") => return Ok(DSError::NoMaintenanceModeChange),
            Some("UpgradeTooHeavy") => return Ok(DSError::UpgradeTooHeavy),
            Some("StakingWithNoValue") => return Ok(DSError::StakingWithNoValue),
            Some("InsufficientValue") => return Ok(DSError::InsufficientValue),
            Some("MaxNumberOfStakersExceeded") => return Ok(DSError::MaxNumberOfStakersExceeded),
            Some("NotOperatedContract") => return Ok(DSError::NotOperatedContract),
            Some("NotStakedContract") => return Ok(DSError::NotStakedContract),
            Some("NotUnregisteredContract") => return Ok(DSError::NotUnregisteredContract),
            Some("UnclaimedRewardsRemaining") => return Ok(DSError::UnclaimedRewardsRemaining),
            Some("UnstakingWithNoValue") => return Ok(DSError::UnstakingWithNoValue),
            Some("NothingToWithdraw") => return Ok(DSError::NothingToWithdraw),
            Some("AlreadyRegisteredContract") => return Ok(DSError::AlreadyRegisteredContract),
            Some("ContractIsNotValid") => return Ok(DSError::ContractIsNotValid),
            Some("AlreadyUsedDeveloperAccount") => return Ok(DSError::AlreadyUsedDeveloperAccount),
            Some("NotOwnedContract") => return Ok(DSError::NotOwnedContract),
            Some("UnknownEraReward") => return Ok(DSError::UnknownEraReward),
            Some("UnexpectedStakeInfoEra") => return Ok(DSError::UnexpectedStakeInfoEra),
            Some("TooManyUnlockingChunks") => return Ok(DSError::TooManyUnlockingChunks),
            Some("AlreadyClaimedInThisEra") => return Ok(DSError::AlreadyClaimedInThisEra),
            Some("EraOutOfBounds") => return Ok(DSError::EraOutOfBounds),
            Some("TooManyEraStakeValues") => return Ok(DSError::TooManyEraStakeValues),
            Some("RequiredContractPreApproval") => return Ok(DSError::RequiredContractPreApproval),
            Some("AlreadyPreApprovedDeveloper") => return Ok(DSError::AlreadyPreApprovedDeveloper),
            Some("NotActiveStaker") => return Ok(DSError::NotActiveStaker),
            Some("NominationTransferToSameContract") => {
                return Ok(DSError::NominationTransferToSameContract)
            }
            _ => return Err(DispatchError::Other("DappsStakingExtension: Unknown error")),
        }
    }
}
