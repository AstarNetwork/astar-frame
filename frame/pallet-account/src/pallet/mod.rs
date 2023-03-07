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
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Custom origin type that used for derived accounts.
        type CustomOrigin: Parameter + TryInto<Self::AccountId>;
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
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        CallExecuted {
            origin: T::CustomOrigin,
            result: DispatchResult,
        },
    }

    #[pallet::origin]
    pub type Origin<T> = <T as Config>::CustomOrigin;

    /// Account origins
    #[pallet::storage]
    pub type AccountOrigin<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<T::CustomOrigin>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
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
				 // AccountData for inner call origin accountdata.
				.saturating_add(T::DbWeight::get().reads_writes(1, 1))
				.saturating_add(di.weight),
			di.class)
		})]
        #[pallet::call_index(0)]
        pub fn proxy_call(
            origin: OriginFor<T>,
            #[pallet::compact] origin_index: u32,
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let custom_origin = AccountOrigin::<T>::get(who)
                .get(origin_index as usize)
                .ok_or(Error::<T>::UnregisteredOrigin)?
                .clone();

            let e = if let Ok(id) = custom_origin.clone().try_into() {
                // in case of native dispatch with system signed origin
                call.dispatch(frame_system::RawOrigin::Signed(id).into())
            } else {
                // in other case dispatch with custom origin
                call.dispatch(custom_origin.clone().into())
            };

            Self::deposit_event(Event::CallExecuted {
                origin: custom_origin,
                result: e.map(|_| ()).map_err(|e| e.error),
            });

            Ok(())
        }

        /// Derive new origin for account.
        ///
        /// The dispatch origin for this call must be _Signed_.
        #[pallet::weight(T::WeightInfo::new_origin())]
        #[pallet::call_index(1)]
        pub fn new_origin(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Ok(())
        }
    }
}
