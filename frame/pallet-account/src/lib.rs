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

//! # Account abstraction pallet
//!
//! ## Overview
//!
//! An accout abstraction pallet makes it possible to derive new blockchain based
//! account for an existing external owned account (seed phrase based). The onchain
//! account could be derived to multiple address spaces: H160 and SS58. For example,
//! it makes possible to predictably interact between substrate native account and
//! EVM smart contracts.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! * new_origin() - create new origin for account
//! * proxy_call() - make proxy call with derived account as origin
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub mod origins;
pub use origins::*;

pub mod weights;
pub use weights::*;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use crate::*;

    use frame_support::pallet_prelude::*;
    use frame_support::{
        dispatch::{Dispatchable, GetDispatchInfo},
        traits::IsSubType,
    };
    use frame_system::pallet_prelude::*;

    /// The current storage version.                                                                                      
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Custom origin type.
        type CustomOrigin: Parameter + TryInto<Self::AccountId> + MaxEncodedLen;
        /// Parameter that defin different origin options and how to create it.
        type CustomOriginKind: Parameter + OriginDeriving<Self::AccountId, Self::CustomOrigin>;
        /// The runtime origin type.
        type RuntimeOrigin: From<Self::CustomOrigin>
            + From<frame_system::RawOrigin<Self::AccountId>>;
        /// The overarching call type.
        type RuntimeCall: Parameter
            + Dispatchable<RuntimeOrigin = <Self as Config>::RuntimeOrigin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsSubType<Call<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeCall>;
        /// General event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Origin with given index not registered.
        UnregisteredOrigin,
        /// Signature does not match Signer, check nonce, magic and try again.
        BadSignature,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        NewOrigin {
            account: T::AccountId,
            origin: T::CustomOrigin,
        },
        ProxyCall {
            origin: T::CustomOrigin,
            result: DispatchResult,
        },
    }

    #[pallet::origin]
    pub type Origin<T> = <T as Config>::CustomOrigin;

    /// Account origins
    #[pallet::storage]
    pub type AccountOrigin<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Twox64Concat, u32, T::CustomOrigin>;

    /// Account last origin index
    #[pallet::storage]
    pub type AccountLastOrigin<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, u32, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Derive new origin for account.
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(T::WeightInfo::new_origin())]
        #[pallet::call_index(0)]
        pub fn new_origin(
            origin: OriginFor<T>,
            origin_kind: T::CustomOriginKind,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let next_index = AccountLastOrigin::<T>::get(&who);
            let new_origin = origin_kind.derive(&who, next_index);
            AccountOrigin::<T>::insert(&who, next_index, new_origin.clone());
            AccountLastOrigin::<T>::insert(&who, next_index + 1);

            Self::deposit_event(Event::NewOrigin {
                account: who,
                origin: new_origin,
            });

            Ok(())
        }

        /// Dispatch the given `call` from an account that the sender is authorised.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// Parameters:
        /// - `origin_index`: Account origin index for using.
        /// - `call`: The call to be made by the `derived` account.
        #[pallet::weight({
			let di = call.get_dispatch_info();
			(T::WeightInfo::proxy_call()
				.saturating_add(T::DbWeight::get().reads_writes(1, 1))
				.saturating_add(di.weight),
			di.class)
		})]
        #[pallet::call_index(1)]
        pub fn proxy_call(
            origin: OriginFor<T>,
            #[pallet::compact] origin_index: u32,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                origin_index < AccountLastOrigin::<T>::get(&who),
                Error::<T>::UnregisteredOrigin
            );

            let custom_origin = AccountOrigin::<T>::get(&who, origin_index)
                .ok_or(Error::<T>::UnregisteredOrigin)?;

            let e = if let Ok(id) = custom_origin.clone().try_into() {
                // in case of native dispatch with system signed origin
                call.dispatch(frame_system::RawOrigin::Signed(id).into())
            } else {
                // in other case dispatch with custom origin
                call.dispatch(custom_origin.clone().into())
            };

            Self::deposit_event(Event::ProxyCall {
                origin: custom_origin,
                result: e.map(|_| ()).map_err(|e| e.error),
            });

            Ok(())
        }
    }
}
