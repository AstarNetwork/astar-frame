#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use frame_support::pallet_prelude::MaxEncodedLen;
use sp_core::H160;
use sp_runtime::{DispatchError, ModuleError};

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
        return match error_text {
            Some("Disabled") => Ok(DSError::Disabled),
            Some("NoMaintenanceModeChange") => Ok(DSError::NoMaintenanceModeChange),
            Some("UpgradeTooHeavy") => Ok(DSError::UpgradeTooHeavy),
            Some("StakingWithNoValue") => Ok(DSError::StakingWithNoValue),
            Some("InsufficientValue") => Ok(DSError::InsufficientValue),
            Some("MaxNumberOfStakersExceeded") => Ok(DSError::MaxNumberOfStakersExceeded),
            Some("NotOperatedContract") => Ok(DSError::NotOperatedContract),
            Some("NotStakedContract") => Ok(DSError::NotStakedContract),
            Some("NotUnregisteredContract") => Ok(DSError::NotUnregisteredContract),
            Some("UnclaimedRewardsRemaining") => Ok(DSError::UnclaimedRewardsRemaining),
            Some("UnstakingWithNoValue") => Ok(DSError::UnstakingWithNoValue),
            Some("NothingToWithdraw") => Ok(DSError::NothingToWithdraw),
            Some("AlreadyRegisteredContract") => Ok(DSError::AlreadyRegisteredContract),
            Some("ContractIsNotValid") => Ok(DSError::ContractIsNotValid),
            Some("AlreadyUsedDeveloperAccount") => Ok(DSError::AlreadyUsedDeveloperAccount),
            Some("NotOwnedContract") => Ok(DSError::NotOwnedContract),
            Some("UnknownEraReward") => Ok(DSError::UnknownEraReward),
            Some("UnexpectedStakeInfoEra") => Ok(DSError::UnexpectedStakeInfoEra),
            Some("TooManyUnlockingChunks") => Ok(DSError::TooManyUnlockingChunks),
            Some("AlreadyClaimedInThisEra") => Ok(DSError::AlreadyClaimedInThisEra),
            Some("EraOutOfBounds") => Ok(DSError::EraOutOfBounds),
            Some("TooManyEraStakeValues") => Ok(DSError::TooManyEraStakeValues),
            Some("RequiredContractPreApproval") => Ok(DSError::RequiredContractPreApproval),
            Some("AlreadyPreApprovedDeveloper") => Ok(DSError::AlreadyPreApprovedDeveloper),
            Some("NotActiveStaker") => Ok(DSError::NotActiveStaker),
            Some("NominationTransferToSameContract") => {
                Ok(DSError::NominationTransferToSameContract)
            }
            _ => Err(DispatchError::Other("DappsStakingExtension: Unknown error")),
        };
    }
}

/// This is only used to encode SmartContract enum
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum Contract<Account> {
    // EVM smart contract instance.
    Evm(H160),
    // Wasm smart contract instance. Not used in this precompile
    Wasm(Account),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingValueInput<Balance> {
    pub contract: [u8; 32],
    pub value: Balance,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingAccountInput {
    pub contract: [u8; 32],
    pub staker: [u8; 32],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen)]
pub struct DappsStakingEraInput {
    pub contract: [u8; 32],
    pub era: u32,
}
