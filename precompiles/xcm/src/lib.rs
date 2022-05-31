#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use fp_evm::{Context, ExitSucceed, PrecompileOutput};
use pallet_evm::{Precompile, AddressMapping};
use sp_core::{H256, U256};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::latest::prelude::*;
use xcm_executor::traits::Convert;

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use precompile_utils::{
    Address, EvmDataReader, EvmDataWriter, EvmResult, FunctionModifier, Gasometer, RuntimeHelper,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    AssetsWithdraw = "assets_withdraw(address[],uint256[],bytes32,bool,uint256,uint256)",
}

/// A precompile that expose XCM related functions.
pub struct XCMPrecompile<T, C>(PhantomData<(T, C)>);

impl<R, C> Precompile for XCMPrecompile<R, C> 
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    <R as frame_system::Config>::Call: From<pallet_xcm::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    C: Convert<MultiLocation, <R as pallet_assets::Config>::AssetId>,
{
    fn execute(
        input: &[u8], //Reminder this is big-endian
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XCM precompile");

        let mut gasometer = Gasometer::new(target_gas);
        let gasometer = &mut gasometer;

        let (mut input, selector) = EvmDataReader::new_with_selector(gasometer, input)?;
        let input = &mut input;

        gasometer.check_function_modifier(context, is_static, FunctionModifier::NonPayable)?;

        match selector {
            // Dispatchables
            Action::AssetsWithdraw => Self::assets_withdraw(input, gasometer, context),
        }
    }
}

impl<R, C> XCMPrecompile<R, C> 
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    <R as frame_system::Config>::Call: From<pallet_xcm::Call<R>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    C: Convert<MultiLocation, <R as pallet_assets::Config>::AssetId>,
{
    fn assets_withdraw(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        context: &Context,
    ) -> EvmResult<PrecompileOutput> {
        // Bound check
        input.expect_arguments(gasometer, 6)?;

        // Read arguments and check it
        let assets: Vec<MultiLocation> = input.read::<Vec<Address>>(gasometer)?
            .iter()
            .cloned()
            .filter_map(|address| 
                R::address_to_asset_id(address.into())
                    .map(|x| C::reverse_ref(x).ok())
                    .flatten()
            )
            .collect();
        let amounts: Vec<u128> = input.read::<Vec<U256>>(gasometer)?
            .iter()
            .map(|x| x.low_u128())
            .collect();

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() {
            return Err(precompile_utils::error("bad assets list") )
        }

        let recipient: [u8; 32] = input.read::<H256>(gasometer)?.into();
        let is_relay = input.read::<bool>(gasometer)?;
        let parachain_id: u32 = input.read::<U256>(gasometer)?.low_u32();
        let fee_asset_item: u32 = input.read::<U256>(gasometer)?.low_u32();

        // Prepare pallet-xcm call arguments
        let dest = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(parachain_id)).into()
        };

        let beneficiary: MultiLocation = X1(Junction::AccountId32 {
            network: Any,
            id: recipient,
        }).into();

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(R::AddressMapping::into_account_id(context.caller)).into();
        let call = pallet_xcm::Call::<R>::reserve_withdraw_assets {
            dest: Box::new(dest.into()),
            beneficiary: Box::new(beneficiary.into()),
            assets: Box::new(assets.into()),
            fee_asset_item,
        }.into();

        // Dispatch a call.
        RuntimeHelper::<R>::try_dispatch(origin, call, gasometer)?;

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(true).build(),
            logs: Default::default(),
        })
    }
}
