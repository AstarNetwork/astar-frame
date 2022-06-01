#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_asset_manager.
pub trait WeightInfo {
	fn register_asset_location() -> Weight;
	fn set_asset_units_per_second() -> Weight;
	fn change_existing_asset_location() -> Weight;
	fn remove_payment_asset() -> Weight;
	fn remove_asset() -> Weight;
}

/// Weights for pallet_asset_manager using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: AssetManager AssetIdType (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Metadata (r:1 w:1)
	// Storage: AssetManager AssetTypeId (r:0 w:1)
	fn register_asset_location() -> Weight {
		(47_597_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: AssetManager AssetTypeId (r:1 w:0)
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:0 w:1)
	fn set_asset_units_per_second() -> Weight {
		(32_865_000 as Weight)
			// Standard Error: 3_000
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetIdType (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:1 w:2)
	// Storage: AssetManager AssetTypeId (r:0 w:2)
	fn change_existing_asset_location() -> Weight {
		(40_757_000 as Weight)
			// Standard Error: 4_000
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(6 as Weight))
	}
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:0 w:1)
	fn remove_payment_asset() -> Weight {
		(25_700_000 as Weight)
			// Standard Error: 3_000
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetIdType (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:0 w:1)
	// Storage: AssetManager AssetTypeId (r:0 w:1)
	fn remove_asset() -> Weight {
		(32_359_000 as Weight)
			// Standard Error: 3_000
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: AssetManager AssetIdType (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Metadata (r:1 w:1)
	// Storage: AssetManager AssetTypeId (r:0 w:1)
	fn register_asset_location() -> Weight {
		(47_597_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
	// Storage: AssetManager AssetTypeId (r:1 w:0)
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:0 w:1)
	fn set_asset_units_per_second() -> Weight {
		(32_865_000 as Weight)
			// Standard Error: 3_000
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetIdType (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:1 w:2)
	// Storage: AssetManager AssetTypeId (r:0 w:2)
	fn change_existing_asset_location() -> Weight {
		(40_757_000 as Weight)
			// Standard Error: 4_000
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(6 as Weight))
	}
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:0 w:1)
	fn remove_payment_asset() -> Weight {
		(25_700_000 as Weight)
			// Standard Error: 3_000
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: AssetManager SupportedFeePaymentAssets (r:1 w:1)
	// Storage: AssetManager AssetIdType (r:1 w:1)
	// Storage: AssetManager AssetTypeUnitsPerSecond (r:0 w:1)
	// Storage: AssetManager AssetTypeId (r:0 w:1)
	fn remove_asset() -> Weight {
		(32_359_000 as Weight)
			// Standard Error: 3_000
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
}
