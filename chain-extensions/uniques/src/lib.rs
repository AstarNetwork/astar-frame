#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;

use chain_extension_trait::ChainExtensionExec;

use frame_support::log;
// use frame_support::{traits::Get, pallet_prelude::{Weight}};
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    Environment, Ext, InitState, RetVal, RetVal::Converging, SysConfig, UncheckedFrom,
};
use pallet_uniques::WeightInfo;
use sp_std::marker::PhantomData;
use uniques_chain_extension_types::{UniquesError, UniquesFunc};

pub struct UniquesExtension<R>(PhantomData<R>);

impl<T: pallet_uniques::Config> ChainExtensionExec<T> for UniquesExtension<T> {
    fn execute_func<E>(
        func_id: u32,
        env: Environment<E, InitState>,
    ) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let func_id = UniquesFunc::try_from(func_id)?;

        match func_id {
            // READ functions
            // UniquesFunc::NextCollectionId => {

            //     let weight_to_charge = T::DbWeight::get().reads(1 as Weight);
            //     env.charge_weight(weight_to_charge)?;

            //     let next_collection_id = pallet_uniques::NextCollectionId::<T, _>::get();
            //     log::trace!(target: "runtime",
            //         "[UniquesExtension] NextCollectionId() {:?}",
            //         next_collection_id
            //     );
            //     env.write(&next_collection_id.encode(), false, None)?;
            // }
            UniquesFunc::CollectionDetails => {
                // let mut env = env.buf_in_buf_out();
                // let index = pallet_uniques::Pallet::<T>::collection_index();
                // let index_encoded = index.encode();

                // env.write(&index_encoded, false, None).map_err(|_| {
                //     DispatchError::Other("RMRK chain Extension failed to write collection_index")
                // })?;

                log::trace!(target: "runtime",
                    "[UniquesExtension] CollectionDetails() UnImplemented");
                return Err(DispatchError::Other("UnImplemented"));
            }

            // WRITE functions
            UniquesFunc::Create => {
                log::trace!(target: "runtime", "[UniquesExtension] create() initiating");
                let mut env = env.buf_in_buf_out();
                // let (collection_id): (T::CollectionId) = env.read_as()?;
                let contract = env.ext().address().clone();
                let caller = env.ext().caller().clone();

                let weight_to_charge = <T as pallet_uniques::Config>::WeightInfo::create();
                env.charge_weight(weight_to_charge)?;

                log::trace!(target: "runtime",
                    "[UniquesExtension] create() call owner {:?}, weight {:?}, admin {:?}",
                    contract, weight_to_charge, caller
                );
                let result = pallet_uniques::Pallet::<T>::create(
                    RawOrigin::Signed(contract.clone()).into(),
                    <T::Lookup as StaticLookup>::unlookup(contract), // this contract will be admin/issuer
                );
                // let result = pallet_uniques::Pallet::<T>::do_create_collection(
                //     // collection_id,
                //     contract.clone(),
                //     caller.clone(), // admin
                //     T::CollectionDeposit::get(),
                //     false,
                //     pallet_uniques::Event::Created {
                //         collection: collection_id,
                //         creator: issuer.clone(),
                //         owner: issuer.clone(),
                //     },
                // ); // TODO use ? here after removing debug code below

                log::trace!(target: "runtime",
                    "[UniquesExtension] create() result {:?}",
                    result
                );

                return match result {
                    Ok(_) => Ok(Converging(UniquesError::Success as u32)),
                    Err(e) => {
                        let mapped_error = UniquesError::try_from(e)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                };
            }

            UniquesFunc::Mint => {
                log::trace!(target: "runtime", "[UniquesExtension] mint() initiating");
                let mut env = env.buf_in_buf_out();
                let (collection_id, item_id, mint_to): (T::CollectionId, T::ItemId, T::AccountId) =
                    env.read_as()?;
                let contract = env.ext().address().clone();

                let weight_to_charge = <T as pallet_uniques::Config>::WeightInfo::mint();
                env.charge_weight(weight_to_charge)?;

                log::trace!(target: "runtime",
                    "[UniquesExtension] mint() col_owner {:?} \nmint_to {:?} \ncollection_id {:?}, item_id {:?} weight {:?}",
                    contract, mint_to,
                    collection_id, item_id, weight_to_charge
                );
                let result = pallet_uniques::Pallet::<T>::mint(
                    RawOrigin::Signed(contract.clone()).into(), // collection owner is this contract
                    collection_id,                              // collection_id for this contrat
                    item_id,                                    // item_id to be minted
                    <T::Lookup as StaticLookup>::unlookup(mint_to), // new owner of the item
                );

                log::trace!(target: "runtime",
                    "[UniquesExtension] mint() result {:?}",
                    result
                );

                return match result {
                    Ok(_) => Ok(Converging(UniquesError::Success as u32)),
                    Err(e) => {
                        let mapped_error = UniquesError::try_from(e)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                };
            }

            UniquesFunc::Transfer => {
                log::trace!(target: "runtime", "[UniquesExtension] transfer() initiating");
                let mut env = env.buf_in_buf_out();
                let (collection_id, item_id, to): (T::CollectionId, T::ItemId, T::AccountId) =
                    env.read_as()?;
                // in case of transfer we need caller's address as origin
                let caller = env.ext().caller().clone();
                let weight_to_charge = <T as pallet_uniques::Config>::WeightInfo::transfer();
                env.charge_weight(weight_to_charge)?;

                log::trace!(target: "runtime",
                    "[UniquesExtension] transfer() caller {:?} \to {:?} \ncollection_id {:?}, item_id {:?} weight {:?}",
                    caller, to,
                    collection_id, item_id, weight_to_charge
                );
                let result = pallet_uniques::Pallet::<T>::transfer(
                    RawOrigin::Signed(caller.clone()).into(), // current item owner
                    collection_id,                            // collection_id
                    item_id,                                  // item_id to be transfered
                    <T::Lookup as StaticLookup>::unlookup(to), // new owner of the item
                );

                log::trace!(target: "runtime",
                    "[UniquesExtension] transfer() result {:?}",
                    result
                );

                return match result {
                    Ok(_) => Ok(Converging(UniquesError::Success as u32)),
                    Err(e) => {
                        let mapped_error = UniquesError::try_from(e)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                };
            }

            UniquesFunc::ApproveTransfer => {
                log::trace!(target: "runtime", "[UniquesExtension] approve() initiating");
                let mut env = env.buf_in_buf_out();
                let (collection_id, item_id, operator): (T::CollectionId, T::ItemId, T::AccountId) =
                    env.read_as()?;
                // in case of approve we need caller's address as origin
                let caller = env.ext().caller().clone();
                let weight_to_charge =
                    <T as pallet_uniques::Config>::WeightInfo::approve_transfer();
                env.charge_weight(weight_to_charge)?;

                log::trace!(target: "runtime",
                    "[UniquesExtension] approve() caller {:?} \noperator {:?} \ncollection_id {:?}, item_id {:?} weight {:?}",
                    caller, operator,
                    collection_id, item_id, weight_to_charge
                );
                let result = pallet_uniques::Pallet::<T>::approve_transfer(
                    RawOrigin::Signed(caller.clone()).into(), // current item owner
                    collection_id,                            // collection_id
                    item_id,                                  // item_id to be approved
                    <T::Lookup as StaticLookup>::unlookup(operator), // new operator of the item
                );

                log::trace!(target: "runtime",
                    "[UniquesExtension] approve() result {:?}",
                    result
                );

                return match result {
                    Ok(_) => Ok(Converging(UniquesError::Success as u32)),
                    Err(e) => {
                        let mapped_error = UniquesError::try_from(e)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                };
            }

            UniquesFunc::CancelApproval => {
                log::trace!(target: "runtime", "[UniquesExtension] approve() initiating");
                let mut env = env.buf_in_buf_out();
                let (collection_id, item_id, operator): (T::CollectionId, T::ItemId, T::AccountId) =
                    env.read_as()?;
                // in case of approve we need caller's address as origin
                // let contract = env.ext().address().clone();
                let caller = env.ext().caller().clone();
                let weight_to_charge = <T as pallet_uniques::Config>::WeightInfo::cancel_approval();
                env.charge_weight(weight_to_charge)?;

                log::trace!(target: "runtime",
                    "[UniquesExtension] cancel approval() caller {:?} \noperator {:?} \ncollection_id {:?}, item_id {:?} weight {:?}",
                    caller, operator,
                    collection_id, item_id, weight_to_charge
                );
                let result = pallet_uniques::Pallet::<T>::cancel_approval(
                    RawOrigin::Signed(caller.clone()).into(), // current item owner
                    collection_id,                            // collection_id
                    item_id,                                  // item_id to be approved
                    Some(<T::Lookup as StaticLookup>::unlookup(operator)), // remove approval for this operator
                );

                log::trace!(target: "runtime",
                    "[UniquesExtension] cancel approval() result {:?}",
                    result
                );

                return match result {
                    Ok(_) => Ok(Converging(UniquesError::Success as u32)),
                    Err(e) => {
                        let mapped_error = UniquesError::try_from(e)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                };
            }
        }

        // Ok(Converging(UniquesError::Success as u32))
    }
}
