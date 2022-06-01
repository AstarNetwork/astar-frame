//! # Cross-chain Asset Config Pallet
//!
//! ## Overview
//!
//! This pallet provides mappings between local asset Id and remove asset location.
//! E.g. a multilocation like `{parents: 0, interior: X1::(Junction::Parachain(1000))}` could ba mapped to local asset Id `789`.
//!
//! Additionally, it stores information whether a foreign asset is supported as a payment currency for execution on local network.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! - `register_asset_location` - used to register mapping between local asset Id and remote asset location
//! - `set_asset_units_per_second` - registers asset as payment currency and sets the desired payment per second of execution time
//! - `change_existing_asset_location` - changes the remote location of an existing local asset Id
//! - `remove_payment_asset` - removes asset from the set of supported payment assets
//! - `remove_asset` - removes all information related to this asset
//!
//! User is encouraged to refer to specific function implementations for more comprehensive documentation.
//!
//! ### Other
//!
//! `AssetLocationGetter` interface for mapping asset Id to asset location and vice versa
//! - `get_asset_location`
//! - `get_asset_id`
//!
//! `ExecutionPaymentRate` interface for fetching `units per second` if asset is supported payment asset
//! - `get_units_per_second`
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod benchmarking;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

pub mod weights;

#[pallet]
pub mod pallet {

    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::HasCompact;
    use sp_std::boxed::Box;
    use xcm::{v1::MultiLocation, VersionedMultiLocation};

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    /// Defines conversion between asset Id and asset type
    pub trait AssetLocationGetter<AssetId> {
        /// Get asset type from assetId
        fn get_asset_location(asset_id: AssetId) -> Option<MultiLocation>;

        /// Get assetId from AssetLocation
        fn get_asset_id(asset_location: MultiLocation) -> Option<AssetId>;
    }

    /// Used to fetch `units per second` if asset is applicable for local execution payment.
    pub trait ExecutionPaymentRate {
        /// returns units per second from asset type or `None` if asset type isn't a supported payment asset.
        fn get_units_per_second(asset_location: MultiLocation) -> Option<u128>;
    }

    impl<T: Config> AssetLocationGetter<T::AssetId> for Pallet<T> {
        fn get_asset_location(asset_id: T::AssetId) -> Option<MultiLocation> {
            AssetIdToLocation::<T>::get(asset_id).map_or(None, |x| x.try_into().ok())
        }

        fn get_asset_id(asset_location: MultiLocation) -> Option<T::AssetId> {
            AssetLocationToId::<T>::get(asset_location.versioned())
        }
    }

    impl<T: Config> ExecutionPaymentRate for Pallet<T> {
        fn get_units_per_second(asset_location: MultiLocation) -> Option<u128> {
            AssetLocationUnitsPerSecond::<T>::get(asset_location.versioned())
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The Asset Id. This will be used to create the asset and to associate it with
        /// a AssetLocation
        type AssetId: Member + Parameter + Default + Copy + HasCompact + MaxEncodedLen;

        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Asset is already registered.
        AssetAlreadyRegistered,
        /// Asset does not exist (hasn't been registered).
        AssetDoesNotExist,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Registed mapping between asset type and asset Id.
        AssetRegistered {
            asset_location: VersionedMultiLocation,
            asset_id: T::AssetId,
        },
        /// Changed the amount of units we are charging per execution second for an asset
        UnitsPerSecondChanged {
            asset_location: VersionedMultiLocation,
            units_per_second: u128,
        },
        /// Changed the asset type mapping for a given asset id
        AssetLocationChanged {
            previous_asset_location: VersionedMultiLocation,
            asset_id: T::AssetId,
            new_asset_location: VersionedMultiLocation,
        },
        /// Supported asset type for fee payment removed.
        SupportedAssetRemoved {
            asset_location: VersionedMultiLocation,
        },
        /// Removed all information related to an asset Id
        AssetRemoved {
            asset_location: VersionedMultiLocation,
            asset_id: T::AssetId,
        },
    }

    /// Mapping from an asset id to asset type.
    /// Can be used when receiving transaction specifying an asset directly,
    /// like transferring an asset from this chain to another.
    #[pallet::storage]
    #[pallet::getter(fn asset_id_to_location)]
    pub type AssetIdToLocation<T: Config> =
        StorageMap<_, Twox64Concat, T::AssetId, VersionedMultiLocation>;

    /// Mapping from an asset type to an asset id.
    /// Can be used when receiving a multilocation XCM message to retrieve
    /// the corresponding asset in which tokens should me minted.
    #[pallet::storage]
    #[pallet::getter(fn asset_location_to_id)]
    pub type AssetLocationToId<T: Config> =
        StorageMap<_, Twox64Concat, VersionedMultiLocation, T::AssetId>;

