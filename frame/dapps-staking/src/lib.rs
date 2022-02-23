//! # dApps Staking Module
//!
//! The dApps staking module manages era, total amounts of rewards and how to distribute.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::traits::Currency;
use frame_system::{self as system};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{AtLeast32BitUnsigned, Zero},
    RuntimeDebug,
};
use sp_std::{ops::Add, prelude::*};

pub mod pallet;
pub mod traits;
pub mod weights;
pub use traits::*;

#[cfg(any(feature = "runtime-benchmarks"))]
pub mod benchmarking;
pub mod migrations;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod testing_utils;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_lib;

pub use pallet::pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// DApp State descriptor
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
enum DAppState {
    /// Contract is registered and active.
    Registered,
    /// Contract has been unregistered and is inactive.
    /// Claim for past eras and unbonding is still possible but no additional staking can be done.
    Unregistered(EraIndex),
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct DAppInfo<AccountId> {
    /// Developer (owner) account
    developer: AccountId,
    /// Current DApp State
    state: DAppState,
}

impl<AccountId> DAppInfo<AccountId> {
    /// Create new `DAppInfo` struct instance with the given developer and state `Registered`
    fn new(developer: AccountId) -> Self {
        Self {
            developer: developer,
            state: DAppState::Registered,
        }
    }
}

/// Mode of era-forcing.
#[derive(Copy, Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Forcing {
    /// Not forcing anything - just let whatever happen.
    NotForcing,
    /// Force a new era, then reset to `NotForcing` as soon as it is done.
    /// Note that this will force to trigger an election until a new era is triggered, if the
    /// election failed, the next session end will trigger a new election again, until success.
    ForceNew,
}

impl Default for Forcing {
    fn default() -> Self {
        Forcing::NotForcing
    }
}

/// A record of rewards allocated for stakers and dapps
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct RewardInfo<Balance: HasCompact> {
    /// Total amount of rewards for stakers in an era
    #[codec(compact)]
    pub stakers: Balance,
    /// Total amount of rewards for dapps in an era
    #[codec(compact)]
    pub dapps: Balance,
}

/// A record for total rewards and total amount staked for an era
#[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct EraInfo<Balance: HasCompact> {
    /// Total amount of earned rewards for an era
    pub rewards: RewardInfo<Balance>,
    /// Total staked amount in an era
    #[codec(compact)]
    pub staked: Balance,
    /// Total locked amount in an era
    #[codec(compact)]
    pub locked: Balance,
}

/// Used to split total EraPayout among contracts.
/// Each tuple (contract, era) has this structure.
/// This will be used to reward contracts developer and his stakers.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct EraStakingPoints<Balance: HasCompact> {
    /// Total staked amount.
    #[codec(compact)]
    pub total: Balance,
    /// Total number of active stakers
    #[codec(compact)]
    number_of_stakers: u32,
    /// Indicates whether rewards were claimed for this era or not
    contract_reward_claimed: bool,
}

/// Storage value representing the current Dapps staking pallet storage version.
/// Used by `on_runtime_upgrade` to determine whether a storage migration is needed or not.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum Version {
    V1_0_0,
    V2_0_0,
    V3_0_0,
}

impl Default for Version {
    fn default() -> Self {
        Version::V2_0_0
    }
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct EraStake<Balance: AtLeast32BitUnsigned + Copy> {
    /// Staked amount in era
    #[codec(compact)]
    staked: Balance,
    /// Staked era
    #[codec(compact)]
    era: EraIndex,
}

impl<Balance: AtLeast32BitUnsigned + Copy> EraStake<Balance> {
    fn new(staked: Balance, era: EraIndex) -> Self {
        Self { staked, era }
    }
}

/// Contains information about how much was staked in each era
#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct StakerInfo<Balance: AtLeast32BitUnsigned + Copy> {
    // Size of this list would be limited by a configurable constant
    stakes: Vec<EraStake<Balance>>,
}

impl<Balance: AtLeast32BitUnsigned + Copy> StakerInfo<Balance> {
    /// true if no stakes exist, false otherwise
    fn is_empty(&self) -> bool {
        self.stakes.is_empty()
    }

