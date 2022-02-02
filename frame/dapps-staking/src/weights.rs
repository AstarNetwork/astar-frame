#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_staking.
pub trait WeightInfo {
    fn register() -> Weight;
    fn unregister() -> Weight;
	fn withdraw_from_unregistered() -> Weight;
    fn enable_developer_pre_approval() -> Weight;
    fn developer_pre_approval() -> Weight;
    fn bond_and_stake() -> Weight;
    fn unbond_and_unstake() -> Weight;
    fn withdraw_unbonded() -> Weight;
    fn claim_staker() -> Weight;
	fn claim_dapp() -> Weight;
    fn force_new_era() -> Weight;
}

/// Weights for pallet_staking using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDevelopers (r:1 w:1)
	// Storage: DappsStaking RegisteredDapps (r:1 w:1)
	// Storage: DappsStaking PreApprovalIsEnabled (r:1 w:0)
	fn register() -> Weight {
		(37_109_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:1)
	// Storage: DappsStaking RegisteredDevelopers (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	fn unregister() -> Weight {
		(50_003_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	fn withdraw_from_unregistered() -> Weight {
		(50_895_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking PreApprovalIsEnabled (r:0 w:1)
	fn enable_developer_pre_approval() -> Weight {
		(3_326_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking PreApprovedDevelopers (r:1 w:1)
	fn developer_pre_approval() -> Weight {
		(6_212_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:1)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	fn bond_and_stake() -> Weight {
		(108_181_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:1)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	fn unbond_and_unstake() -> Weight {
		(106_107_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	fn withdraw_unbonded() -> Weight {
		(81_501_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:0)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:0)
	fn claim_staker() -> Weight {
		(41_878_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:0)
	fn claim_dapp() -> Weight {
		(35_316_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking ForceEra (r:0 w:1)
	fn force_new_era() -> Weight {
		(3_466_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}




// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDevelopers (r:1 w:1)
	// Storage: DappsStaking RegisteredDapps (r:1 w:1)
	// Storage: DappsStaking PreApprovalIsEnabled (r:1 w:0)
	fn register() -> Weight {
		(37_109_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:1)
	// Storage: DappsStaking RegisteredDevelopers (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: System Account (r:1 w:1)
	fn unregister() -> Weight {
		(50_003_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	fn withdraw_from_unregistered() -> Weight {
		(50_895_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking PreApprovalIsEnabled (r:0 w:1)
	fn enable_developer_pre_approval() -> Weight {
		(3_326_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking PreApprovedDevelopers (r:1 w:1)
	fn developer_pre_approval() -> Weight {
		(6_212_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:1)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	fn bond_and_stake() -> Weight {
		(108_181_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(8 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:1)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	fn unbond_and_unstake() -> Weight {
		(106_107_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(8 as Weight))
			.saturating_add(RocksDbWeight::get().writes(5 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking Ledger (r:1 w:1)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: Balances Locks (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:1)
	fn withdraw_unbonded() -> Weight {
		(81_501_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking StakersInfo (r:1 w:1)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:0)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:0)
	fn claim_staker() -> Weight {
		(41_878_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking RegisteredDapps (r:1 w:0)
	// Storage: DappsStaking CurrentEra (r:1 w:0)
	// Storage: DappsStaking ContractEraStake (r:1 w:1)
	// Storage: DappsStaking EraRewardsAndStakes (r:1 w:0)
	fn claim_dapp() -> Weight {
		(35_316_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(5 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: DappsStaking PalletDisabled (r:1 w:0)
	// Storage: DappsStaking ForceEra (r:0 w:1)
	fn force_new_era() -> Weight {
		(3_466_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}
