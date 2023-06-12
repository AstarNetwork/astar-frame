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

//! # dApp Staking v3 Pallet
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

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, LockIdentifier, LockableCurrency, StorageVersion, WithdrawReasons},
    weights::Weight,
    BoundedVec,
};
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::traits::{AtLeast32BitUnsigned, BadOrigin, Saturating, Zero};

pub use pallet::*;

#[cfg(test)]
mod test;

const STAKING_ID: LockIdentifier = *b"dapstake";

// TODO: move all structs & types into a separate `types.rs` file, after receiving first review batch

/// The balance type used by the currency system.
pub type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Convenience type for `AccountLedger` usage.
pub type AccountLedgerFor<T> = AccountLedger<
    BalanceOf<T>,
    BlockNumberFor<T>,
    <T as Config>::MaxLockedChunks,
    <T as Config>::MaxUnlockingChunks,
>;

/// Era number type
pub type EraNumber = u32;
/// Period number type
pub type PeriodNumber = u32;
/// Dapp Id type
pub type DAppId = u16;

/// Distinct period types in dApp staking protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PeriodType {
    /// Period during which the focus is on voting.
    /// Inner value is the era in which the voting period ends.
    VotingPeriod(#[codec(compact)] EraNumber),
    /// Period during which dApps and stakers earn rewards.
    /// Inner value is the era in which the Build&Eearn period ends.
    BuildAndEarnPeriod(#[codec(compact)] EraNumber),
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
pub struct ProtocolState<BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen> {
    /// Ongoing era number.
    #[codec(compact)]
    pub era: EraNumber,
    /// Block number at which the next era should start.
    /// TODO: instead of abusing on-initialize and wasting block-space,
    /// I believe we should utilize `pallet-scheduler` to schedule the next era. Make an item for this.
    #[codec(compact)]
    pub next_era_start: BlockNumber,
    /// Ongoing period number.
    #[codec(compact)]
    pub period: PeriodNumber,
    /// Ongoing period type and when is it expected to end.
    pub period_type: PeriodType,
    /// `true` if pallet is in maintenance mode (disabled), `false` otherwise.
    /// TODO: provide some configurable barrier to handle this on the runtime level instead? Make an item for this?
    pub pallet_disabled: bool,
}

impl<BlockNumber> Default for ProtocolState<BlockNumber>
where
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
{
    fn default() -> Self {
        Self {
            era: 0,
            next_era_start: BlockNumber::from(1_u32),
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
    Unregistered(#[codec(compact)] EraNumber),
}

/// General information about dApp.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct DAppInfo<AccountId> {
    /// Owner of the dApp, default reward beneficiary.
    pub owner: AccountId,
    /// dApp's unique identifier in dApp staking.
    #[codec(compact)]
    pub id: DAppId,
    /// Current state of the dApp.
    pub state: DAppState,
    // If `None`, rewards goes to the developer account, otherwise to the account Id in `Some`.
    pub reward_destination: Option<AccountId>,
}

/// How much was locked in a specific era
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct LockedChunk<Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    #[codec(compact)]
    amount: Balance,
    #[codec(compact)]
    era: EraNumber,
}

impl<Balance> Default for LockedChunk<Balance>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
{
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            era: EraNumber::zero(),
        }
    }
}

// TODO: would users get better UX if we kept using eras? Using blocks is more precise though.
/// How much was unlocked in some block.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct UnlockingChunk<
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
> {
    #[codec(compact)]
    amount: Balance,
    #[codec(compact)]
    unlock_block: BlockNumber,
}

impl<Balance, BlockNumber> Default for UnlockingChunk<Balance, BlockNumber>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
{
    fn default() -> Self {
        Self {
            amount: Balance::zero(),
            unlock_block: BlockNumber::zero(),
        }
    }
}

/// General info about user's stakes
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[scale_info(skip_type_params(LockedLen, UnlockingLen))]
pub struct AccountLedger<
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
> {
    /// How much was staked in each era
    locked: BoundedVec<LockedChunk<Balance>, LockedLen>,
    /// How much started unlocking on a certain block
    unlocking: BoundedVec<UnlockingChunk<Balance, BlockNumber>, UnlockingLen>,
    //TODO, make this a compact struct!!!
    /// How much user had staked in some period
    // #[codec(compact)]
    staked: (Balance, PeriodNumber),
}

impl<Balance, BlockNumber, LockedLen, UnlockingLen> Default
    for AccountLedger<Balance, BlockNumber, LockedLen, UnlockingLen>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
{
    fn default() -> Self {
        Self {
            locked: BoundedVec::<LockedChunk<Balance>, LockedLen>::default(),
            unlocking: BoundedVec::<UnlockingChunk<Balance, BlockNumber>, UnlockingLen>::default(),
            staked: (Balance::zero(), 0),
        }
    }
}

impl<Balance, BlockNumber, LockedLen, UnlockingLen>
    AccountLedger<Balance, BlockNumber, LockedLen, UnlockingLen>
where
    Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy,
    BlockNumber: AtLeast32BitUnsigned + MaxEncodedLen,
    LockedLen: Get<u32>,
    UnlockingLen: Get<u32>,
{
    /// Empty if no locked/unlocking/staked info exists.
    pub fn is_empty(&self) -> bool {
        self.locked.is_empty() && self.unlocking.is_empty() && self.staked.0.is_zero()
    }

    /// Returns latest locked chunk if it exists, `None` otherwise
    pub fn latest_locked_chunk(&self) -> Option<&LockedChunk<Balance>> {
        self.locked.last()
    }

    /// Returns locked amount.
    /// If `zero`, means that associated account hasn't locked any funds.
    pub fn locked_amount(&self) -> Balance {
        self.latest_locked_chunk()
            .map_or(Balance::zero(), |locked| locked.amount)
    }

    /// Returns latest era in which locked amount was updated or zero in case no lock amount exists
    pub fn era(&self) -> EraNumber {
        self.latest_locked_chunk()
            .map_or(EraNumber::zero(), |locked| locked.era)
    }

    /// Adds the specified amount to the total locked amount, if possible.
    ///
    /// If entry for the specified era already exists, it's updated.
    ///
    /// If entry for the specified era doesn't exist, it's created and insertion is attempted.
    /// In case vector has no more capacity, error is returned, and whole operation is a noop.
    pub fn add_lock_amount(&mut self, amount: Balance, era: EraNumber) -> Result<(), ()> {
        if amount.is_zero() {
            return Ok(());
        }

        let mut locked_chunk = if let Some(&locked_chunk) = self.locked.last() {
            locked_chunk
        } else {
            LockedChunk::default()
        };

        locked_chunk.amount.saturating_accrue(amount);

        if locked_chunk.era == era && !self.locked.is_empty() {
            if let Some(last) = self.locked.last_mut() {
                *last = locked_chunk;
            }
        } else {
            locked_chunk.era = era;
            self.locked.try_push(locked_chunk).map_err(|_| ())?;
        }

        Ok(())
    }
}

/// Rewards pool for lock participants  & dApps
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct RewardInfo<Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    /// Rewards pool for accounts which have locked funds in dApp staking
    #[codec(compact)]
    participants: Balance,
    /// Reward pool for dApps
    #[codec(compact)]
    dapps: Balance,
}