    /// number of `EraStake` chunks
    fn len(&self) -> u32 {
        self.stakes.len() as u32
    }

    /// Stakes some value in the specified era.
    /// User should ensure that given era is either equal or greater than the
    /// latest available era in the staking info.
    fn stake(&mut self, current_era: EraIndex, value: Balance) -> Result<(), &str> {
        if self.stakes.is_empty() {
            self.stakes.push(EraStake::new(value, current_era))
        } else {
            let era_stake = self.stakes.last().unwrap(); // exists if vec not empty
            if era_stake.era > current_era {
                return Err("Unexpected era".into());
            }

            let new_stake_value = era_stake.staked.saturating_add(value);

            if current_era == era_stake.era {
                *self.stakes.last_mut().unwrap() = EraStake::new(new_stake_value, current_era)
            } else {
                self.stakes
                    .push(EraStake::new(new_stake_value, current_era))
            }
        }
        Ok(())
    }

    /// Unstakes some value in the specified era.
    /// User should ensure that given era is either equal or greater than the
    /// latest available era in the staking info.
    fn unstake(&mut self, current_era: EraIndex, value: Balance) -> Result<(), &str> {
        if self.stakes.is_empty() {
            return Ok(());
        }

        let era_stake = self.stakes.last().unwrap(); // not empty so it exists
        if era_stake.era > current_era {
            return Err("Unexpected era".into());
        }

        let new_stake_value = era_stake.staked.saturating_sub(value);

        if current_era == era_stake.era {
            *self.stakes.last_mut().unwrap() = EraStake::new(new_stake_value, current_era)
        } else {
            self.stakes
                .push(EraStake::new(new_stake_value, current_era))
        }

        self.clean_unstaked();
        Ok(())
    }

    /// `Claims` the oldest era available for claiming.
    /// In case valid era exists, returns (claim era, staked amount) tuple.
    /// If no valid era exists, returns (0, 0) tuple.
    fn claim(&mut self) -> (EraIndex, Balance) {
        if self.is_empty() {
            (0, Zero::zero())
        } else {
            let era_stake = self.stakes[0]; // not empty so it exists

            if self.stakes.len() == 1 || self.stakes[1].era > era_stake.era + 1 {
                *self.stakes.first_mut().unwrap() = EraStake {
                    staked: era_stake.staked,
                    era: era_stake.era.saturating_add(1),
                }
            } else {
                // in case: self.stakes[1].era == era_stake.era + 1
                self.stakes.remove(0);
            }

            self.clean_unstaked();

            (era_stake.era, era_stake.staked)
        }
    }

    /// Latest staked value.
    /// E.g. if staker is fully unstaked, this will return `Zero`.
    /// Othwerise returns a non-zero balance.
    pub fn latest_staked_value(&self) -> Balance {
        self.stakes.last().map_or(Zero::zero(), |x| x.staked)
    }

    /// Removes unstaked values if they're no longer valid for comprehension
    fn clean_unstaked(&mut self) {
        if !self.stakes.is_empty() && self.stakes[0].staked.is_zero() {
            self.stakes.remove(0);
        }
    }

    /// Adjust era stakes information to the unregistered era.
    /// All information that exists from `unregistered_era` onwards will be cleared.
    /// This should only be used after fetching the claim era and stake value.
    fn unregistered_era_adjust(&mut self, unregistered_era: EraIndex) {
        if self.stakes.is_empty() {
            return;
        }

        if self.stakes[0].era >= unregistered_era {
            // In this scenario, all eras prior to unregistered era have already been claimed
            self.stakes.clear();
        } else {
            self.stakes.retain(|x| x.era < unregistered_era);
            self.stakes
                .push(EraStake::new(Zero::zero(), unregistered_era));
        }
    }
}

/// Represents an balance amount undergoing the unbonding process.
/// Since unbonding takes time, it's important to keep track of when and how much was unbonded.
#[derive(Clone, Copy, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct UnlockingChunk<Balance> {
    /// Amount being unlocked
    #[codec(compact)]
    amount: Balance,
    /// Era in which the amount will become unlocked and can be withdrawn.
    #[codec(compact)]
    unlock_era: EraIndex,
}

