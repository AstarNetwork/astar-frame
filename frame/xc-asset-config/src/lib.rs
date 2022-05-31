#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;
pub use pallet::*;

// TODO
// #[cfg(any(test, feature = "runtime-benchmarks"))]
// mod benchmarks;

// #[cfg(test)]
// pub mod mock;
// #[cfg(test)]
// pub mod tests;

pub mod weights;

#[pallet]
pub mod pallet {

    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::HasCompact;

    #[pallet::pallet]
    #[pallet::without_storage_info] // TODO: MultiLocation could be problematic?
    pub struct Pallet<T>(PhantomData<T>);

    /// Defines conversion between asset Id and asset type
    pub trait AssetTypeGetter<AssetId, AssetType> {
        /// Get asset type from assetId
        fn get_asset_type(asset_id: AssetId) -> Option<AssetType>;

        /// Get assetId from assetType
        fn get_asset_id(asset_type: AssetType) -> Option<AssetId>;
    }

    /// Used to fetch `units per second` if asset is applicable for local execution payment
    pub trait UnitsToWeightRatio<AssetType> {
        /// returns units per second from asset type or `None` if asset type isn't a supported payment asset
        fn get_units_per_second(asset_type: AssetType) -> Option<u128>;
    }

    impl<T: Config> AssetTypeGetter<T::AssetId, T::AssetType> for Pallet<T> {
        fn get_asset_type(asset_id: T::AssetId) -> Option<T::AssetType> {
            AssetIdToType::<T>::get(asset_id)
        }

        fn get_asset_id(asset_type: T::AssetType) -> Option<T::AssetId> {
            AssetTypeToId::<T>::get(asset_type)
        }
    }

    impl<T: Config> UnitsToWeightRatio<T::AssetType> for Pallet<T> {
        fn get_units_per_second(asset_type: T::AssetType) -> Option<u128> {
            AssetTypeUnitsPerSecond::<T>::get(asset_type)
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The Asset Id. This will be used to create the asset and to associate it with
        /// a assetType
        type AssetId: Member + Parameter + Default + Copy + HasCompact + MaxEncodedLen;

        /// The Foreign Asset Kind.
        type AssetType: Parameter + Member + Ord + PartialOrd + Into<Self::AssetId> + Default;

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
            asset_type: T::AssetType,
            asset_id: T::AssetId,
        },
        /// Changed the amount of units we are charging per execution second for a given asset
        UnitsPerSecondChanged {
            asset_type: T::AssetType,
            units_per_second: u128,
        },
        /// Changed the xcm type mapping for a given asset id
        AssetTypeChanged {
            previous_asset_type: T::AssetType,
            asset_id: T::AssetId,
            new_asset_type: T::AssetType,
        },
        /// Removed all information related to an asset Id
        AssetRemoved {
            asset_id: T::AssetId,
            asset_type: T::AssetType,
        },
        /// Supported asset type for fee payment removed
        SupportedAssetRemoved { asset_type: T::AssetType },
    }

    /// Mapping from an asset id to asset type.
    /// Can be used when receiving transaction specifying an asset directly,
    /// like transferring an asset from this chain to another.
    #[pallet::storage]
    #[pallet::getter(fn asset_id_type)]
    pub type AssetIdToType<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetId, T::AssetType>;

    /// Mapping from an asset type to an asset id.
    /// Can be used when receiving a multilocation XCM message to retrieve
    /// the corresponding asset in which tokens should me minted.
    #[pallet::storage]
    #[pallet::getter(fn asset_type_id)]
    pub type AssetTypeToId<T: Config> = StorageMap<_, Blake2_128Concat, T::AssetType, T::AssetId>;

