#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as XcAssetConfig;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use sp_std::boxed::Box;
use xcm::v1::MultiLocation;

benchmarks! {

    register_asset_location {
        let asset_location = MultiLocation::parent();
        let asset_id = T::AssetId::default();

    }: _(RawOrigin::Root, Box::new(asset_location.clone().versioned()), asset_id)
    verify {
        assert_eq!(AssetIdToLocation::<T>::get(&asset_id), Some(asset_location.versioned()));
    }

    set_asset_units_per_second {
        let asset_location = MultiLocation::parent();
        let asset_id = T::AssetId::default();
        let units = 123;

        XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), asset_id)?;

    }: _(RawOrigin::Root, Box::new(asset_location.clone().versioned()), units)
    verify {
        assert_eq!(AssetLocationUnitsPerSecond::<T>::get(&asset_location.versioned()), Some(units));
    }

    change_existing_asset_location {
        let asset_location = MultiLocation::parent();
        let asset_id = T::AssetId::default();
        let units = 123;

        XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), asset_id)?;
        XcAssetConfig::<T>::set_asset_units_per_second(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), units)?;

        let new_asset_location = MultiLocation::here();

    }: _(RawOrigin::Root, Box::new(new_asset_location.clone().versioned()), asset_id)
    verify {
        assert!(!AssetLocationToId::<T>::contains_key(&asset_location.clone().versioned()));
        assert_eq!(AssetLocationToId::<T>::get(&new_asset_location.clone().versioned()), Some(asset_id));
        assert_eq!(AssetLocationUnitsPerSecond::<T>::get(&new_asset_location.versioned()), Some(units));
    }

    remove_payment_asset {
        let asset_location = MultiLocation::parent();
        let asset_id = T::AssetId::default();
        let units = 123;

        XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), asset_id)?;
        XcAssetConfig::<T>::set_asset_units_per_second(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), units)?;

    }: _(RawOrigin::Root, Box::new(asset_location.clone().versioned()))
    verify {
        assert!(!AssetLocationUnitsPerSecond::<T>::contains_key(&asset_location.versioned()));
    }

    remove_asset {
        let asset_location = MultiLocation::parent();
        let asset_id = T::AssetId::default();
        let units = 123;

        XcAssetConfig::<T>::register_asset_location(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), asset_id)?;
        XcAssetConfig::<T>::set_asset_units_per_second(RawOrigin::Root.into(), Box::new(asset_location.clone().versioned()), units)?;

    }: _(RawOrigin::Root, asset_id)
    verify {
        assert!(!AssetLocationToId::<T>::contains_key(&asset_location.clone().versioned()));
        assert!(!AssetIdToLocation::<T>::contains_key(asset_id));
        assert!(!AssetLocationUnitsPerSecond::<T>::contains_key(&asset_location.versioned()));
    }

}

#[cfg(test)]
mod tests {
    use crate::mock;
    use sp_io::TestExternalities;

    pub fn new_test_ext() -> TestExternalities {
        mock::ExternalityBuilder::build()
    }
}

impl_benchmark_test_suite!(
    XcAssetConfig,
    crate::benchmarking::tests::new_test_ext(),
    crate::mock::Test
);
