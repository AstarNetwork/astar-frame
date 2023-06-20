// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{PrecompileHandle, PrecompileOutput};
use frame_support::{
    dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
    pallet_prelude::Weight,
    traits::{ConstU32, Get},
};
pub const XCM_SIZE_LIMIT: u32 = 2u32.pow(16);
type GetXcmSizeLimit = ConstU32<XCM_SIZE_LIMIT>;

use pallet_evm::{AddressMapping, Precompile};
use parity_scale_codec::DecodeLimit;
use sp_core::{H160, U256};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use xcm::latest::prelude::*;
use xcm_executor::traits::Convert;

use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use precompile_utils::{
    bytes::BoundedBytes, revert, succeed, Address, Bytes, EvmDataWriter, EvmResult,
    FunctionModifier, PrecompileHandleExt, RuntimeHelper,
};
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    AssetsWithdraw = "assets_withdraw(address[],uint256[],(uint8,bytes[]),(uint8,bytes[]),uint256)",
    RemoteTransact = "remote_transact((uint8,bytes[]),address,uint256,bytes,uint64)",
    AssetsReserveTransfer =
        "assets_reserve_transfer(address[],uint256[],(uint8,bytes[]),(uint8,bytes[]),uint256)",
    SendXCM = "send_xcm((uint8,bytes[]),bytes)",
}

/// Dummy H160 address representing native currency (e.g. ASTR or SDN)
const NATIVE_ADDRESS: H160 = H160::zero();

/// A precompile that expose XCM related functions.
pub struct XcmPrecompile<T, C>(PhantomData<(T, C)>);

impl<Runtime, C> Precompile for XcmPrecompile<Runtime, C>
where
    Runtime: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<Runtime as pallet_assets::Config>::AssetId>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::AccountId: Into<[u8; 32]>,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    C: Convert<MultiLocation, <Runtime as pallet_assets::Config>::AssetId>,
{
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "xcm-precompile", "In XCM precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::NonPayable)?;

        // Dispatch the call
        match selector {
            Action::AssetsWithdraw => Self::assets_withdraw(handle),
            Action::RemoteTransact => Self::remote_transact(handle),
            Action::AssetsReserveTransfer => Self::assets_reserve_transfer(handle),
            Action::SendXCM => Self::send_xcm(handle),
        }
    }
}

