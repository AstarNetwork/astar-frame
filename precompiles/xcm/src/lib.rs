#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, Precompile};
use sp_core::{H256, U256};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::latest::prelude::*;
use xcm_executor::traits::Convert;

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use precompile_utils::{
    revert, succeed, Address, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt,
    RuntimeHelper,
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
pub struct XcmPrecompile<T, C>(PhantomData<(T, C)>);

impl<R, C> Precompile for XcmPrecompile<R, C>
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    <R as frame_system::Config>::Call:
        From<pallet_xcm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    C: Convert<MultiLocation, <R as pallet_assets::Config>::AssetId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XCM precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::NonPayable)?;

        match selector {
            // Dispatchables
            Action::AssetsWithdraw => Self::assets_withdraw(handle),
        }
    }
}

impl<R, C> XcmPrecompile<R, C>
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    <<R as frame_system::Config>::Call as Dispatchable>::Origin: From<Option<R::AccountId>>,
    <R as frame_system::Config>::Call:
        From<pallet_xcm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    C: Convert<MultiLocation, <R as pallet_assets::Config>::AssetId>,
{
    fn assets_withdraw(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        // Read arguments and check it
        let assets: Vec<MultiLocation> = input
            .read::<Vec<Address>>()?
            .iter()
            .cloned()
            .filter_map(|address| {
                R::address_to_asset_id(address.into()).and_then(|x| C::reverse_ref(x).ok())
            })
            .collect();
        let amounts_raw = input.read::<Vec<U256>>()?;
        if amounts_raw.iter().any(|x| *x > u128::max_value().into()) {
            return Err(revert("Asset amount is too big"));
        }
        let amounts: Vec<u128> = amounts_raw.iter().map(|x| x.low_u128()).collect();

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let recipient: [u8; 32] = input.read::<H256>()?.into();
        let is_relay = input.read::<bool>()?;
        let parachain_id: u32 = input.read::<U256>()?.low_u32();
        let fee_asset_item: u32 = input.read::<U256>()?.low_u32();

        if fee_asset_item as usize > assets.len() {
            return Err(revert("Bad fee index."));
        }

        // Prepare pallet-xcm call arguments
        let dest = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(parachain_id)).into_exterior(1)
        };

        let beneficiary: MultiLocation = X1(Junction::AccountId32 {
            network: Any,
            id: recipient,
        })
        .into();

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(R::AddressMapping::into_account_id(handle.context().caller)).into();
        let call = pallet_xcm::Call::<R>::reserve_withdraw_assets {
            dest: Box::new(dest.into()),
            beneficiary: Box::new(beneficiary.into()),
            assets: Box::new(assets.into()),
            fee_asset_item,
        };

        // Dispatch a call.
        RuntimeHelper::<R>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }
}