/// Info about current era, including the rewards, how much is locked, unlocking, etc.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo, Default)]
pub struct EraInfo<Balance: AtLeast32BitUnsigned + MaxEncodedLen + Copy> {
    /// Info about era rewards
    rewards: RewardInfo<Balance>,
    /// How much balance is considered to be locked in the current era.
    /// This value influences the reward distribution.
    #[codec(compact)]
    active_era_locked: Balance,
    /// How much balance is locked in dApp staking, in total.
    /// For rewards, this amount isn't relevant for the current era, but only from the next one.
    #[codec(compact)]
    total_locked: Balance,
    /// How much balance is undergoing unlocking process (still counts into locked amount)
    #[codec(compact)]
    unlocking: Balance,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The current storage version.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used for staking.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

        /// Describes smart contract in the context required by dApp staking.
        type SmartContract: Parameter + Member + MaxEncodedLen;

        /// Privileged origin for managing dApp staking pallet.
        type ManagerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Maximum number of contracts that can be integrated into dApp staking at once.
        /// TODO: maybe this can be reworded or improved later on - but we want a ceiling!
        #[pallet::constant]
        type MaxNumberOfContracts: Get<DAppId>;

        /// Maximum number of locked chunks that can exist per account at a time.
        #[pallet::constant]
        type MaxLockedChunks: Get<u32>;

        /// Maximum number of unlocking chunks that can exist per account at a time.
        #[pallet::constant]
        type MaxUnlockingChunks: Get<u32>;

