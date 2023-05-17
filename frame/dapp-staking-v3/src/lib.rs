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

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, LockableCurrency, ReservableCurrency, StorageVersion},
    weights::Weight,
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

/// TODO: change/improve name
pub type DAppId = u16;

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

/// General information about dApp.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct DAppInfo<AccountId, Balance> {
    /// Owner of the dApp, default reward beneficiary.
    pub owner: AccountId,
    /// dApp's unique identifier in dApp staking.
    pub id: u16,
    /// Current state of the dApp.
    pub state: DAppState,
    /// Reserved amount during registration of the dApp.
    /// Sort of a rent fee for all the storage items required to have the dApp registered.
    pub reserved: Balance,
    // If `None`, rewards goes to the developer account, otherwise to the account Id in `Some`.
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

        /// Describes smart contract in the context required by dApp staking.
        type SmartContract: Parameter + Member + MaxEncodedLen;

        /// Privileged origin for managing dApp staking pallet.
        type ManagerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Maximum number of contracts that can be integrated into dApp staking at once.
        /// TODO: maybe this can be reworded or improved later on - but we want a ceiling!
        type MaxNumberOfContracts: Get<DAppId>;

        /// Deposit reserved for registering a new smart contract. It will be reserved from the developers's account.
        #[pallet::constant]
        type RegistrationDeposit: Get<BalanceOf<Self>>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A smart contract has been registered for dApp staking
        ContractRegistered {
            owner: T::AccountId,
            smart_contract: T::SmartContract,
            dapp_id: DAppId,
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
        /// Developer account balance insufficent to pay for the dApp registration deposit.
        InsufficientOwnerBalance,
        /// Specified smart contract does not exist in dApp staking.
        ContractNotFound,
        /// Call origin is not dApp owner.
        OriginNotDAppOwner,
    }

    /// General information about dApp staking protocol state.
    #[pallet::storage]
    pub type ActiveProtocolState<T: Config> = StorageValue<_, ProtocolState, ValueQuery>;

    /// Counter for unique dApp identifiers.
    #[pallet::storage]
    pub type NextDAppId<T: Config> = StorageValue<_, DAppId, ValueQuery>;

    /// Map of all dApps integrated into dApp staking protocol.
    #[pallet::storage]
    pub type IntegratedDApps<T: Config> = CountedStorageMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        DAppInfo<T::AccountId, BalanceOf<T>>,
        OptionQuery,
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used to register contract for dApp staking, with the owner account as the owner.
        ///
        /// If successful, smart contract will be assigned a simple, unique numerical identifier.
        /// Requires register deposit to be paid by the owner account.
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
                !IntegratedDApps::<T>::count() < T::MaxNumberOfContracts::get().into(),
                Error::<T>::ExcededMaxNumberOfContracts
            );

            let dapp_id = NextDAppId::<T>::get();
            // MAX value must never be assigned as a dApp Id since it serves as a sentinel value.
            ensure!(dapp_id < DAppId::MAX, Error::<T>::NewDAppIdUnavailable);

            T::Currency::reserve(&owner, T::RegistrationDeposit::get())
                .map_err(|_| Error::<T>::InsufficientOwnerBalance)?;

            IntegratedDApps::<T>::insert(
                &smart_contract,
                DAppInfo {
                    owner: owner.clone(),
                    id: dapp_id,
                    state: DAppState::Registered,
                    reserved: T::RegistrationDeposit::get(),
                    reward_destination: None,
                },
            );

            NextDAppId::<T>::put(dapp_id.saturating_add(1));

            Self::deposit_event(Event::<T>::ContractRegistered {
                owner,
                smart_contract,
                dapp_id,
            });

            Ok(().into())
        }

        /// Used to modify the reward destination account for a dApp.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::zero())]
        pub fn set_dapp_reward_destination(
            origin: OriginFor<T>,
            smart_contract: T::SmartContract,
            beneficiary: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            Self::ensure_pallet_enabled()?;
            let dev_account = ensure_signed(origin)?;

            IntegratedDApps::<T>::try_mutate(
                smart_contract,
                |maybe_dapp_info| -> DispatchResult {
                    let dapp_info = maybe_dapp_info
                        .as_mut()
                        .ok_or(Error::<T>::ContractNotFound)?;

                    ensure!(
                        dapp_info.owner == dev_account,
                        Error::<T>::OriginNotDAppOwner
                    );

                    dapp_info.reward_destination = Some(beneficiary);

                    Ok(().into())
                },
            )?;

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// `Err` if pallet disabled for maintenance, `Ok` otherwise
        pub fn ensure_pallet_enabled() -> Result<(), Error<T>> {
            if ActiveProtocolState::<T>::get().pallet_disabled {
                Err(Error::<T>::Disabled)
            } else {
                Ok(())
            }
        }
    }
}