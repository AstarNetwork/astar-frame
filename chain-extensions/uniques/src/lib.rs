#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;

use chain_extension_trait::ChainExtensionExec;

use frame_support::log;
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
                    "[UniquesExtension] create() call admin {:?}, weight {:?}",
                    contract, weight_to_charge
                );
                let result = pallet_uniques::Pallet::<T>::create(
                    RawOrigin::Signed(contract.clone()).into(),
                    <T::Lookup as StaticLookup>::unlookup(caller), // admin
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
        }

        // Ok(Converging(UniquesError::Success as u32))
    }
}
