//! Dapps staking migration utility module

use super::*;
use codec::{Decode, FullCodec};
use frame_support::storage::unhashed;
use pallet::pallet::*;
use sp_std::fmt::Debug;

/// TODO: this should be part of `IterableStorageMap` and all other `Iterable` storage traits.
/// Translates a value from format `O` into format `V`.
/// If key is invalid, translation is ignored.
/// If translation function `F` fails (returns None), entry is removed from the underlying map.
fn translate<O: Decode + Debug, V: FullCodec + Debug, F: FnMut(O) -> Option<V>>(
    key: &[u8],
    mut f: F,
) {
    let value = match unhashed::get::<O>(key) {
        Some(value) => value,
        None => {
            return;
        }
    };

    match f(value) {
        Some(new) => {
            unhashed::put::<V>(key, &new);
        }
        None => unhashed::kill(key),
    }
}

pub mod v2 {

    use super::*;
    use codec::{Decode, Encode};
    use frame_support::{
        storage::generator::{StorageDoubleMap, StorageMap},
        traits::Get,
        weights::Weight,
    };
    use sp_std::collections::btree_map::BTreeMap;

    // #[cfg(feature = "try-runtime")]
    use frame_support::log;
    #[cfg(feature = "try-runtime")]
    use frame_support::traits::OnRuntimeUpgradeHelpersExt;
    #[cfg(feature = "try-runtime")]
    use sp_runtime::traits::Zero;

    // The old value used to store locked amount
    type OldLedger<T> = BalanceOf<T>;

    #[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
    pub struct NewAccountLedger<Balance: AtLeast32BitUnsigned + Default + Copy> {
        #[codec(compact)]
        locked: Balance,
        unbonding_info: UnbondingInfo<Balance>,
    }

    // The old struct used to sotre staking points. Contains unused `formed_staked_era` value.
    #[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug)]
    struct OldEraStakingPoints<AccountId: Ord, Balance: HasCompact> {
        total: Balance,
        stakers: BTreeMap<AccountId, Balance>,
        former_staked_era: EraIndex,
        claimed_rewards: Balance,
    }

    #[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
    struct NewEraStakingPoints<AccountId: Ord, Balance: HasCompact> {
        #[codec(compact)]
        total: Balance,
        stakers: BTreeMap<AccountId, Balance>,
        #[codec(compact)]
        claimed_rewards: Balance,
    }

    #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
    struct OldEraRewardAndStake<Balance> {
        rewards: Balance,
        staked: Balance,
    }

    #[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
    pub struct NewEraRewardAndStake<Balance: HasCompact> {
        #[codec(compact)]
        rewards: Balance,
        #[codec(compact)]
        staked: Balance,
    }

    /// Serves as migration state representation.
    /// E.g. we might be migrating `Ledger` but need to stop since we've reached the predefined weight limit.
    /// Therefore we use this enum to store migration state `MigrationState::Ledger(Some(last_processed_key))`.
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
    pub enum MigrationState {
        NotStarted,
        /// In the middle of `Ledger` migration.
        Ledger(Option<Vec<u8>>),
        /// In the middle of `StakingInfo` migration.
        StakingInfo(Option<Vec<u8>>),
        /// In the middle of `RewardsAndStakes` migration.
        RewardsAndStakes(Option<Vec<u8>>),
        Finished,
    }

    impl Default for MigrationState {
        fn default() -> Self {
            MigrationState::NotStarted
        }
    }

    #[cfg(feature = "try-runtime")]
    pub fn pre_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        assert_eq!(Version::V1_0_0, StorageVersion::<T>::get());

        let ledger_count = Ledger::<T>::iter_keys().count() as u64;
        U::set_temp_storage::<u64>(ledger_count, "ledger_count");

        let staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        U::set_temp_storage(staking_info_count, "staking_info_count");

        let rewards_and_stakes_count = EraRewardsAndStakes::<T>::iter_keys().count() as u64;
        U::set_temp_storage(rewards_and_stakes_count, "rewards_and_stakes_count");

        log::info!(
            ">>> PreMigrate: ledger count: {:?}, staking info count: {:?}, rewards&stakes count: {:?}",
            ledger_count,
            staking_info_count,
            rewards_and_stakes_count,
        );

