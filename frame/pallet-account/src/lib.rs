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
//! An accout abstraction pallet make possible to derive new blockchain based
//! account for your existed external owned account (seed phrase based). The onchain
//! account could be drived to multiple address spaces: H160 and SS58. For example,
//! it makes possible predictable interaction between substrate native account and
//! EVM smart contracts.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! * proxy() - make proxy call with derived account as origin
//!
//!
//! ### Other
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::weights::Weight;
use sp_core::RuntimeDebug;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// A method to derive new account from existed one
pub trait AccountDeriving<AccountId> {
    /// Derive new account from existed one
    fn derive(&self, source: &AccountId) -> AccountId;
}

/// Use simple salt and Blake2 hash for account deriving.
#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, scale_info::TypeInfo, RuntimeDebug)]
pub struct SimpleSalt(pub u32);

impl<AccountId: AsRef<[u8]> + From<[u8; 32]>> AccountDeriving<AccountId> for SimpleSalt {
    fn derive(&self, source: &AccountId) -> AccountId {
        let salted_source = [source.as_ref(), &self.encode()[..]].concat();
        sp_core::blake2_256(&salted_source).into()
    }
}

pub trait WeightInfo {
    fn call_as() -> Weight;
}

impl WeightInfo for () {
    fn call_as() -> Weight {
        Default::default()
    }
}

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

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Supported account deriving types.
        type DerivingType: Parameter + AccountDeriving<Self::AccountId>;
        /// The overarching call type.
        type RuntimeCall: Parameter
            + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
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
    pub enum Error<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        CallExecuted {
            account: T::AccountId,
            result: DispatchResult,
        },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Dispatch the given `call` from an account that the sender is authorised.
        ///
        /// The dispatch origin for this call must be _Signed_.
        ///
        /// Parameters:
        /// - `deriving`: Account deriving method to be used for the call.
        /// - `call`: The call to be made by the `derived` account.
        #[pallet::weight({
			let di = call.get_dispatch_info();
			(T::WeightInfo::call_as()
				 // AccountData for inner call origin accountdata.
				.saturating_add(T::DbWeight::get().reads_writes(1, 1))
				.saturating_add(di.weight),
			di.class)
		})]
        #[pallet::call_index(0)]
        pub fn call_as(
            origin: OriginFor<T>,
            deriving: T::DerivingType,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let account = deriving.derive(&who);
            let origin: T::RuntimeOrigin = frame_system::RawOrigin::Signed(account.clone()).into();
            let e = call.dispatch(origin);

            Self::deposit_event(Event::CallExecuted {
                account,
                result: e.map(|_| ()).map_err(|e| e.error),
            });

            Ok(())
        }
    }
}
