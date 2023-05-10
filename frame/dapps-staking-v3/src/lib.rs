// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// You should have received a copy of the PolyForm-Noncommercial license with this crate.
// If not, see <https://polyformproject.org/licenses/noncommercial/1.0.0//>.

//! # Dapps Staking v3 Pallet
//!
//! - [`Config`]
//!
//! ## Overview
//!
//! Pallet that implements dapps staking protocol.
//!
//! <>
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! <>
//!
//! ### Other
//!
//! <>
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, LockableCurrency, ReservableCurrency, StorageVersion},
};
use frame_system::pallet_prelude::*;

pub use pallet::*;

/// The balance type used by the currency system.
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// TODO: change/improve name
pub type EraNumber = u32;

/// TODO: change/improve name
pub type PeriodNumber = u32;

/// TODO: just a placeholder, associated type should be used
pub type BlockNumber = u64;

/// Distinct period types in dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PeriodType {
    /// Period during which the focus is on voting.
    /// Inner value is the era in which the voting period ends.
    VotingPeriod(EraNumber),
    /// Period during which dApps and stakers earn rewards.
    /// Inner value is the era in which the Build&Eearn period ends.
    BuildAndEarnPeriod(EraNumber),
}

/// Force types to speed up the next era, and even period.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum ForcingTypes {
    /// Force the next era to start.
    NewEra,
    /// Force the current period phase to end, and new one to start
    NewEraAndPeriodPhase,
}

/// General information & state of the dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct ProtocolState {
    /// Ongoing era number.
    pub era: EraNumber,
    /// Block number at which the next era should start.
    /// TODO: instead of abusing on-initialize and wasting block-space,
    /// I believe we should utilize `pallet-scheduler` to schedule the next era. Make an item for this.
    pub next_era_start: Option<BlockNumber>,
    /// Ongoing period number.
    pub period: PeriodNumber,
    /// Ongoing period type and when is it expected to end.
    pub period_type: PeriodType,
    /// `true` if pallet is in maintenance mode (disabled), `false` otherwise.
    /// TODO: provide some configurable barrier to handle this on the runtime level instead? Make an item for this?
    pub pallet_disabled: bool,
}

impl Default for ProtocolState {
    fn default() -> Self {
        Self {
            era: 0,
            next_era_start: None,
            period: 0,
            period_type: PeriodType::VotingPeriod(0),
            pallet_disabled: false,
        }
    }
}

/// dApp state in which some dApp is in.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum DAppState {
    /// dApp is registered and active.
    Registered,
    /// dApp has been unregistered in the contained era
    Unregistered(EraNumber),
}

/// TODO
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct DAppInfo<AccountId, Balance> {
    /// Owner of the dApp, default reward beneficiary.
    pub owner: AccountId,
    /// dApp's unique identifier in dApp staking.
    pub id: u16,
    /// Current state of the dApp.
    pub state: DAppState,
    // TODO: Should we get rid of this?
    /// Reserved amount during registration of the dApp.
    pub reserved: Balance,
    // If `None`, rewards goes to the owner, otherwise to the account Id
    pub reward_destination: Option<AccountId>,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used for staking.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
            + ReservableCurrency<Self::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        // TODO: Add events
    }

    #[pallet::error]
    pub enum Error<T> {
        // TODO: Add errors
    }

    /// <>
    #[pallet::storage]
    pub type ActiveProtocolState<T: Config> = StorageValue<_, ProtocolState, ValueQuery>;

    // <>
    // #[pallet::storage]
    // #[pallet::getter(fn registered_smart_contract)]
    // pub type RegisteredSmartContracts<T: Config> =
    //     StorageMap<_, Blake2_128Concat, T::SmartContract, DAppInfo<T::AccountId, BalanceOf<T>>, OptionQuery>;
}