    /// Stores the units per second for local execution for a AssetLocation.
    /// This is used to know how to charge for XCM execution in a particular asset.
    ///
    /// Not all asset types are supported for payment. If value exists here, it means it is supported.
    #[pallet::storage]
    #[pallet::getter(fn asset_location_units_per_second)]
    pub type AssetLocationUnitsPerSecond<T: Config> =
        StorageMap<_, Twox64Concat, VersionedMultiLocation, u128>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register new asset location to asset Id mapping.
        ///
        /// This makes the asset eligible for XCM interaction.
        #[pallet::weight(T::WeightInfo::register_asset_location())]
        pub fn register_asset_location(
            origin: OriginFor<T>,
            asset_location: Box<VersionedMultiLocation>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Ensure such an assetId does not exist
            ensure!(
                !AssetIdToLocation::<T>::contains_key(&asset_id),
                Error::<T>::AssetAlreadyRegistered
            );

            let asset_location = *asset_location;

            AssetIdToLocation::<T>::insert(&asset_id, asset_location.clone());
            AssetLocationToId::<T>::insert(&asset_location, asset_id.clone());

            Self::deposit_event(Event::AssetRegistered {
                asset_location,
                asset_id,
            });
            Ok(())
        }

        /// Change the amount of units we are charging per execution second
        /// for a given AssetLocation.
        #[pallet::weight(T::WeightInfo::set_asset_units_per_second())]
        pub fn set_asset_units_per_second(
            origin: OriginFor<T>,
            asset_location: Box<VersionedMultiLocation>,
            #[pallet::compact] units_per_second: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let asset_location = *asset_location;

            ensure!(
                AssetLocationToId::<T>::contains_key(&asset_location),
                Error::<T>::AssetDoesNotExist
            );

            AssetLocationUnitsPerSecond::<T>::insert(&asset_location, units_per_second);

            Self::deposit_event(Event::UnitsPerSecondChanged {
                asset_location,
                units_per_second,
            });
            Ok(())
        }

        /// Change the xcm type mapping for a given asset Id.
        /// The new asset type will inherit old `units per second` value.
        #[pallet::weight(T::WeightInfo::change_existing_asset_location())]
        pub fn change_existing_asset_location(
            origin: OriginFor<T>,
            new_asset_location: Box<VersionedMultiLocation>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let new_asset_location = *new_asset_location;

            let previous_asset_location =
                AssetIdToLocation::<T>::get(&asset_id).ok_or(Error::<T>::AssetDoesNotExist)?;

            // Insert new asset type info
            AssetIdToLocation::<T>::insert(&asset_id, new_asset_location.clone());
            AssetLocationToId::<T>::insert(&new_asset_location, asset_id.clone());

            // Remove previous asset type info
            AssetLocationToId::<T>::remove(&previous_asset_location);

            // Change AssetLocationUnitsPerSecond
            if let Some(units) = AssetLocationUnitsPerSecond::<T>::take(&previous_asset_location) {
                AssetLocationUnitsPerSecond::<T>::insert(&new_asset_location, units);
            }

            Self::deposit_event(Event::AssetLocationChanged {
                previous_asset_location,
                asset_id,
                new_asset_location,
            });
            Ok(())
        }

        /// Removes asset from the set of supported payment assets.
        ///
        /// The asset can still be interacted with via XCM but it cannot be used to pay for execution time.
        #[pallet::weight(T::WeightInfo::remove_payment_asset())]
        pub fn remove_payment_asset(
            origin: OriginFor<T>,
            asset_location: Box<VersionedMultiLocation>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let asset_location = *asset_location;

            AssetLocationUnitsPerSecond::<T>::remove(&asset_location);

            Self::deposit_event(Event::SupportedAssetRemoved { asset_location });
            Ok(())
        }

        /// Removes all information related to asset, removing it from XCM support.
        #[pallet::weight(T::WeightInfo::remove_asset())]
        pub fn remove_asset(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let asset_location =
                AssetIdToLocation::<T>::get(&asset_id).ok_or(Error::<T>::AssetDoesNotExist)?;

            AssetIdToLocation::<T>::remove(&asset_id);
            AssetLocationToId::<T>::remove(&asset_location);
            AssetLocationUnitsPerSecond::<T>::remove(&asset_location);

            Self::deposit_event(Event::AssetRemoved {
                asset_id,
                asset_location,
            });
            Ok(())
        }
    }
}
