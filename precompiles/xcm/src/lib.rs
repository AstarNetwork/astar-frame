#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, Precompile};
use sp_core::{H160, H256, U256};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::latest::prelude::*;
use xcm_executor::traits::{Convert, InvertLocation};

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use precompile_utils::{
    revert, succeed, Address, Bytes, EvmDataWriter, EvmResult, FunctionModifier,
    PrecompileHandleExt, RuntimeHelper,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    AssetsWithdrawNative = "assets_withdraw(address[],uint256[],bytes32,bool,uint256,uint256)",
    AssetsWithdrawEvm = "assets_withdraw(address[],uint256[],address,bool,uint256,uint256)",
    RemoteTransact = "remote_transact(uint256,bool,address,uint256,bytes,uint64)",
    AssetsReserveTransferNative =
        "assets_reserve_transfer(address[],uint256[],bytes32,bool,uint256,uint256)",
    AssetsReserveTransferEvm =
        "assets_reserve_transfer(address[],uint256[],address,bool,uint256,uint256)",
}

/// A precompile that expose XCM related functions.
pub struct XcmPrecompile<T, C>(PhantomData<(T, C)>);

impl<R, C> Precompile for XcmPrecompile<R, C>
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::RuntimeCall:
        From<pallet_xcm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    C: Convert<MultiLocation, <R as pallet_assets::Config>::AssetId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XCM precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::NonPayable)?;

        // Dispatch the call
        match selector {
            Action::AssetsWithdrawNative => {
                Self::assets_withdraw(handle, BeneficiaryType::Account32)
            }
            Action::AssetsWithdrawEvm => Self::assets_withdraw(handle, BeneficiaryType::Account20),
            Action::RemoteTransact => Self::remote_transact(handle),
            Action::AssetsReserveTransferNative => {
                Self::assets_reserve_transfer(handle, BeneficiaryType::Account32)
            }
            Action::AssetsReserveTransferEvm => {
                Self::assets_reserve_transfer(handle, BeneficiaryType::Account20)
            }
        }
    }
}

/// The supported beneficiary account types
enum BeneficiaryType {
    /// 256 bit (32 byte) public key
    Account32,
    /// 160 bit (20 byte) address is expected
    Account20,
}