        Ok(().into())
    }

    pub fn stateful_migrate<T: Config>(weight_limit: Weight) -> Weight {
        // Ensure this is a valid migration for this version
        if StorageVersion::<T>::get() != Version::V1_0_0 {
            return T::DbWeight::get().reads(1);
        }

        log::info!("Executing a step of stateful storage migration.");

        let mut migration_state = MigrationStateV2::<T>::get();
        let mut consumed_weight = T::DbWeight::get().reads(2);

        // The first storage we process is `Ledger` so we set the starting state if needed
        if migration_state == MigrationState::NotStarted {
            migration_state = MigrationState::Ledger(None);
            PalletDisabled::<T>::put(true);
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

            // If normal run, just exit here to avoid the risk of clogging the upgrade block.
            if !cfg!(feature = "try-runtime") {
                MigrationStateV2::<T>::put(migration_state);
                return consumed_weight;
            }
        }

        // Process ledger
        if let MigrationState::Ledger(last_processed_key) = migration_state.clone() {
            // First, get correct iterator.
            let key_iter = if let Some(previous_key) = last_processed_key {
                Ledger::<T>::iter_keys_from(previous_key)
            } else {
                Ledger::<T>::iter_keys()
            };

            for key in key_iter {
                let key_as_vec = Ledger::<T>::storage_map_final_key(key);
                translate(&key_as_vec, |value: OldLedger<T>| {
                    Some(NewAccountLedger {
                        locked: value,
                        unbonding_info: Default::default(),
                    })
                });

                // Increment total consumed weight.
                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                // Check if we've consumed enough weight already.
                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> Ledger migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV2::<T>::put(MigrationState::Ledger(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    // we want try-runtime to execute the entire migration
                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> Ledger migration finished.");
            // This means we're finished with migration of the Ledger. Hooray!
            // Next step of the migration should be configured.
            migration_state = MigrationState::StakingInfo(None);
        }

        if let MigrationState::StakingInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                ContractEraStake::<T>::iter_keys_from(previous_key)
            } else {
                ContractEraStake::<T>::iter_keys()
            };

            for (key1, key2) in key_iter {
                let key_as_vec = ContractEraStake::<T>::storage_double_map_final_key(key1, key2);
                translate(
                    &key_as_vec,
                    |value: OldEraStakingPoints<T::AccountId, BalanceOf<T>>| {
                        Some(NewEraStakingPoints {
                            total: value.total,
                            stakers: value.stakers,
                            claimed_rewards: value.claimed_rewards,
                        })
                    },
                );

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> EraStakingPoints migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV2::<T>::put(MigrationState::StakingInfo(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> EraStakingPoints migration finished.");

            migration_state = MigrationState::RewardsAndStakes(None);
        }

        if let MigrationState::RewardsAndStakes(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                EraRewardsAndStakes::<T>::iter_keys_from(previous_key)
            } else {
                EraRewardsAndStakes::<T>::iter_keys()
            };

            for key in key_iter {
                let key_as_vec = EraRewardsAndStakes::<T>::storage_map_final_key(key);
                translate(&key_as_vec, |value: OldEraRewardAndStake<BalanceOf<T>>| {
                    Some(NewEraRewardAndStake {
                        rewards: value.rewards,
                        staked: value.staked,
                    })
                });

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> EraRewardsAndStakes migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV2::<T>::put(MigrationState::RewardsAndStakes(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> EraRewardsAndStakes migration finished.");
        }

        MigrationStateV2::<T>::put(MigrationState::Finished);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));
        log::info!(">>> Migration finalized.");

        StorageVersion::<T>::put(Version::V2_0_0);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

        PalletDisabled::<T>::put(false);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

        consumed_weight
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        assert_eq!(Version::V2_0_0, StorageVersion::<T>::get());

        let init_ledger_count = U::get_temp_storage::<u64>("ledger_count").unwrap();
        let init_staking_info_count = U::get_temp_storage::<u64>("staking_info_count").unwrap();
        let init_reward_and_stakes_count =
            U::get_temp_storage::<u64>("rewards_and_stakes_count").unwrap();

        let current_ledger_count = Ledger::<T>::iter_keys().count() as u64;
        let current_staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        let current_rewards_and_stakes_count = EraRewardsAndStakes::<T>::iter_keys().count() as u64;

        assert_eq!(init_ledger_count, current_ledger_count);
        assert_eq!(init_staking_info_count, current_staking_info_count);
        assert_eq!(
            init_reward_and_stakes_count,
            current_rewards_and_stakes_count
        );

        for acc_ledger in Ledger::<T>::iter_values() {
            assert!(acc_ledger.locked > Zero::zero());
            assert!(acc_ledger.unbonding_info.is_empty());
        }

        log::info!(
            ">>> PostMigrate: ledger count: {:?}, staking info count: {:?}, rewards&stakes count: {:?}",
            current_ledger_count,
            current_staking_info_count,
            current_rewards_and_stakes_count,
        );

        Ok(())
    }
}

pub mod v3 {

    use super::*;
    use codec::{Decode, Encode};
    use frame_support::log;
    use frame_support::{
        storage::{
            child::KillStorageResult,
            generator::{StorageDoubleMap, StorageMap},
        },
        traits::Get,
        weights::Weight,
    };
    use sp_runtime::traits::{Saturating, Zero};
    use sp_std::collections::btree_map::BTreeMap;

    #[cfg(feature = "try-runtime")]
    use frame_support::traits::OnRuntimeUpgradeHelpersExt;

    #[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
    pub struct OldAccountLedger<Balance: AtLeast32BitUnsigned + Default + Copy> {
        #[codec(compact)]
        locked: Balance,
        unbonding_info: UnbondingInfo<Balance>,
    }

    #[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
    pub struct OldEraRewardAndStake<Balance: HasCompact> {
        #[codec(compact)]
        rewards: Balance,
        #[codec(compact)]
        staked: Balance,
    }

    #[derive(Clone, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo)]
    struct OldEraStakingPoints<AccountId: Ord, Balance: HasCompact> {
        #[codec(compact)]
        total: Balance,
        stakers: BTreeMap<AccountId, Balance>,
        #[codec(compact)]
        claimed_rewards: Balance,
    }

    /// Serves as migration state representation.
    /// E.g. we might be migrating `Ledger` but need to stop since we've reached the predefined weight limit.
    /// Therefore we use this enum to store migration state `MigrationState::Ledger(Some(last_processed_key))`.
    #[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, RuntimeDebug)]
    pub enum MigrationState {
        NotStarted,
        /// In the middle of `AccountLedger` migration.
        AccountLedger(Option<Vec<u8>>),
        /// In the middle of `GeneralEraInfo` migration.
        GeneralEraInfo(Option<Vec<u8>>),
        /// In the middle of `StakersInfo` migration. This requires more complex storage since it requires breaking
        /// of a BTreeMap struct into individual entries.
        StakersInfo(StakersInfoMigrationState),
        /// In the middle of `StakingInfo` migration.
        StakingInfo(Option<Vec<u8>>),
        /// In the middle of `RegisteredDapps` migration
        DAppInfo(Option<Vec<u8>>),
        /// In the middle of `EraRewardAndStake` storage removal
        EraRewardAndStake,
        Finished,
    }

    impl Default for MigrationState {
        fn default() -> Self {
            MigrationState::NotStarted
        }
    }

    #[derive(PartialEq, Eq, Clone, Default, Encode, Decode, RuntimeDebug, TypeInfo)]
    pub struct StakersInfoMigrationState {
        /// Last fully processed key. If `None` it means that first iter element hasn't been processed yet.
        last_processed_key: Option<Vec<u8>>,
        /// Number of processed items related to the current processed key.
        /// E.g. if last_processed_key is `None` and processed_items are 100, it means that 100 StakerInfo structs
        /// have been written to the DB but this particular contract hasn't been fully processed yet.
        processed_items: u32,
    }

    impl StakersInfoMigrationState {
        /// Returns key prefix iterator that takes into account which key was processed last.
        /// This is a convenience method.
        fn iter_keys<T: Config>(
            &self,
        ) -> frame_support::storage::KeyPrefixIterator<T::SmartContract> {
            if let Some(previous_key) = self.last_processed_key.clone() {
                RegisteredDapps::<T>::iter_keys_from(previous_key)
            } else {
                RegisteredDapps::<T>::iter_keys()
            }
        }
    }

    /// Returns appropriate staking points for a contract and current era.
    fn staking_points_and_era<T: Config>(
        contract_id: &T::SmartContract,
        current_era: EraIndex,
    ) -> (OldEraStakingPoints<T::AccountId, BalanceOf<T>>, EraIndex) {
        // The requirement for this upgrade is that ALL POSSIBLE eras have been claimed.
        // This means that there has to exist ContractEraStake entry for either current era era or the former era.
        let first_key = ContractEraStake::<T>::storage_double_map_final_key(
            (*contract_id).clone(),
            current_era,
        );
        let second_key = ContractEraStake::<T>::storage_double_map_final_key(
            (*contract_id).clone(),
            current_era - 1,
        );

        unhashed::get::<OldEraStakingPoints<T::AccountId, BalanceOf<T>>>(&first_key).map_or_else(
            || {
                // Panic is intentional to detect it during test. MUST be impossible to happen when upgrading production!
                (
                    unhashed::get::<OldEraStakingPoints<T::AccountId, BalanceOf<T>>>(&second_key)
                        .unwrap(),
                    current_era - 1,
                )
            },
            |v| (v, current_era),
        )
    }

    #[cfg(feature = "try-runtime")]
    pub fn pre_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        assert_eq!(Version::V2_0_0, StorageVersion::<T>::get());

        let ledger_count = Ledger::<T>::iter_keys().count() as u64;
        U::set_temp_storage::<u64>(ledger_count, "ledger_count");

        let staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        U::set_temp_storage(staking_info_count, "staking_info_count");

        let rewards_and_stakes_count = EraRewardsAndStakes::<T>::iter_keys().count() as u64;
        U::set_temp_storage(rewards_and_stakes_count, "rewards_and_stakes_count");

        log::info!(
            ">>> PreMigrate: ledger count: {:?}, staking info count: {:?}, General era info count: {:?}",
            ledger_count,
            staking_info_count,
            rewards_and_stakes_count,
        );

        Ok(().into())
    }

    pub fn stateful_migrate<T: Config>(weight_limit: Weight) -> Weight {
        // Ensure this is a valid migration for this version
        if StorageVersion::<T>::get() != Version::V2_0_0 {
            return T::DbWeight::get().reads(1);
        }

        log::info!("Executing a step of stateful storage migration.");

        let mut migration_state = MigrationStateV3::<T>::get();
        let mut consumed_weight = T::DbWeight::get().reads(2);

        //
        // 0
        //
        if migration_state == MigrationState::NotStarted {
            // Since we want to continue accumulating block rewards, we need to translate
            // the block reward accumulator immediately
            let _ = BlockRewardAccumulator::<T>::translate(|x: Option<BalanceOf<T>>| {
                if let Some(reward) = x {
                    let dapps_reward = T::DeveloperRewardPercentage::get() * reward;
                    let stakers_reward = reward.saturating_sub(dapps_reward);
                    Some(RewardInfo::<BalanceOf<T>> {
                        dapps: dapps_reward,
                        stakers: stakers_reward,
                    })
                } else {
                    None
                }
            });

            PalletDisabled::<T>::put(true);
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(2));

            migration_state = MigrationState::AccountLedger(None);

            // If normal run, just exit here to avoid the risk of clogging the upgrade block.
            if !cfg!(feature = "try-runtime") {
                MigrationStateV3::<T>::put(migration_state);
                consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));
                return consumed_weight;
            }
        }

        //
        // 1
        //
        if let MigrationState::AccountLedger(last_processed_key) = migration_state.clone() {
            // First, get correct iterator.
            let key_iter = if let Some(previous_key) = last_processed_key {
                Ledger::<T>::iter_keys_from(previous_key)
            } else {
                Ledger::<T>::iter_keys()
            };

            let mut total_locked = BalanceOf::<T>::zero();

            for key in key_iter {
                let key_as_vec = Ledger::<T>::storage_map_final_key(key);
                translate(&key_as_vec, |x: OldAccountLedger<BalanceOf<T>>| {
                    total_locked += x.unbonding_info.sum();
                    Some(AccountLedger {
                        locked: x.locked,
                        unbonding_info: x.unbonding_info,
                        reward_handling: Default::default(),
                    })
                });

                // Increment total consumed weight.
                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                // Check if we've consumed enough weight already.
                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> AccountLedger migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV3::<T>::put(MigrationState::AccountLedger(Some(key_as_vec)));
                    MigrationUndergoingUnbonding::<T>::mutate(|x| *x += total_locked);
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(2));

                    // we want try-runtime to execute the entire migration
                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> AccountLedger migration finished.");
            migration_state = MigrationState::GeneralEraInfo(None);
        }

        //
        // 2
        //
        if let MigrationState::GeneralEraInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                EraRewardsAndStakes::<T>::iter_keys_from(previous_key)
            } else {
                EraRewardsAndStakes::<T>::iter_keys()
            };

            for key in key_iter {
                let key_as_vec = EraRewardsAndStakes::<T>::storage_map_final_key(key);

                // Read value from old storage
                let reward_and_stake = EraRewardsAndStakes::<T>::get(&key).unwrap();
                let dapps_reward = T::DeveloperRewardPercentage::get() * reward_and_stake.rewards;

                GeneralEraInfo::<T>::insert(
                    key,
                    EraInfo::<BalanceOf<T>> {
                        rewards: RewardInfo::<BalanceOf<T>> {
                            stakers: reward_and_stake.rewards - dapps_reward,
                            dapps: dapps_reward,
                        },
                        staked: reward_and_stake.staked,
                        // This is obviously incorrect. However, it is very close to the actual truth.
                        // What is important is ensuring that LATEST era has the correct information.
                        // The solution should be to sum up all ubonding chunks while iterating over ledger
                        // and set this to 'value.staked - total_unbonding'
                        // Probably should be done AFTER migration has been finalized, then we just update the latest value.
                        locked: reward_and_stake.staked,
                    },
                );

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> GeneralEraInfo migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV3::<T>::put(MigrationState::GeneralEraInfo(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            // At this point, all `GeneralEraInfo` values have been migrated.
            // Update the latest era info to contain correct dApps staking TVL value
            let current_era = Pallet::<T>::current_era();
            let total_locked = MigrationUndergoingUnbonding::<T>::get();
            GeneralEraInfo::<T>::mutate(current_era, |value| {
                if let Some(x) = value {
                    x.locked = x.staked + total_locked;
                }
            });
            consumed_weight = consumed_weight
                .saturating_add(T::DbWeight::get().writes(1) + T::DbWeight::get().reads(3));

            log::info!(">>> GeneralEraInfo migration finished.");

            migration_state = MigrationState::StakersInfo(Default::default());
        }

        //
        // 3
        //
        if let MigrationState::StakersInfo(last_processed_info) = migration_state.clone() {
            let mut last_processed_info = last_processed_info;
            let key_iter = last_processed_info.iter_keys::<T>();

            let current_era = Pallet::<T>::current_era();
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(2));

            // The outer loop will process staking info per contract
            for contract_id in key_iter {
                let (staking_info, era_to_use) =
                    staking_points_and_era::<T>(&contract_id, current_era);

                // The inner loop will process each staker individually
                for (staker_id, staked_amount) in staking_info
                    .stakers
                    .iter()
                    .skip(last_processed_info.processed_items as usize)
                {
                    StakersInfo::<T>::insert(
                        &staker_id,
                        &contract_id,
                        StakerInfo::<BalanceOf<T>> {
                            stakes: vec![EraStake::<BalanceOf<T>> {
                                staked: *staked_amount,
                                era: era_to_use,
                            }],
                        },
                    );

                    // One additional item in the map has been processed
                    last_processed_info.processed_items += 1;
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if last_processed_info.processed_items == staking_info.stakers.len() as u32 {
                        // this means one contract has been fully processed
                        last_processed_info.last_processed_key = Some(
                            RegisteredDapps::<T>::storage_map_final_key(contract_id.clone()),
                        );
                        last_processed_info.processed_items = 0;
                    }

                    if consumed_weight >= weight_limit {
                        log::info!(
                            ">>> StakersInfo migration stopped after consuming {:?} weight.",
                            consumed_weight
                        );
                        MigrationStateV3::<T>::put(MigrationState::StakersInfo(
                            last_processed_info,
                        ));
                        consumed_weight =
                            consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                        if cfg!(feature = "try-runtime") {
                            return stateful_migrate::<T>(weight_limit);
                        } else {
                            return consumed_weight;
                        }
                    }
                }
            }

            log::info!(">>> StakerInfo migration finished.");
            migration_state = MigrationState::StakingInfo(None);
        }

        //
        // 4
        //
        if let MigrationState::StakingInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                ContractEraStake::<T>::iter_keys_from(previous_key)
            } else {
                ContractEraStake::<T>::iter_keys()
            };

            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));

            for (contract_id, era) in key_iter {
                let key_as_vec =
                    ContractEraStake::<T>::storage_double_map_final_key(contract_id, era);
                translate(
                    &key_as_vec,
                    |value: OldEraStakingPoints<T::AccountId, BalanceOf<T>>| {
                        Some(EraStakingPoints {
                            total: value.total,
                            number_of_stakers: value.stakers.len() as u32,
                            contract_reward_claimed: value.claimed_rewards > Zero::zero(),
                        })
                    },
                );

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> EraStakingPoints migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV3::<T>::put(MigrationState::StakingInfo(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> EraStakingPoints migration finished.");
            migration_state = MigrationState::DAppInfo(None);
        }

        //
        // 5
        //
        if let MigrationState::DAppInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                RegisteredDapps::<T>::iter_keys_from(previous_key)
            } else {
                RegisteredDapps::<T>::iter_keys()
            };

            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));

            for key in key_iter {
                let key_as_vec = RegisteredDapps::<T>::storage_map_final_key(key);
                translate(&key_as_vec, |value: T::AccountId| {
                    Some(DAppInfo::<T::AccountId> {
                        developer: value,
                        state: DAppState::Registered,
                    })
                });

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> DAppInfo migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV3::<T>::put(MigrationState::DAppInfo(Some(key_as_vec)));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> DAppInfo migration finished.");
            migration_state = MigrationState::EraRewardAndStake;

            MigrationStateV3::<T>::put(MigrationState::EraRewardAndStake);
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));
        }

        //
        // 6
        //
        if let MigrationState::EraRewardAndStake = migration_state {
            let remaining_weight = weight_limit - consumed_weight;

            // Just to be on the safe side, we reduce the number of ops.
            let adjusted_weight = remaining_weight * 9 / 10;
            let approximate_deletions_remaining = adjusted_weight / T::DbWeight::get().writes(1);
            let approximate_deletions_remaining = approximate_deletions_remaining.max(1);

            // Remove up to limited amount of entries from the DB
            let result =
                EraRewardsAndStakes::<T>::remove_all(Some(approximate_deletions_remaining as u32));

            consumed_weight = consumed_weight
                .saturating_add(approximate_deletions_remaining * T::DbWeight::get().writes(1));

            if let KillStorageResult::AllRemoved(num) = result {
                consumed_weight =
                    consumed_weight.saturating_add(num as u64 * T::DbWeight::get().writes(1));
            } else {
                consumed_weight = consumed_weight
                    .saturating_add(approximate_deletions_remaining * T::DbWeight::get().writes(1));
                log::info!(
                    ">>> EraRewardAndStake removal stopped after consuming {:?} weight.",
                    consumed_weight
                );
                if cfg!(feature = "try-runtime") {
                    return stateful_migrate::<T>(weight_limit);
                } else {
                    return consumed_weight;
                }
            };

            log::info!(">>> EraRewardAndStke removal finished.");
        }

        MigrationStateV3::<T>::put(MigrationState::Finished);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));
        log::info!(">>> Migration finalized.");

        StorageVersion::<T>::put(Version::V3_0_0);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

        PalletDisabled::<T>::put(false);
        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

        consumed_weight
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config, U: OnRuntimeUpgradeHelpersExt>() -> Result<(), &'static str> {
        assert_eq!(Version::V3_0_0, StorageVersion::<T>::get());

        let current_era = Pallet::<T>::current_era();
        let general_era_info = GeneralEraInfo::<T>::get(current_era).unwrap();
        let total_unlocking = Ledger::<T>::iter_values()
            .map(|x| x.unbonding_info.sum())
            .reduce(|c1, c2| c1 + c2)
            .unwrap();
        assert_eq!(
            general_era_info.locked,
            general_era_info.staked + total_unlocking
        );

        assert!(EraRewardsAndStakes::<T>::iter_keys().count().is_zero());

        let ledger_count = Ledger::<T>::iter_keys().count() as u64;
        U::set_temp_storage::<u64>(ledger_count, "ledger_count");

        let staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        U::set_temp_storage(staking_info_count, "staking_info_count");

        let rewards_and_stakes_count = GeneralEraInfo::<T>::iter_keys().count() as u64;
        U::set_temp_storage(rewards_and_stakes_count, "rewards_and_stakes_count");

        let stakers_info_count = StakersInfo::<T>::iter_keys().count() as u64;

        log::info!(
            ">>> PostMigrate: ledger count: {:?}, staking info count: {:?}, rewards&stakes count: {:?}, stakers info count: {:?}",
            ledger_count,
            staking_info_count,
            rewards_and_stakes_count,
            stakers_info_count,
        );

        Ok(())
    }
}
