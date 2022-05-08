use super::*;
use pallet::pallet::*;

pub mod restake {

    use super::*;
    use codec::{Encode, Decode};
    use frame_support::log;
    use frame_support::{
        storage::generator::StorageMap,
        traits::Get,
        weights::Weight,
    };
    use sp_std::{vec::Vec, collections::btree_map::BTreeMap};

    // Temp struct of corrected ContractStakeInfo records with progress data
    #[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
    pub struct RestakeFix<Balance: HasCompact> {
        // store progress so iteration can continue in the next block
        last_processed_staker: Option<Vec<u8>>,
        contract_staking_info: BTreeMap<Vec<u8>, ContractStakeInfo<Balance>>,
        // should be flipped once the process is complete
        all_stakers_processed: bool,
    }

    pub fn restake_fix_migration<T: Config>(weight_limit: Weight) -> Weight {
        let mut restake_fix = RestakeFixAccumulator::<T>::get();
        let mut consumed_weight = T::DbWeight::get().reads_writes(1, 1);

        if !restake_fix.all_stakers_processed {
            // read ledger from last_processed_staker or first if None
            let staker_iter = if let Some(x) = restake_fix.last_processed_staker {
                Ledger::<T>::iter_keys_from(x)
            } else {
                Ledger::<T>::iter_keys()
            };

            // We always process the staker entirely
            for staker in staker_iter {
                consumed_weight += T::DbWeight::get().reads(1);

                // Process all stakes related to staker, even though we might overshoot the weight limit a bit
                for (contract, staking_info) in GeneralStakerInfo::<T>::iter_prefix(&staker) {
                    consumed_weight += T::DbWeight::get().reads(1);

                    let staked_value = staking_info.latest_staked_value();

                    let entry = restake_fix.contract_staking_info.entry(contract.encode()).or_default();
                    entry.total += staked_value;
                    entry.number_of_stakers += 1;
                }

                if consumed_weight >= weight_limit {
                    let last_processed_key = Ledger::<T>::storage_map_final_key(staker);
                    restake_fix.last_processed_staker = Some(last_processed_key);
                    RestakeFixAccumulator::<T>::put(restake_fix);

                    log::info!(
                        ">>> Re-stake fix stopped after consuming {:?} weight.",
                        consumed_weight
                    );

                    if cfg!(feature = "try-runtime") {
                        return restake_fix_migration::<T>(weight_limit);
                    } else {
                        return consumed_weight;
                    }
                }
            }
        }

        // At this point, all data has been read
        restake_fix.all_stakers_processed = true;

        let current_era = Pallet::<T>::current_era();
        consumed_weight += T::DbWeight::get().reads(1);


        // Need to create a copy of this data since we also need to delete
        let raw_contracts = restake_fix.contract_staking_info
            .keys().map(|x| (*x).clone()).collect::<Vec<_>>();
        
        for raw_contract in raw_contracts.iter() {
            let contract = T::SmartContract::decode(&mut &raw_contract[..]).unwrap();

            // Will always execute
            if let Some(info) = restake_fix.contract_staking_info.remove(raw_contract) {
                ContractEraStake::<T>::insert(contract, current_era, info);
                consumed_weight += T::DbWeight::get().writes(1);
            } else {
                log::error!("Raw key not found when iterating `restake fix` map! Should be impossible");
            }

            if consumed_weight >= weight_limit {
                // map is being drained
                restake_fix.last_processed_staker = None;
                RestakeFixAccumulator::<T>::put(restake_fix);
                log::info!(
                    ">>> Re-stake fix stopped after consuming {:?} weight.",
                    consumed_weight
                );

                if cfg!(feature = "try-runtime") {
                    return restake_fix_migration::<T>(weight_limit);
                } else {
                    return consumed_weight;
                }
            }
        }

        RestakeFixAccumulator::<T>::kill();

        consumed_weight
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
        // Pallet should be enabled after we finish
        assert!(PalletDisabled::<T>::get());

        assert!(!RestakeFixAccumulator::<T>::exists());

        let current_era = Pallet::<T>::current_era();
        let general_era_info = GeneralEraInfo::<T>::get(current_era).unwrap();

        let mut restake_fix: BTreeMap<Vec<u8>, ContractStakeInfo<BalanceOf<T>>> =
            Default::default();

        // Construct the expected storage state
        for staker in Ledger::<T>::iter_keys() {
            for (contract_id, staking_info) in GeneralStakerInfo::<T>::iter_prefix(staker) {
                let staked_value = staking_info.latest_staked_value();

                let entry = restake_fix.entry(contract_id.encode()).or_default();
                entry.total += staked_value;
                entry.number_of_stakers += 1;
            }
        }

        // Verify that current state matches the expected(constructed) state
        let mut total_staked_sum = Zero::zero();
        for (contract_id, dapp_info) in RegisteredDapps::<T>::iter() {
            if let DAppState::Unregistered(_) = dapp_info.state {
                continue;
            }

            let on_chain_contract_staking_info =
                ContractEraStake::<T>::get(&contract_id, current_era).unwrap();
            assert_eq!(
                restake_fix[&contract_id.encode()],
                on_chain_contract_staking_info
            );

            total_staked_sum += on_chain_contract_staking_info.total;
        }

        // Sanity check for the sum
        assert_eq!(general_era_info.staked, total_staked_sum);

        Ok(())
    }
}
