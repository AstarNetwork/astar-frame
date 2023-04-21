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

#[cfg(test)]
mod mock;
#[cfg(test)]
pub mod tests;
pub mod weights;

use frame_support::dispatch::Weight;
use frame_support::traits::{schedule, Currency};
use frame_system::RawOrigin;
use pallet_contracts::{
    chain_extension::{ChainExtension, Environment, Ext, InitState, RetVal, SysConfig},
    Call as PalletContractCall,
};
use pallet_scheduler::WeightInfo;
use parity_scale_codec::{Encode, HasCompact};
use scale_info::TypeInfo;
use scheduler_chain_extension_types::*;
use sp_core::Get;
use sp_runtime::traits::StaticLookup;
use sp_runtime::DispatchError;
use sp_std::boxed::Box;
use sp_std::fmt::Debug;
use sp_std::marker::PhantomData;

enum SchedulerFunc {
    Schedule,
    Cancel,
}

impl TryFrom<u16> for SchedulerFunc {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SchedulerFunc::Schedule),
            2 => Ok(SchedulerFunc::Cancel),
            _ => Err(DispatchError::Other(
                "PalletSchedulerExtension: Unimplemented func_id",
            )),
        }
    }
}

type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

/// Pallet Scheduler chain extension.
pub struct SchedulerExtension<T, W>(PhantomData<(T, W)>);

impl<T, W> Default for SchedulerExtension<T, W> {
    fn default() -> Self {
        SchedulerExtension(PhantomData)
    }
}

impl<T, W> ChainExtension<T> for SchedulerExtension<T, W>
where
    T: pallet_scheduler::Config + pallet_contracts::Config,
    <<T as SysConfig>::Lookup as StaticLookup>::Source: From<<T as SysConfig>::AccountId>,
    <T as SysConfig>::AccountId: From<[u8; 32]>,
    <<<T as pallet_contracts::Config>::Currency as Currency<<T as SysConfig>::AccountId>>::Balance as HasCompact>::Type: Clone + Encode + TypeInfo + Debug + Eq,
    <T as pallet_contracts::Config>::RuntimeCall: From<pallet_contracts::Call<T>> + Encode,
    <T as pallet_scheduler::Config>::RuntimeCall: From<pallet_contracts::Call<T>>,
    W: weights::WeightInfo,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            SchedulerFunc::Schedule => {
                let (origin, when, maybe_periodic, priority, call_input): (
                    Origin,
                    T::BlockNumber,
                    Option<schedule::Period<T::BlockNumber>>,
                    schedule::Priority,
                    ContractCallInput<T::AccountId, BalanceOf<T>>,
                ) = env.read_as_unbounded(env.in_len())?;
                
                let read_weigth = <W as weights::WeightInfo>::read_as_unbounded(env.in_len());
                env.charge_weight(read_weigth)?;
                
                let base_weight = <T as pallet_scheduler::Config>::WeightInfo::schedule(
                    T::MaxScheduledPerBlock::get(),
                );
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call: <T as pallet_scheduler::Config>::RuntimeCall =
                    PalletContractCall::<T>::call {
                    dest: call_input.dest.into(),
                        value: call_input.value,
                        gas_limit: Weight::from_parts(call_input.gas_limit.0, call_input.gas_limit.1),
                        data: call_input.data.into(),
                        storage_deposit_limit: None,
                    }.into();

                let call_result = pallet_scheduler::Pallet::<T>::schedule(
                    raw_origin.into(),
                    when,
                    maybe_periodic,
                    priority,
                    Box::new(call),
                );
                return match call_result {
                    Err(e) => {
                        sp_std::if_std! {println!("Schedule:{:?}", e)};
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            },
            SchedulerFunc::Cancel => {
                let (origin, when, index): (
                    Origin,
                    T::BlockNumber,
                    u32,
                ) = env.read_as()?;

                let base_weight = <T as pallet_scheduler::Config>::WeightInfo::cancel(
                    T::MaxScheduledPerBlock::get(),
                );
                env.charge_weight(base_weight)?;

                let raw_origin = select_origin!(&origin, env.ext().address().clone());

                let call_result = pallet_scheduler::Pallet::<T>::cancel(
                    raw_origin.into(),
                    when,
                    index,
                );
                return match call_result {
                    Err(e) => {
                        sp_std::if_std! {println!("Cancel:{:?}", e)};
                        let mapped_error = Outcome::from(e);
                        Ok(RetVal::Converging(mapped_error as u32))
                    }
                    Ok(_) => Ok(RetVal::Converging(Outcome::Success as u32)),
                };
            }
        }
    }
}
