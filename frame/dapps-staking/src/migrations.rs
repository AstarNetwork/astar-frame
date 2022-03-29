//! Dapps staking migration utility module

use super::*;
use codec::{Decode, FullCodec};
use frame_support::storage::unhashed;
use pallet::pallet::*;
use sp_runtime::Perbill;
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
        /// In the middle of `GeneralStakerInfo` migration. This requires more complex storage since it requires breaking
        /// of a BTreeMap struct into individual entries.
        GeneralStakerInfo(StakersInfoMigrationState),
        /// In the middle of `StakingInfo` migration.
        StakingInfo(Option<Vec<u8>>),
        /// In the middle of `RegisteredDapps` migration
        DAppInfo(Option<Vec<u8>>),
        /// In the middle of `EraRewardAndStake` storage removal
        EraRewardAndStake,
        /// In the middle of rotating contract stake info to make them available for current era
        ContractStakeInfoRotation(Option<Vec<u8>>),
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

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate_shibuya_fix_for_v3<T: Config>() -> Result<(), &'static str> {
        // Ensure that all necessary `ContractEraStake` entries exist.
        let current_era = Pallet::<T>::current_era();
        for contract_id in RegisteredDapps::<T>::iter_keys() {
            assert!(
                !ContractEraStake::<T>::get(&contract_id, current_era)
                    .unwrap()
                    .contract_reward_claimed
            );
            assert!(
                !ContractEraStake::<T>::get(&contract_id, current_era - 1)
                    .unwrap()
                    .contract_reward_claimed
            );
        }

        Ok(().into())
    }

    /// Used to fix the current inconsistent state we have on Shibuya.
    /// This code isn't reusable for other chains.
    pub fn shibuya_fix_for_v3<T: Config>() -> Weight {
        let current_era = Pallet::<T>::current_era();
        let previous_era = current_era - 1;

        let mut consumed_weight = 0;

        for contract_id in RegisteredDapps::<T>::iter_keys() {
            if let Some(mut contract_stake_info) =
                ContractEraStake::<T>::get(&contract_id, previous_era)
            {
                contract_stake_info.contract_reward_claimed = false;
                ContractEraStake::<T>::insert(&contract_id, current_era, contract_stake_info);

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
            }
        }

        consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));

        consumed_weight
    }

    pub fn stateful_migrate<T: Config>(weight_limit: Weight) -> Weight {
        // Ensure this is a valid migration for this version
        if StorageVersion::<T>::get() != Version::V2_0_0 {
            return T::DbWeight::get().reads(1);
        }

        log::info!("Executing a step of stateful storage migration.");
        const CONTRACT_ERA_STAKE_READ_LIMIT: u32 = 64;

        let mut migration_state = MigrationStateV3::<T>::get();
        let mut consumed_weight = T::DbWeight::get().reads(2);

        // Just a placeholder since old configurable constant has been removed
        let dev_reward_percentage = Perbill::from_percent(50);

        //
        // 0
        //
        if migration_state == MigrationState::NotStarted {
            // Since we want to continue accumulating block rewards, we need to translate
            // the block reward accumulator immediately
            let _ = BlockRewardAccumulator::<T>::translate(|x: Option<BalanceOf<T>>| {
                if let Some(reward) = x {
                    let dapps_reward = dev_reward_percentage * reward;
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
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));

            migration_state = MigrationState::DAppInfo(None);

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
        if let MigrationState::DAppInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                RegisteredDapps::<T>::iter_keys_from(previous_key)
            } else {
                RegisteredDapps::<T>::iter_keys()
            };

            for key in key_iter {
                let key_as_vec = RegisteredDapps::<T>::storage_map_final_key(key.clone());

                translate(&key_as_vec, |value: T::AccountId| {
                    let is_registered = RegisteredDevelopers::<T>::contains_key(&value);

                    // We no longer delete enry for RegisteredDevelopers, so we will restore this in case it was deleted before
                    if !is_registered {
                        RegisteredDevelopers::<T>::insert(&value, key.clone());
                        consumed_weight =
                            consumed_weight.saturating_add(T::DbWeight::get().writes(1));
                    }

                    Some(DAppInfo::<T::AccountId> {
                        developer: value,
                        state: if is_registered {
                            DAppState::Registered
                        } else {
                            DAppState::Unregistered(0)
                        },
                    })
                });

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(3, 1));

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
            migration_state = MigrationState::AccountLedger(None);
        }

        //
        // 2
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
                        reward_destination: Default::default(),
                    })
                });

                // Increment total consumed weight.
                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));

                // Check if we've consumed enough weight already.
                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> AccountLedger migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );
                    MigrationStateV3::<T>::put(MigrationState::AccountLedger(Some(key_as_vec)));
                    MigrationUndergoingUnbonding::<T>::mutate(|x| *x += total_locked);
                    consumed_weight =
                        consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));

                    // we want try-runtime to execute the entire migration
                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            MigrationUndergoingUnbonding::<T>::mutate(|x| *x += total_locked);
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

            log::info!(">>> AccountLedger migration finished.");
            migration_state = MigrationState::GeneralEraInfo(None);
        }

        //
        // 3
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
                let dapps_reward = dev_reward_percentage * reward_and_stake.rewards;

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
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));

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
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads_writes(3, 1));

            log::info!(">>> GeneralEraInfo migration finished.");

            migration_state = MigrationState::GeneralStakerInfo(Default::default());
        }

        //
        // 4
        //
        if let MigrationState::GeneralStakerInfo(last_processed_info) = migration_state.clone() {
            let mut last_processed_info = last_processed_info;
            let key_iter = last_processed_info.iter_keys::<T>();

            let current_era = Pallet::<T>::current_era();
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));

            // `ContractEraStake` can be huge in size so to prevent reading too much these entries
            // in a single block, we limit it.
            let mut contract_era_stake_limiter = CONTRACT_ERA_STAKE_READ_LIMIT;

            // The outer loop will process staking info per contract
            for contract_id in key_iter {
                // skip unregistered dapps
                if let Some(dapp_info) = RegisteredDapps::<T>::get(&contract_id) {
                    if let DAppState::Unregistered(_) = dapp_info.state {
                        continue;
                    }
                }

                let (staking_info, era_to_use) =
                    staking_points_and_era::<T>(&contract_id, current_era);
                let is_claimed = staking_info.claimed_rewards > Zero::zero();

                // The inner loop will process each staker individually
                for (staker_id, staked_amount) in staking_info
                    .stakers
                    .iter()
                    .skip(last_processed_info.processed_items as usize)
                {
                    GeneralStakerInfo::<T>::insert(
                        &staker_id,
                        &contract_id,
                        StakerInfo::<BalanceOf<T>> {
                            stakes: vec![EraStake::<BalanceOf<T>> {
                                staked: *staked_amount,
                                era: if is_claimed {
                                    era_to_use + 1
                                } else {
                                    era_to_use
                                },
                            }],
                        },
                    );

                    // One additional item in the map has been processed
                    last_processed_info.processed_items += 1;
                    consumed_weight =
                        consumed_weight.saturating_add(T::DbWeight::get().reads_writes(3, 1));

                    if last_processed_info.processed_items == staking_info.stakers.len() as u32 {
                        // this means one contract has been fully processed
                        last_processed_info.last_processed_key = Some(
                            RegisteredDapps::<T>::storage_map_final_key(contract_id.clone()),
                        );
                        last_processed_info.processed_items = 0;
                        contract_era_stake_limiter = contract_era_stake_limiter.saturating_sub(1);
                    }

                    if consumed_weight >= weight_limit || contract_era_stake_limiter.is_zero() {
                        log::info!(
                            ">>> GeneralStakerInfo migration stopped after consuming {:?} weight.",
                            consumed_weight
                        );
                        MigrationStateV3::<T>::put(MigrationState::GeneralStakerInfo(
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
        // 5
        //
        if let MigrationState::StakingInfo(last_processed_key) = migration_state.clone() {
            let key_iter = if let Some(previous_key) = last_processed_key {
                ContractEraStake::<T>::iter_keys_from(previous_key)
            } else {
                ContractEraStake::<T>::iter_keys()
            };

            // `ContractEraStake` can be huge in size so to prevent reading too much these entries
            // in a single block, we limit it.
            let mut contract_era_stake_limiter = CONTRACT_ERA_STAKE_READ_LIMIT;

            for (contract_id, era) in key_iter {
                let key_as_vec =
                    ContractEraStake::<T>::storage_double_map_final_key(contract_id, era);
                translate(
                    &key_as_vec,
                    |value: OldEraStakingPoints<T::AccountId, BalanceOf<T>>| {
                        Some(ContractStakeInfo {
                            total: value.total,
                            number_of_stakers: value.stakers.len() as u32,
                            contract_reward_claimed: value.claimed_rewards > Zero::zero(),
                        })
                    },
                );

                contract_era_stake_limiter = contract_era_stake_limiter.saturating_sub(1);

                consumed_weight =
                    consumed_weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));

                if consumed_weight >= weight_limit || contract_era_stake_limiter.is_zero() {
                    log::info!(
                        ">>> ContractStakeInfo migration stopped after consuming {:?} weight.",
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

            log::info!(">>> ContractStakeInfo migration finished.");
            migration_state = MigrationState::EraRewardAndStake;
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

            consumed_weight = consumed_weight.saturating_add(
                approximate_deletions_remaining.saturating_mul(T::DbWeight::get().writes(1)),
            );

            if let KillStorageResult::AllRemoved(num) = result {
                consumed_weight = consumed_weight
                    .saturating_add((num as u64).saturating_mul(T::DbWeight::get().writes(1)));
            } else {
                consumed_weight = consumed_weight.saturating_add(
                    approximate_deletions_remaining.saturating_mul(T::DbWeight::get().writes(1)),
                );
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
            migration_state = MigrationState::ContractStakeInfoRotation(None);
        }

        //
        // 7
        //
        if let MigrationState::ContractStakeInfoRotation(last_processed_key) =
            migration_state.clone()
        {
            let key_iter = if let Some(previous_key) = last_processed_key {
                RegisteredDapps::<T>::iter_keys_from(previous_key)
            } else {
                RegisteredDapps::<T>::iter_keys()
            };

            let current_era = Pallet::<T>::current_era();
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));

            for key in key_iter {
                // We need to ensure that current era has a copy of all eligible contract staking infos.
                // Even though it's impossible for current era to be claimed, total staked amount could have been modified
                // so we need to account for that.
                if !ContractEraStake::<T>::contains_key(&key, current_era) {
                    // Since we ensure all rewards have been claimed, this will always be true
                    if let Some(mut contract_stake_info) =
                        ContractEraStake::<T>::get(&key, current_era - 1)
                    {
                        contract_stake_info.contract_reward_claimed = false;
                        ContractEraStake::<T>::insert(&key, current_era, contract_stake_info);
                        consumed_weight =
                            consumed_weight.saturating_add(T::DbWeight::get().reads_writes(3, 1));
                    }
                } else {
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(2));
                }

                if consumed_weight >= weight_limit {
                    log::info!(
                        ">>> ContractStakeInfoRotation migration stopped after consuming {:?} weight.",
                        consumed_weight
                    );

                    let key_as_vec = RegisteredDapps::<T>::storage_map_final_key(key);
                    MigrationStateV3::<T>::put(MigrationState::ContractStakeInfoRotation(Some(
                        key_as_vec,
                    )));
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));

                    if cfg!(feature = "try-runtime") {
                        return stateful_migrate::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }

            log::info!(">>> DAppInfo migration finished.");
        }

        // Since enum was modified, we want to avoid corrupt data decoding
        ForceEra::<T>::put(Forcing::NotForcing);

        MigrationStateV3::<T>::put(MigrationState::Finished);
        StorageVersion::<T>::put(Version::V3_0_0);
        PalletDisabled::<T>::put(false);
        log::info!(">>> Migration finalized.");

        consumed_weight.saturating_add(T::DbWeight::get().writes(4))
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

        // Ensure that all necessary `ContractEraStake` entries exist.
        let current_era = Pallet::<T>::current_era();
        for (contract_id, dapp_info) in RegisteredDapps::<T>::iter() {
            if let DAppState::Unregistered(_) = dapp_info.state {
                continue;
            }

            assert!(
                !ContractEraStake::<T>::get(&contract_id, current_era)
                    .unwrap()
                    .contract_reward_claimed
            );
            assert!(
                ContractEraStake::<T>::get(&contract_id, current_era - 1)
                    .unwrap()
                    .contract_reward_claimed
            );
        }

        // Ensure that all dapps have the inverse map mapping for dev (even unregistered contracts)
        for (contract_id, dapp_info) in RegisteredDapps::<T>::iter() {
            assert_eq!(
                RegisteredDevelopers::<T>::get(dapp_info.developer).unwrap(),
                contract_id
            );
        }

        // Ensure that all staker info is as expected
        for staker_id in Ledger::<T>::iter_keys() {
            for (_contract_id, mut staker_info) in GeneralStakerInfo::<T>::iter_prefix(staker_id) {
                let (last_staked_era, _staked) = staker_info.claim();
                // MUST be true since we do the upgrade IFF all pending rewards have been claimed
                assert_eq!(last_staked_era, current_era);
            }
        }

        let ledger_count = Ledger::<T>::iter_keys().count() as u64;
        U::set_temp_storage::<u64>(ledger_count, "ledger_count");

        let staking_info_count = ContractEraStake::<T>::iter_keys().count() as u64;
        U::set_temp_storage(staking_info_count, "staking_info_count");

        let rewards_and_stakes_count = GeneralEraInfo::<T>::iter_keys().count() as u64;
        U::set_temp_storage(rewards_and_stakes_count, "rewards_and_stakes_count");

        let stakers_info_count = GeneralStakerInfo::<T>::iter_keys().count() as u64;

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

pub mod festival_end {

    use super::*;
    use frame_support::{log, storage::child::KillStorageResult, traits::Get, weights::Weight};
    use sp_runtime::traits::Zero;

    #[cfg(feature = "try-runtime")]
    pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
        assert!(Ledger::<T>::iter().count().is_zero());
        assert!(BlockRewardAccumulator::<T>::exists());

        // Sanity check for pallet free balance. UT covers this but not via live chain check.
        let halved_unit: BalanceOf<T> = 1_000_000_000_u32.into();
        let unit = halved_unit * halved_unit;
        let threshold_balance = unit * 25_000_000_u32.into();

        assert!(T::Currency::free_balance(&Pallet::<T>::account_id()) > threshold_balance);

        Ok(())
    }

    /// Should be executed during runtime upgrade
    /// Won't do the entire cleanup, instead it will just do necessary preparatory actions.
    pub fn on_runtime_upgrade<T: Config>(weight_limit: Weight) -> Weight {
        if Ledger::<T>::iter_keys().peekable().peek().is_some() {
            log::warn!(">>> There are existing account ledgers. This upgrade is invalid!");
            return T::DbWeight::get().reads(1);
        }

        // 1. Accumulated rewards for the ongoing era are killed - however, funds will remain in pallet account
        BlockRewardAccumulator::<T>::kill();

        // 2. To ensure we don't have errors with decoding
        ForceEra::<T>::put(Forcing::NotForcing);

        // 3. Sanity check, should be entirely empty already
        RegisteredDevelopers::<T>::remove_all(None);

        // 4. Reset eras since we're doing a clean start.
        CurrentEra::<T>::put(0);

        // 5. Maintenance mode so we can keep track of cleanup progress
        PalletDisabled::<T>::put(true);

        let consumed_weight = T::DbWeight::get().reads_writes(1, 2);
        if cfg!(feature = "try-runtime") {
            consumed_weight + storage_cleanup::<T>(weight_limit)
        } else {
            consumed_weight
        }
    }

    /// Used to cleanup leftover storage items from legacy dapps-staking
    ///
    /// `weight-limit` - used to limit how many entries can be deleted using a single call
    ///
    pub fn storage_cleanup<T: Config>(weight_limit: Weight) -> Weight {
        if Ledger::<T>::iter_keys().peekable().peek().is_some() {
            log::warn!(">>> There are existing account ledgers, cannot do cleanup.");
            return T::DbWeight::get().reads(1);
        }
        if !CurrentEra::<T>::get().is_zero() || !PalletDisabled::<T>::get() {
            log::warn!(">>> Pallet should be in maintenance mode and current era should be zero while doing storage cleanup.");
            return T::DbWeight::get().reads(3);
        }

        log::info!(">>> Executing a step of staking festival storage cleanup.");

        let mut consumed_weight = Zero::zero();

        let deletion_weight = T::DbWeight::get().writes(1) * 11 / 10;
        let mut approximate_deletions_remaining =
            (weight_limit / (T::DbWeight::get().writes(1) * 11 / 10)).max(1);

        //
        // 1
        //
        if RegisteredDapps::<T>::iter_keys()
            .peekable()
            .peek()
            .is_some()
        {
            let removal_result = if cfg!(feature = "try-runtime") {
                RegisteredDapps::<T>::remove_all(None)
            } else {
                RegisteredDapps::<T>::remove_all(Some(approximate_deletions_remaining as u32))
            };
            match removal_result {
                KillStorageResult::AllRemoved(removed_entries_num) => {
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    approximate_deletions_remaining -= removed_entries_num as u64;
                    log::info!(">>> RegisteredDapps cleanup finished.");
                }
                KillStorageResult::SomeRemaining(removed_entries_num) => {
                    log::info!(
                    ">>> RegisteredDapps cleanup stopped due to reaching max amount of deletions."
                );
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    if cfg!(feature = "try-runtime") {
                        return consumed_weight + storage_cleanup::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }
        }

        //
        // 2
        //
        if PreApprovedDevelopers::<T>::iter_keys()
            .peekable()
            .peek()
            .is_some()
        {
            let removal_result = if cfg!(feature = "try-runtime") {
                PreApprovedDevelopers::<T>::remove_all(None)
            } else {
                PreApprovedDevelopers::<T>::remove_all(Some(approximate_deletions_remaining as u32))
            };
            match removal_result {
                KillStorageResult::AllRemoved(removed_entries_num) => {
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    approximate_deletions_remaining -= removed_entries_num as u64;
                    log::info!(">>> PreApprovedDevelopers cleanup finished.");
                }
                KillStorageResult::SomeRemaining(removed_entries_num) => {
                    log::info!(">>> PreApprovedDevelopers cleanup stopped due to reaching max amount of deletions.");
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    if cfg!(feature = "try-runtime") {
                        return consumed_weight + storage_cleanup::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }
        }

        //
        // 3
        //
        if EraRewardsAndStakes::<T>::iter_keys()
            .peekable()
            .peek()
            .is_some()
        {
            let removal_result = if cfg!(feature = "try-runtime") {
                EraRewardsAndStakes::<T>::remove_all(None)
            } else {
                EraRewardsAndStakes::<T>::remove_all(Some(approximate_deletions_remaining as u32))
            };
            match removal_result {
                KillStorageResult::AllRemoved(removed_entries_num) => {
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    approximate_deletions_remaining -= removed_entries_num as u64;
                    log::info!(">>> EraRewardsAndStakes cleanup finished.");
                }
                KillStorageResult::SomeRemaining(removed_entries_num) => {
                    log::info!(">>> EraRewardsAndStakes cleanup stopped due to reaching max amount of deletions.");
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    if cfg!(feature = "try-runtime") {
                        return consumed_weight + storage_cleanup::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }
        }

        //
        // 4
        //
        // We limit this in order to reduce PoV size
        approximate_deletions_remaining = approximate_deletions_remaining.min(128);
        if ContractEraStake::<T>::iter_keys()
            .peekable()
            .peek()
            .is_some()
        {
            // `remove_all` cannot be called multiple times in the same block since it won't produce different results
            let removal_result = if cfg!(feature = "try-runtime") {
                ContractEraStake::<T>::remove_all(None)
            } else {
                ContractEraStake::<T>::remove_all(Some(approximate_deletions_remaining as u32))
            };
            match removal_result {
                KillStorageResult::AllRemoved(removed_entries_num) => {
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    log::info!(">>> ContractEraStake cleanup finished.");
                }
                KillStorageResult::SomeRemaining(removed_entries_num) => {
                    log::info!(
                    ">>> ContractEraStake cleanup stopped due to reaching max amount of deletions."
                );
                    consumed_weight += deletion_weight * removed_entries_num as u64;
                    if cfg!(feature = "try-runtime") {
                        return consumed_weight + storage_cleanup::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }
        }

        log::info!(">>> Storage cleanup finished.");
        // Disable maintenance mode
        PalletDisabled::<T>::put(false);
        consumed_weight += T::DbWeight::get().writes(1);

        consumed_weight
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
        assert!(Ledger::<T>::iter().count().is_zero());
        assert!(CurrentEra::<T>::get().is_zero());
        assert!(ForceEra::<T>::exists());
        assert_eq!(ForceEra::<T>::get(), Forcing::NotForcing);

        assert!(!PalletDisabled::<T>::get());

        assert!(RegisteredDapps::<T>::iter().count().is_zero());
        assert!(RegisteredDevelopers::<T>::iter().count().is_zero());
        assert!(EraRewardsAndStakes::<T>::iter().count().is_zero());
        assert!(ContractEraStake::<T>::iter().count().is_zero());
        assert!(PreApprovedDevelopers::<T>::iter().count().is_zero());

        Ok(())
    }
}
