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

//! WASM (substrate contracts) support for XVM pallet.

use crate::*;
use codec::HasCompact;
use frame_support::traits::Currency;
use scale_info::TypeInfo;
use sp_runtime::traits::Get;
use sp_std::fmt::Debug;

pub struct WASM<I, T>(sp_std::marker::PhantomData<(I, T)>);

type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

impl<I, T> SyncVM<T::AccountId> for WASM<I, T>
where
    I: Get<VmId>,
    T: pallet_contracts::Config + frame_system::Config,
    <BalanceOf<T> as HasCompact>::Type: Clone + Eq + PartialEq + Debug + TypeInfo + Encode,
{
    fn id() -> VmId {
        I::get()
    }

    fn xvm_call(context: XvmContext, from: T::AccountId, to: Vec<u8>, input: Vec<u8>) -> XvmResult {
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "Start WASM XVM: {:?}, {:?}, {:?}",
            from, to, input,
        );
        let gas_limit = context.max_weight;
        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM xvm call gas (weight) limit: {:?}", gas_limit);
        let dest = Decode::decode(&mut to.as_ref()).map_err(|_| XvmCallError {
            error: XvmError::EncodingFailure,
            consumed_weight: PLACEHOLDER_WEIGHT,
        })?;
        let res = pallet_contracts::Pallet::<T>::call(
            frame_support::dispatch::RawOrigin::Signed(from).into(),
            dest,
            Default::default(),
            gas_limit.into(),
            None,
            input,
        )
        .map_err(|e| {
            let consumed_weight = if let Some(weight) = e.post_info.actual_weight {
                weight.ref_time()
            } else {
                gas_limit.ref_time()
            };
            XvmCallError {
                error: XvmError::ExecutionError(Into::<&str>::into(e.error).into()),
                consumed_weight,
            }
        })?;

        log::trace!(
            target: "xvm::WASM::xvm_call",
            "WASM XVM call result: {:?}", res
        );

        let consumed_weight = if let Some(weight) = res.actual_weight {
            weight.ref_time()
        } else {
            gas_limit.ref_time()
        };
        Ok(XvmCallOk {
            output: Default::default(), // TODO: Fill in with output from the call
            consumed_weight,
        })
    }
}