impl<R, C> XcmPrecompile<R, C>
where
    R: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
    <<R as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<R::AccountId>>,
    <R as frame_system::Config>::RuntimeCall:
        From<pallet_xcm::Call<R>> + Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    C: Convert<MultiLocation, <R as pallet_assets::Config>::AssetId>,
{
    fn assets_withdraw(
        handle: &mut impl PrecompileHandle,
        beneficiary_type: BeneficiaryType,
    ) -> EvmResult<PrecompileOutput> {
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
        if amounts_raw.iter().any(|x| *x > u128::MAX.into()) {
            return Err(revert("Asset amount is too big"));
        }
        let amounts: Vec<u128> = amounts_raw.iter().map(|x| x.low_u128()).collect();

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let beneficiary: MultiLocation = match beneficiary_type {
            BeneficiaryType::Account32 => {
                let recipient: [u8; 32] = input.read::<H256>()?.into();
                X1(Junction::AccountId32 {
                    network: Any,
                    id: recipient,
                })
            }
            BeneficiaryType::Account20 => {
                let recipient: H160 = input.read::<Address>()?.into();
                X1(Junction::AccountKey20 {
                    network: Any,
                    key: recipient.to_fixed_bytes(),
                })
            }
        }
        .into();

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

    fn remote_transact(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        // Raw call arguments
        let para_id: u32 = input.read::<U256>()?.low_u32();
        let is_relay = input.read::<bool>()?;

        let fee_asset_addr = input.read::<Address>()?;
        let fee_amount = input.read::<U256>()?;

        let remote_call: Vec<u8> = input.read::<Bytes>()?.into();
        let transact_weight = input.read::<u64>()?;

        log::trace!(target: "xcm-precompile:remote_transact", "Raw arguments: para_id: {}, is_relay: {}, fee_asset_addr: {:?}, \
         fee_amount: {:?}, remote_call: {:?}, transact_weight: {}",
        para_id, is_relay, fee_asset_addr, fee_amount, remote_call, transact_weight);

        // Process arguments
        let dest = if is_relay {
            MultiLocation::parent()
        } else {
            X1(Junction::Parachain(para_id)).into_exterior(1)
        };

        let fee_asset = {
            let fee_asset_id = R::address_to_asset_id(fee_asset_addr.into())
                .ok_or(revert("Failed to resolve fee asset id from address"))?;
            C::reverse_ref(fee_asset_id)
                .map_err(|_| revert("Failed to resolve fee asset multilocation from local id"))?
        };

        if fee_amount > u128::MAX.into() {
            return Err(revert("Fee amount is too big"));
        }
        let fee_amount = fee_amount.low_u128();

        let ancestry = R::LocationInverter::ancestry();
        let fee_multilocation = MultiAsset {
            id: Concrete(fee_asset),
            fun: Fungible(fee_amount),
        };
        let fee_multilocation = fee_multilocation
            .reanchored(&dest, &ancestry)
            .map_err(|_| revert("Failed to reanchor fee asset"))?;

        // Prepare XCM
        let xcm = Xcm(vec![
            WithdrawAsset(fee_multilocation.clone().into()),
            BuyExecution {
                fees: fee_multilocation.clone().into(),
                weight_limit: WeightLimit::Unlimited,
            },
            Transact {
                origin_type: OriginKind::SovereignAccount,
                require_weight_at_most: transact_weight,
                call: remote_call.into(),
            },
        ]);

        log::trace!(target: "xcm-precompile:remote_transact", "Processed arguments: dest: {:?}, fee asset: {:?}, XCM: {:?}", dest, fee_multilocation, xcm);

        // Build call with origin.
        let origin = Some(R::AddressMapping::into_account_id(handle.context().caller)).into();
        let call = pallet_xcm::Call::<R>::send {
            dest: Box::new(dest.into()),
            message: Box::new(xcm::VersionedXcm::V2(xcm)), // TODO: could this be problematic in case destination doesn't support v2?
        };

        // Dispatch a call.
        RuntimeHelper::<R>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn assets_reserve_transfer(
        handle: &mut impl PrecompileHandle,
        beneficiary_type: BeneficiaryType,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        // Read arguments and check it
        let assets: Vec<MultiLocation> = input
            .read::<Vec<Address>>()?
            .iter()
            .cloned()
            .filter_map(|address| {
                let address: H160 = address.into();

                // Special case where zero address maps to native token by convention.
                if address == H160::zero() {
                    Some(Here.into())
                } else {
                    R::address_to_asset_id(address).and_then(|x| C::reverse_ref(x).ok())
                }
            })
            .collect();
        let amounts_raw = input.read::<Vec<U256>>()?;
        if amounts_raw.iter().any(|x| *x > u128::MAX.into()) {
            return Err(revert("Asset amount is too big"));
        }
        let amounts: Vec<u128> = amounts_raw.iter().map(|x| x.low_u128()).collect();

        log::trace!(target: "xcm-precompile:assets_reserve_transfer", "Processed arguments: assets {:?}, amounts: {:?}", assets, amounts);

        // Check that assets list is valid:
        // * all assets resolved to multi-location
        // * all assets has corresponded amount
        if assets.len() != amounts.len() || assets.is_empty() {
            return Err(revert("Assets resolution failure."));
        }

        let beneficiary: MultiLocation = match beneficiary_type {
            BeneficiaryType::Account32 => {
                let recipient: [u8; 32] = input.read::<H256>()?.into();
                X1(Junction::AccountId32 {
                    network: Any,
                    id: recipient,
                })
            }
            BeneficiaryType::Account20 => {
                let recipient: H160 = input.read::<Address>()?.into();
                X1(Junction::AccountKey20 {
                    network: Any,
                    key: recipient.to_fixed_bytes(),
                })
            }
        }
        .into();

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

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(R::AddressMapping::into_account_id(handle.context().caller)).into();
        let call = pallet_xcm::Call::<R>::reserve_transfer_assets {
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
