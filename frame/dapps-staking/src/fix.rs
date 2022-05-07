use super::*;
use codec::Decode;

pub mod restake_fix {

    use super::*;
    use codec::Encode;
    use frame_support::log;
    use frame_support::{
        storage::generator::StorageMap, storage::PrefixIterator, traits::Get, weights::Weight,
    };
    use sp_runtime::traits::Saturating;
    use sp_std::collections::btree_map::BTreeMap;

    // Temp struct of corrected ContractStakeInfo records with progress data
    #[derive(Clone, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
    pub struct RestakeFix<Balance: HasCompact> {
        // store last processed staker from Ledger
        last_fully_processed_staker: Option<Vec<u8>>,
        // store last processed staker's dapp
        last_processed_staker_contract: Option<Vec<u8>>,
        // accumulate ContractStakeInfo by contract address
        contract_staking_info: BTreeMap<Vec<u8>, ContractStakeInfo<Balance>>,
        // flip to true when all Ledger data has been processed
        all_stakers_processed: bool,
    }

    pub fn restake_fix_migration<T: Config>(weight_limit: Weight) -> Weight {
        log::info!("Executing a step of restake fix migration.");
        let mut restake_fix = RestakeFixAccumulator::<T>::get();
        // one read for RestakeFixAccumulator
        let mut consumed_weight = T::DbWeight::get().reads(1);
        // still stakers left to process
        if !restake_fix.all_stakers_processed {
            log::info!("Accumulating staker info.");
            // take iter from start or last processed
            let staker_iter = if let Some(last_processed_staker) =
                restake_fix.last_fully_processed_staker.clone()
            {
                Ledger::<T>::iter_keys_from(last_processed_staker)
            } else {
                Ledger::<T>::iter_keys()
            };
            // star iterating, label to be able to break out from inner loop
            'staker_iter: for staker in staker_iter {
                // every iteration adds one more read
                consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));
                // take iter for inner loop from start or last processed contract
                let contract_iter = if let Some(last_processed_contract) =
                    restake_fix.last_processed_staker_contract.clone()
                {
                    GeneralStakerInfo::<T>::iter_prefix_from(&staker, last_processed_contract)
                } else {
                    GeneralStakerInfo::<T>::iter_prefix(&staker)
                };
                for (contract, staking_info) in contract_iter {
                    log::info!(
                        "Extracting contract stake data for staker address: {:?}.",
                        staker
                    );
                    // every inner iteration also adds one read
                    consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));
                    let staked_value = staking_info.latest_staked_value();
                    // get contract address as Vec<u8>
                    let contract_address = PrefixIterator::last_raw_key(&contract_iter);
                    // select staking_info for the contract
                    let mut contract_stake_info = restake_fix
                        .contract_staking_info
                        .entry(contract_address.clone().to_vec())
                        .or_default();

                    // update total and number of stakers
                    contract_stake_info.total =
                        contract_stake_info.total.saturating_add(staked_value);
                    contract_stake_info.number_of_stakers += 1;
                    // check for total consumed weight
                    // store last processed contract and break out if the limit is reached
                    if consumed_weight >= weight_limit {
                        log::info!(
                            ">>> Ledger migration stopped after consuming {:?} weight.",
                            consumed_weight
                        );
                        restake_fix.last_processed_staker_contract =
                            Some(contract_address.clone().to_vec());
                        break 'staker_iter;
                    }
                }

                // when one staker is processed, store it and clear stored contract
                restake_fix.last_processed_staker_contract = None;
                restake_fix.last_fully_processed_staker =
                    Some(Ledger::<T>::storage_map_final_key(staker));
            }

            // store updated accumulator and update total weight for that write
            RestakeFixAccumulator::<T>::put(restake_fix);
            consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().writes(1));
        } else {
            log::info!(">>> Starting to empty the accumulator");
            // if true
            // if contractStakeInfo is empty, we're done
            // for each ContractStakeInfo in RestakeFix
            // write to ContractEraStake until weight hits limit
            // delete used records
        }

        consumed_weight
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
        // Pallet should be enabled after we finish
        assert!(PalletDisabled::<T>::get());

        // TODO: add check that storage for migration stuff was cleaned up

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