impl<Runtime, C> XcmPrecompile<Runtime, C>
where
    Runtime: pallet_evm::Config
        + pallet_xcm::Config
        + pallet_assets::Config
        + AddressToAssetId<<Runtime as pallet_assets::Config>::AssetId>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
        From<Option<Runtime::AccountId>>,
    <Runtime as frame_system::Config>::AccountId: Into<[u8; 32]>,
    <Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>
        + Dispatchable<PostInfo = PostDispatchInfo>
        + GetDispatchInfo,
    C: Convert<MultiLocation, <Runtime as pallet_assets::Config>::AssetId>,
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
                Runtime::address_to_asset_id(address.into()).and_then(|x| C::reverse_ref(x).ok())
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

        let beneficiary: MultiLocation = input.read::<MultiLocation>()?;
        let dest: MultiLocation = input.read::<MultiLocation>()?;

        let fee_asset_item: u32 = input.read::<U256>()?.low_u32();

        log::trace!(target: "xcm-precompile::asset_withdraw", "Raw arguments: assets: {:?}, asset_amount: {:?} \
         beneficiart: {:?}, destination: {:?}, fee_index: {}",
        assets, amounts_raw, beneficiary, dest, fee_asset_item);

        if fee_asset_item as usize > assets.len() {
            return Err(revert("Bad fee index."));
        }

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::reserve_withdraw_assets {
            dest: Box::new(dest.into()),
            beneficiary: Box::new(beneficiary.into()),
            assets: Box::new(assets.into()),
            fee_asset_item,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn remote_transact(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(6)?;

        let dest: MultiLocation = input.read::<MultiLocation>()?;
        let fee_asset_addr = input.read::<Address>()?;
        let fee_amount = input.read::<U256>()?;

        let remote_call: Vec<u8> = input.read::<Bytes>()?.into();
        let transact_weight = input.read::<u64>()?;
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

        log::trace!(target: "xcm-precompile::remote_transact", "Raw arguments: dest: {:?}, fee_asset_addr: {:?} \
         fee_amount: {:?}, remote_call: {:?}, transact_weight: {}",
        dest, fee_asset_addr, fee_amount, remote_call, transact_weight);

        let fee_asset = {
            let address: H160 = fee_asset_addr.into();

            // Special case where zero address maps to native token by convention.
            if address == NATIVE_ADDRESS {
                Here.into()
            } else {
                let fee_asset_id = Runtime::address_to_asset_id(address)
                    .ok_or(revert("Failed to resolve fee asset id from address"))?;
                C::reverse_ref(fee_asset_id).map_err(|_| {
                    revert("Failed to resolve fee asset multilocation from local id")
                })?
            }
        };

        if fee_amount > u128::MAX.into() {
            return Err(revert("Fee amount is too big"));
        }
        let fee_amount = fee_amount.low_u128();

        let context = Runtime::UniversalLocation::get();
        let fee_multilocation = MultiAsset {
            id: Concrete(fee_asset),
            fun: Fungible(fee_amount),
        };
        let fee_multilocation = fee_multilocation
            .reanchored(&dest, context)
            .map_err(|_| revert("Failed to reanchor fee asset"))?;

        // Prepare XCM
        let xcm = Xcm(vec![
            WithdrawAsset(fee_multilocation.clone().into()),
            BuyExecution {
                fees: fee_multilocation.clone().into(),
                weight_limit: WeightLimit::Unlimited,
            },
            SetAppendix(Xcm(vec![DepositAsset {
                assets: All.into(),
                beneficiary: MultiLocation {
                    parents: 1,
                    interior: X2(
                        // last() returns the last Juction Enum in Junctions
                        // for Univeral Location it is the Parachain() variant
                        *context.last().unwrap(),
                        AccountId32 {
                            network: None,
                            id: origin.into(),
                        },
                    ),
                },
            }])),
            Transact {
                origin_kind: OriginKind::SovereignAccount,
                require_weight_at_most: Weight::from_ref_time(transact_weight),
                call: remote_call.into(),
            },
        ]);

        log::trace!(target: "xcm-precompile:remote_transact", "Processed arguments: dest: {:?}, fee asset: {:?}, XCM: {:?}", dest, fee_multilocation, xcm);

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::send {
            dest: Box::new(dest.into()),
            message: Box::new(xcm::VersionedXcm::V3(xcm)),
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn assets_reserve_transfer(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
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
                if address == NATIVE_ADDRESS {
                    Some(Here.into())
                } else {
                    Runtime::address_to_asset_id(address).and_then(|x| C::reverse_ref(x).ok())
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

        let beneficiary: MultiLocation = input.read::<MultiLocation>()?;
        let dest: MultiLocation = input.read::<MultiLocation>()?;

        let fee_asset_item: u32 = input.read::<U256>()?.low_u32();

        if fee_asset_item as usize > assets.len() {
            return Err(revert("Bad fee index."));
        }

        let assets: MultiAssets = assets
            .iter()
            .cloned()
            .zip(amounts.iter().cloned())
            .map(Into::into)
            .collect::<Vec<MultiAsset>>()
            .into();

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::reserve_transfer_assets {
            dest: Box::new(dest.into()),
            beneficiary: Box::new(beneficiary.into()),
            assets: Box::new(assets.into()),
            fee_asset_item,
        };

        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn send_xcm(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(3)?;

        // Raw call arguments
        let dest: MultiLocation = input.read::<MultiLocation>()?;
        let xcm_call: Vec<u8> = input.read::<BoundedBytes<GetXcmSizeLimit>>()?.into();

        log::trace!(target:"xcm-precompile::send_xcm", "Raw arguments: dest: {:?}, xcm_call: {:?}", dest, xcm_call);

        let xcm = xcm::VersionedXcm::<()>::decode_all_with_depth_limit(
            xcm::MAX_XCM_DECODE_DEPTH,
            &mut xcm_call.as_slice(),
        )
        .map_err(|_| revert("Failed to decode xcm instructions"))?;

        // Build call with origin.
        let origin = Some(Runtime::AddressMapping::into_account_id(
            handle.context().caller,
        ))
        .into();
        let call = pallet_xcm::Call::<Runtime>::send {
            dest: Box::new(dest.into()),
            message: Box::new(xcm),
        };
        log::trace!(target: "xcm-send_xcm", "Processed arguments:  XCM call: {:?}", call);
        // Dispatch a call.
        RuntimeHelper::<Runtime>::try_dispatch(handle, origin, call)?;

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }
}
