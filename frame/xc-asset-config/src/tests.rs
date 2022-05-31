use super::{pallet::Error, pallet::Event, *};
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_runtime::traits::{BadOrigin, Zero};
use xcm::latest::prelude::*;

use xcm::v1::MultiLocation; // The one we use ATM

#[test]
fn only_root_as_origin() {
    ExternalityBuilder::build().execute_with(|| {
        let asset_location = MultiLocation::here().versioned();
        let asset_id = 7;

        assert_noop!(
            XcAssetConfig::register_asset_location(
                Origin::signed(1),
                Box::new(asset_location.clone()),
                asset_id
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::set_asset_units_per_second(
                Origin::signed(1),
                Box::new(asset_location.clone()),
                9
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::change_existing_asset_location(
                Origin::signed(1),
                Box::new(asset_location.clone()),
                asset_id
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::remove_supported_asset(
                Origin::signed(1),
                Box::new(asset_location.clone()),
            ),
            BadOrigin
        );

        assert_noop!(
            XcAssetConfig::remove_asset_info(Origin::signed(1), asset_id,),
            BadOrigin
        );
    })
}

#[test]
fn register_asset_location_and_ups_is_ok() {
    ExternalityBuilder::build().execute_with(|| {
        // Prepare location and Id
        let asset_location = MultiLocation::new(
            1,
            Junctions::X2(Junction::PalletInstance(17), GeneralIndex(7)),
        );
        let asset_id = 13;

        // Register asset and ensure it's ok
        assert_ok!(XcAssetConfig::register_asset_location(
            Origin::root(),
            Box::new(asset_location.clone().versioned()),
            asset_id
        ));

        // Assert storage state after registering asset
        assert_eq!(
            AssetIdToLocation::<Test>::get(&asset_id).unwrap(),
            asset_location.clone().versioned()
        );
        assert_eq!(
            AssetLocationToId::<Test>::get(asset_location.clone().versioned()).unwrap(),
            asset_id
        );
        assert!(!AssetLocationUnitsPerSecond::<Test>::contains_key(
            asset_location.clone().versioned()
        ));

        // Register unit per second rate and verify storage
        let units: u128 = 7 * 11 * 13 * 17 * 29;
        assert_ok!(XcAssetConfig::set_asset_units_per_second(
            Origin::root(),
            Box::new(asset_location.clone().versioned()),
            units
        ));
        assert_eq!(
            AssetLocationUnitsPerSecond::<Test>::get(&asset_location.clone().versioned()).unwrap(),
            units
        );
    })
}
