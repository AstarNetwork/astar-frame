//! Autogenerated weights for `pallet_block_reward`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-12-29, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `shiden-collator-02-ovh`, CPU: `Intel(R) Xeon(R) E-2136 CPU @ 3.30GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("astar-dev"), DB CACHE: 1024

// Executed Command:
// ./astar-collator
// benchmark
// pallet
// --chain
// astar-dev
// --execution
// wasm
// --wasm-execution
// compiled
// --heap-pages
// 4096
// --pallet
// pallet_block_reward
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --output
// pallet_block_reward_weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet-reward-distribution.
pub trait WeightInfo {
    fn set_configuration() -> Weight;
}

/// Weights for pallet-reward-distribution using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: RewardDistribution RewardDistributionConfigStorage (r:0 w:1)
	fn set_configuration() -> Weight {
		Weight::from_ref_time(13_636_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: RewardDistribution RewardDistributionConfigStorage (r:0 w:1)
	fn set_configuration() -> Weight {
		Weight::from_ref_time(13_636_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}