    /// Stores the units per second for local execution for a AssetType.
    /// This is used to know how to charge for XCM execution in a particular asset.
    ///
    /// Not all asset types are supported for payment. If value exists here, it means it is supported.
    #[pallet::storage]
    #[pallet::getter(fn asset_type_units_per_second)]
    pub type AssetTypeUnitsPerSecond<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetType, u128>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register new asset type to asset Id mapping.
        #[pallet::weight(T::WeightInfo::register_asset_type())]
        pub fn register_asset_type(
            origin: OriginFor<T>,
            asset_type: T::AssetType,
            asset_id: T::AssetId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Ensure such an assetId does not exist
            ensure!(
                !AssetIdToType::<T>::contains_key(&asset_id),
                Error::<T>::AssetAlreadyRegistered
            );

            // TODO: check if this asset Id is actually registered? Will need a special type for this.
            // T::SomeType::contains(...);

            AssetIdToType::<T>::insert(&asset_id, &asset_type);
            AssetTypeToId::<T>::insert(&asset_type, &asset_id);

            Self::deposit_event(Event::AssetRegistered {
                asset_type,
                asset_id,
            });
            Ok(())
        }

        /// Change the amount of units we are charging per execution second
        /// for a given AssetType
        #[pallet::weight(T::WeightInfo::set_asset_units_per_second())]
        pub fn set_asset_units_per_second(
            origin: OriginFor<T>,
            asset_type: T::AssetType,
            units_per_second: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(
                AssetTypeToId::<T>::contains_key(&asset_type),
                Error::<T>::AssetDoesNotExist
            );

            AssetTypeUnitsPerSecond::<T>::insert(&asset_type, &units_per_second);

            Self::deposit_event(Event::UnitsPerSecondChanged {
                asset_type,
                units_per_second,
            });
            Ok(())
        }

        /// Change the xcm type mapping for a given asset Id.
        /// The new asset type will inherit old `units per second` value.
        #[pallet::weight(T::WeightInfo::change_existing_asset_type())]
        pub fn change_existing_asset_type(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            new_asset_type: T::AssetType,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let previous_asset_type =
                AssetIdToType::<T>::get(&asset_id).ok_or(Error::<T>::AssetDoesNotExist)?;

            // Insert new asset type info
            AssetIdToType::<T>::insert(&asset_id, &new_asset_type);
            AssetTypeToId::<T>::insert(&new_asset_type, &asset_id);

            // Remove previous asset type info
            AssetTypeToId::<T>::remove(&previous_asset_type);

            // Change AssetTypeUnitsPerSecond
            if let Some(units) = AssetTypeUnitsPerSecond::<T>::take(&previous_asset_type) {
                AssetTypeUnitsPerSecond::<T>::insert(&new_asset_type, units);
            }

            Self::deposit_event(Event::AssetTypeChanged {
                previous_asset_type,
                asset_id,
                new_asset_type,
            });
            Ok(())
        }

        #[pallet::weight(T::WeightInfo::remove_supported_asset())]
        pub fn remove_supported_asset(
            origin: OriginFor<T>,
            asset_type: T::AssetType,
        ) -> DispatchResult {
            ensure_root(origin)?;

            AssetTypeUnitsPerSecond::<T>::remove(&asset_type);

            Self::deposit_event(Event::SupportedAssetRemoved { asset_type });
            Ok(())
        }

        /// Remove a given assetId -> assetType association
        #[pallet::weight(T::WeightInfo::remove_existing_asset_type())]
        pub fn remove_asset_type(origin: OriginFor<T>, asset_id: T::AssetId) -> DispatchResult {
            ensure_root(origin)?;

            let asset_type =
                AssetIdToType::<T>::get(&asset_id).ok_or(Error::<T>::AssetDoesNotExist)?;

            AssetIdToType::<T>::remove(&asset_id);
            AssetTypeToId::<T>::remove(&asset_type);
            AssetTypeUnitsPerSecond::<T>::remove(&asset_type);

            Self::deposit_event(Event::AssetRemoved {
                asset_id,
                asset_type,
            });
            Ok(())
        }
    }
}
