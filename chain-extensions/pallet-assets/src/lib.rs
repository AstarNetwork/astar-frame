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
use assets_chain_extension_types::{AssetsError, Origin};
use frame_system::RawOrigin;
use pallet_assets::WeightInfo;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
};
use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;
use sp_std::marker::PhantomData;

enum AssetsFunc {
    Create,
}

impl TryFrom<u16> for AssetsFunc {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AssetsFunc::Create),
            _ => Err(DispatchError::Other(
                "PalletAssetsExtension: Unimplemented func_id",
            )),
        }
    }
}

/// Pallet Assets chain extension.
pub struct AssetsExtension<T>(PhantomData<T>);

impl<T> Default for AssetsExtension<T> {
    fn default() -> Self {
        AssetsExtension(PhantomData)
    }
}

impl<T> ChainExtension<T> for AssetsExtension<T>
where
    T: pallet_assets::Config + pallet_contracts::Config,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
    <T as SysConfig>::AccountId: From<[u8; 32]>,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        return match func_id {
            AssetsFunc::Create => {
                let (origin, id, admin, min_balance): (
                    Origin,
                    <T as pallet_assets::Config>::AssetId,
                    T::AccountId,
                    T::Balance
                ) = env.read_as()?;

                let base_weight = <T as pallet_assets::Config>::WeightInfo::create();
                env.charge_weight(base_weight)?;

                let runtime_origin = RawOrigin::Signed(match origin {
                    Origin::Caller => {
                        env.ext().caller().clone()
                    }
                    Origin::Address => env.ext().address().clone(),
                });

                let call_result = pallet_assets::Pallet::<T>::create(
                    runtime_origin.into(),
                    id,
                    admin.into(),
                    min_balance,
                );
                match call_result {
                    Err(e) => {
                        let mapped_error = AssetsError::try_from(e)?;
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(AssetsError::Success as u32)),
                }
            }
        };
    }
}