//! # XCM Primitives
//!
//! ## Overview
//!
//! Collection of common XCM primitives used by runtimes.
//!
//! - `AssetLocationIdConverter` - conversion between local asset Id and cross-chain asset multilocation
//! - `FixedRateOfForeignAsset` - weight trader for execution payment in foreign asset
//! - `
//! Please refer to concrete
//!

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::{constants::WEIGHT_PER_SECOND, Weight};
use sp_runtime::traits::Bounded;
use sp_std::{borrow::Borrow, marker::PhantomData};

// Polkadot imports
use xcm::latest::prelude::*;
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{FilterAssetLocation, WeightTrader};

use pallet_xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};

#[cfg(test)]
mod tests;

/// Used to convert between cross-chain asset multilocation and local asset Id.
///
/// This implementation relies on `XcAssetConfig` pallet to handle mapping.
/// In case asset location hasn't been mapped, it means the asset isn't supported (yet).
pub struct AssetLocationIdConverter<AssetId, AssetMapper>(
    sp_std::marker::PhantomData<(AssetId, AssetMapper)>,
);
impl<AssetId, AssetMapper> xcm_executor::traits::Convert<MultiLocation, AssetId>
    for AssetLocationIdConverter<AssetId, AssetMapper>
where
    AssetId: Clone + Eq + Bounded,
    AssetMapper: XcAssetLocation<AssetId>,
{
    fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
        if let Some(asset_id) = AssetMapper::get_asset_id(location.borrow().clone()) {
            Ok(asset_id)
        } else {
            Err(())
        }
    }

    fn reverse_ref(id: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
        if let Some(multilocation) = AssetMapper::get_xc_asset_location(id.borrow().clone()) {
            Ok(multilocation)
        } else {
            Err(())
        }
    }
}

/// Used as weight trader for foreign assets.
///
/// In case foreigin asset is supported as payment asset, XCM execution time
/// on-chain can be paid by the foreign asset, using the configured rate.
pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
    /// Total used weight
    weight: Weight,
    /// Total consumed assets
    consumed: u128,
    /// Asset Id (as MultiLocation) and units per second for payment
    asset_location_and_units_per_second: Option<(MultiLocation, u128)>,
    _pd: PhantomData<(T, R)>,
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> WeightTrader for FixedRateOfForeignAsset<T, R> {
    fn new() -> Self {
        Self {
            weight: 0,
            consumed: 0,
            asset_location_and_units_per_second: None,
            _pd: PhantomData,
        }
    }

    fn buy_weight(
        &mut self,
        weight: Weight,
        payment: xcm_executor::Assets,
    ) -> Result<xcm_executor::Assets, XcmError> {
        log::trace!(
            target: "xcm::weight",
            "FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
            weight, payment,
        );

        // Atm in pallet, we only support one asset so this should work
        let payment_asset = payment
            .fungible_assets_iter()
            .next()
            .ok_or(XcmError::TooExpensive)?;

        match payment_asset {
            MultiAsset {
                id: xcm::latest::AssetId::Concrete(asset_location),
                fun: Fungibility::Fungible(_),
            } => {
                if let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) {
                    let amount = units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128);
                    if amount == 0 {
                        return Ok(payment);
                    }

                    let unused = payment
                        .checked_sub((asset_location.clone(), amount).into())
                        .map_err(|_| XcmError::TooExpensive)?;

                    self.weight = self.weight.saturating_add(weight);

                    // If there are multiple calls to `BuyExecution` but with different assets, we need to be able to handle that.
                    // Current primitive implementation will just keep total track of consumed asset for the FIRST consumed asset.
                    // Others will just be ignored when refund is concerned.
                    if let Some((old_asset_location, _)) =
                        self.asset_location_and_units_per_second.clone()
                    {
                        if old_asset_location == asset_location {
                            self.consumed = self.consumed.saturating_add(amount);
                        }
                    } else {
                        self.consumed = self.consumed.saturating_add(amount);
                        self.asset_location_and_units_per_second =
                            Some((asset_location, units_per_second));
                    }

                    return Ok(unused);
                } else {
                    return Err(XcmError::TooExpensive);
                }
            }
            _ => Err(XcmError::TooExpensive),
        }
    }

    fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
        log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

        if let Some((asset_location, units_per_second)) =
            self.asset_location_and_units_per_second.clone()
        {
            let weight = weight.min(self.weight);
            let amount = units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128);

            self.weight = self.weight.saturating_sub(weight);
            self.consumed = self.consumed.saturating_sub(amount);

            if amount > 0 {
                Some((asset_location, amount).into())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> Drop for FixedRateOfForeignAsset<T, R> {
    fn drop(&mut self) {
        if let Some((asset_location, _)) = self.asset_location_and_units_per_second.clone() {
            if self.consumed > 0 {
                R::take_revenue((asset_location, self.consumed).into());
            }
        }
    }
}

/// Used to determine whether the cross-chain asset is coming from a trusted reserve or not
///
/// Basically, we trust any cross-chain asset from any location to act as a reserve since
/// in order to support the xc-asset, we need to first register it in the `XcAssetConfig` pallet.
///
pub struct ReserveAssetFilter;
impl FilterAssetLocation for ReserveAssetFilter {
    fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
        // We assume that relay chain and sibling parachain assets are trusted reserves for their assets
        let reserve_location = if let Concrete(location) = &asset.id {
            match (location.parents, location.first_interior()) {
                // sibling parachain
                (1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
                // relay chain
                (1, _) => Some(MultiLocation::parent()),
                _ => None,
            }
        } else {
            None
        };

        if let Some(ref reserve) = reserve_location {
            origin == reserve
        } else {
            false
        }
    }
}