        /// Minimum amount an account has to lock in dApp staking in order to participate.
        #[pallet::constant]
        type MinimumLockedAmount: Get<BalanceOf<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A smart contract has been registered for dApp staking
        DAppRegistered {
            owner: T::AccountId,
            smart_contract: T::SmartContract,
            dapp_id: DAppId,
        },
        /// dApp reward destination has been updated.
        DAppRewardDestination {
            smart_contract: T::SmartContract,
            beneficiary: Option<T::AccountId>,
        },
        /// dApp owner has been changed.
        DAppOwnerChanged {
            smart_contract: T::SmartContract,
            new_owner: T::AccountId,
        },
        /// dApp has been unregistered
        DAppUnregistered {
            smart_contract: T::SmartContract,
            era: EraNumber,
        },
        /// Account has locked some amount into dApp staking.
        Locked {
            account: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Pallet is disabled/in maintenance mode.
        Disabled,
        /// Smart contract already exists within dApp staking protocol.
        ContractAlreadyExists,
        /// Maximum number of smart contracts has been reached.
        ExcededMaxNumberOfContracts,
        /// Not possible to assign a new dApp Id.
        /// This should never happen since current type can support up to 65536 - 1 unique dApps.
        NewDAppIdUnavailable,
        /// Specified smart contract does not exist in dApp staking.
        ContractNotFound,
        /// Call origin is not dApp owner.
        OriginNotOwner,
        /// dApp is part of dApp staking but isn't active anymore.
        NotOperatedDApp,
        /// Performing locking or staking with 0 amount.
        ZeroAmount,
        /// Total locked amount for staker is below minimum threshold.
        LockedAmountBelowThreshold,
        /// Cannot add additional locked balance chunks due to size limit.
        TooManyLockedBalanceChunks,
    }

    /// General information about dApp staking protocol state.
    #[pallet::storage]
    pub type ActiveProtocolState<T: Config> =
        StorageValue<_, ProtocolState<BlockNumberFor<T>>, ValueQuery>;

    /// Counter for unique dApp identifiers.
    #[pallet::storage]
    pub type NextDAppId<T: Config> = StorageValue<_, DAppId, ValueQuery>;

    /// Map of all dApps integrated into dApp staking protocol.
    #[pallet::storage]
    pub type IntegratedDApps<T: Config> = CountedStorageMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        DAppInfo<T::AccountId>,
        OptionQuery,
    >;

    /// General locked/staked information for each account.
    #[pallet::storage]
    pub type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountLedgerFor<T>, ValueQuery>;

    /// General information about the current era.
    #[pallet::storage]
    pub type CurrentEraInfo<T: Config> = StorageValue<_, EraInfo<BalanceOf<T>>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used to register a new contract for dApp staking.
        ///
        /// If successful, smart contract will be assigned a simple, unique numerical identifier.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::zero())]
        pub fn register(
            origin: OriginFor<T>,
            owner: T::AccountId,
            smart_contract: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            T::ManagerOrigin::ensure_origin(origin)?;

            ensure!(
                !IntegratedDApps::<T>::contains_key(&smart_contract),
                Error::<T>::ContractAlreadyExists,
            );

            ensure!(
                IntegratedDApps::<T>::count() < T::MaxNumberOfContracts::get().into(),
                Error::<T>::ExcededMaxNumberOfContracts
            );

            let dapp_id = NextDAppId::<T>::get();
            // MAX value must never be assigned as a dApp Id since it serves as a sentinel value.
            ensure!(dapp_id < DAppId::MAX, Error::<T>::NewDAppIdUnavailable);

            IntegratedDApps::<T>::insert(
                &smart_contract,
                DAppInfo {
                    owner: owner.clone(),
                    id: dapp_id,
                    state: DAppState::Registered,
                    reward_destination: None,
                },
            );

            NextDAppId::<T>::put(dapp_id.saturating_add(1));

            Self::deposit_event(Event::<T>::DAppRegistered {
                owner,
                smart_contract,
                dapp_id,
            });

            Ok(().into())
        }

        /// Used to modify the reward destination account for a dApp.
        ///
        /// Caller has to be dApp owner.
        /// If set to `None`, rewards will be deposited to the dApp owner.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::zero())]
        pub fn set_dapp_reward_destination(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            beneficiary: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let dev_account = ensure_signed(origin)?;

            IntegratedDApps::<T>::try_mutate(
                &smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    ensure!(dapp_info.owner == dev_account, Error::<T>::OriginNotOwner);

                    dapp_info.reward_destination = beneficiary.clone();

                    Ok(().into())
                },
            )?;

            Self::deposit_event(Event::<T>::DAppRewardDestination {
                smart_contract,
                beneficiary,
            });