impl<Balance> UnlockingChunk<Balance>
where
    Balance: Add<Output = Balance> + Copy,
{
    // Adds the specified amount to this chunk
    fn add_amount(&mut self, amount: Balance) {
        self.amount = self.amount + amount
    }
}

/// Contains unlocking chunks.
/// This is a convenience struct that provides various utility methods to help with unbonding handling.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct UnbondingInfo<Balance: AtLeast32BitUnsigned + Default + Copy> {
    // Vector of unlocking chunks. Sorted in ascending order in respect to unlock_era.
    unlocking_chunks: Vec<UnlockingChunk<Balance>>,
}

impl<Balance> UnbondingInfo<Balance>
where
    Balance: AtLeast32BitUnsigned + Default + Copy,
{
    /// Returns total number of unlocking chunks.
    fn len(&self) -> u32 {
        self.unlocking_chunks.len() as u32
    }

    /// True if no unlocking chunks exist, false otherwise.
    fn is_empty(&self) -> bool {
        self.unlocking_chunks.is_empty()
    }

    /// Returns sum of all unlocking chunks.
    fn sum(&self) -> Balance {
        self.unlocking_chunks
            .iter()
            .map(|chunk| chunk.amount)
            .reduce(|c1, c2| c1 + c2)
            .unwrap_or_default()
    }

    /// Adds a new unlocking chunk to the vector, preserving the unlock_era based ordering.
    fn add(&mut self, chunk: UnlockingChunk<Balance>) {
        // It is possible that the unbonding period changes so we need to account for that
        match self
            .unlocking_chunks
            .binary_search_by(|x| x.unlock_era.cmp(&chunk.unlock_era))
        {
            // Merge with existing chunk if unlock_eras match
            Ok(pos) => self.unlocking_chunks[pos].add_amount(chunk.amount),
            // Otherwise insert where it should go. Note that this will in almost all cases return the last index.
            Err(pos) => self.unlocking_chunks.insert(pos, chunk),
        }
    }

    /// Partitions the unlocking chunks into two groups:
    ///
    /// First group includes all chunks which have unlock era lesser or equal to the specified era.
    /// Second group includes all the rest.
    ///
    /// Order of chunks is preserved in the two new structs.
    fn partition(self, era: EraIndex) -> (Self, Self) {
        let (matching_chunks, other_chunks): (
            Vec<UnlockingChunk<Balance>>,
            Vec<UnlockingChunk<Balance>>,
        ) = self
            .unlocking_chunks
            .iter()
            .partition(|chunk| chunk.unlock_era <= era);

        (
            Self {
                unlocking_chunks: matching_chunks,
            },
            Self {
                unlocking_chunks: other_chunks,
            },
        )
    }

    #[cfg(test)]
    /// Return clone of the internal vector. Should only be used for testing.
    fn vec(&self) -> Vec<UnlockingChunk<Balance>> {
        self.unlocking_chunks.clone()
    }
}

/// Instruction on how to handle reward payout for stakers.
/// In order to make staking more competitive, majority of stakers will want to
/// automatically restake anything they earn.
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RewardHandling {
    /// Rewards are transfered to stakers balance without any further action.
    OnlyPayout,
    /// Rewards are transfered to stakers balance and are immediately re-staked
    /// on the contract from which the reward was received.
    PayoutAndStake,
}

impl Default for RewardHandling {
    fn default() -> Self {
        RewardHandling::PayoutAndStake
    }
}

/// Contains information about account's locked & unbonding balances.
#[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
pub struct AccountLedger<Balance: AtLeast32BitUnsigned + Default + Copy> {
    /// Total balance locked.
    #[codec(compact)]
    pub locked: Balance,
    /// Information about unbonding chunks.
    unbonding_info: UnbondingInfo<Balance>,
    /// Instruction on how to handle reward payout
    reward_handling: RewardHandling,
}

impl<Balance: AtLeast32BitUnsigned + Default + Copy> AccountLedger<Balance> {
    /// `true` if ledger is empty (no locked funds, no unbonding chunks), `false` otherwise.
    pub(crate) fn is_empty(&self) -> bool {
        self.locked.is_zero() && self.unbonding_info.is_empty()
    }
}