            Ok(().into())
        }

        /// Used to change dApp owner.
        ///
        /// Can be called by dApp owner or dApp staking manager origin.
        /// This is useful in two cases:
        /// 1. when the dApp owner account is compromised, manager can change the owner to a new account
        /// 2. if project wants to transfer ownership to a new account (DAO, multisig, etc.).
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::zero())]
        pub fn set_dapp_owner(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            new_owner: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let origin = Self::ensure_signed_or_manager(origin)?;

            IntegratedDApps::<T>::try_mutate(
                &smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    // If manager origin, `None`, no need to check if caller is the owner.
                    if let Some(caller) = origin {
                        ensure!(dapp_info.owner == caller, Error::<T>::OriginNotOwner);
                    }

                    dapp_info.owner = new_owner.clone();

                    Ok(().into())
                },
            )?;

            Self::deposit_event(Event::<T>::DAppOwnerChanged {
                smart_contract,
                new_owner,
            });

            Ok(().into())
        }

        /// Unregister dApp from dApp staking protocol, making it ineligible for future rewards.
        /// This doesn't remove the dApp completely from the system just yet, but it can no longer be used for staking.
        ///
        /// Can be called by dApp owner or dApp staking manager origin.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::zero())]
        pub fn unregister(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            T::ManagerOrigin::ensure_origin(origin)?;

            let current_era = ActiveProtocolState::<T>::get().era;

            IntegratedDApps::<T>::try_mutate(
                &smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    ensure!(
                        dapp_info.state == DAppState::Registered,
                        Error::<T>::NotOperatedDApp
                    );

                    dapp_info.state = DAppState::Unregistered(current_era);

                    Ok(().into())
                },
            )?;

            // TODO: might require some modification later on, like additional checks to ensure contract can be unregistered.

            Self::deposit_event(Event::<T>::DAppUnregistered {
                smart_contract,
                era: current_era,
            });

            Ok(().into())
        }

        /// Locks additional funds into dApp staking.
        ///
        /// In case caller account doesn't have sufficient balance to cover the specified amount, everything is locked.
        /// After adjustment, lock amount must be greater than zero and in total must be equal or greater than the minimum locked amount.
        ///
        /// It is possible for call to fail due to caller account already having too many locked balance chunks in storage. To solve this,
        /// caller should claim pending rewards, before retrying to lock additional funds.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::zero())]
        pub fn lock(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let account = ensure_signed(origin)?;

            let state = ActiveProtocolState::<T>::get();
            let mut ledger = Ledger::<T>::get(&account);

            // Calculate & check amount available for locking
            let available_balance =
                T::Currency::free_balance(&account).saturating_sub(ledger.locked_amount());
            let amount_to_lock = available_balance.min(amount);
            ensure!(!amount_to_lock.is_zero(), Error::<T>::ZeroAmount);

            // Only lock for the next era onwards.
            let lock_era = state.era.saturating_add(1);
            ledger
                .add_lock_amount(amount_to_lock, lock_era)
                .map_err(|_| Error::<T>::TooManyLockedBalanceChunks)?;
            ensure!(
                ledger.locked_amount() >= T::MinimumLockedAmount::get(),
                Error::<T>::LockedAmountBelowThreshold
            );

            Self::update_ledger(&account, ledger);
            CurrentEraInfo::<T>::mutate(|era_info| {
                era_info.total_locked.saturating_accrue(amount_to_lock);
            });

            Self::deposit_event(Event::<T>::Locked {
                account,
                amount: amount_to_lock,
            });

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// `Err` if pallet disabled for maintenance, `Ok` otherwise.
        pub(crate) fn ensure_pallet_enabled() -> Result<(), Error<T>> {
            if ActiveProtocolState::<T>::get().pallet_disabled {
                Err(Error::<T>::Disabled)
            } else {
                Ok(())
            }
        }

        /// Ensure that the origin is either the `ManagerOrigin` or a signed origin.
        ///
        /// In case of manager, `Ok(None)` is returned, and if signed origin `Ok(Some(AccountId))` is returned.
        pub(crate) fn ensure_signed_or_manager(
            origin: T::RuntimeOrigin,
        ) -> Result<Option<T::AccountId>, BadOrigin> {
            if T::ManagerOrigin::ensure_origin(origin.clone()).is_ok() {
                return Ok(None);
            }
            let who = ensure_signed(origin)?;
            Ok(Some(who))
        }

        /// Update the account ledger, and dApp staking balance lock.
        ///
        /// In case account ledger is empty, entries from the DB are removed and lock is released.
        pub(crate) fn update_ledger(account: &T::AccountId, ledger: AccountLedgerFor<T>) {
            if ledger.is_empty() {
                Ledger::<T>::remove(&account);
                T::Currency::remove_lock(STAKING_ID, account);
            } else {
                T::Currency::set_lock(
                    STAKING_ID,
                    account,
                    ledger.locked_amount(),
                    WithdrawReasons::all(),
                );
                Ledger::<T>::insert(account, ledger);
            }
        }
    }
}
