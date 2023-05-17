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

pub mod weights;
use core::marker::PhantomData;

use weights::CEWeightInfo;

use crate::{Config, Error as PalletError, Pallet, QueryConfig};
use frame_support::DefaultNoBound;
use frame_system::RawOrigin;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, Result as DispatchResult, RetVal, SysConfig,
};
use pallet_xcm::{Pallet as XcmPallet, WeightInfo as PalletXcmWeightInfo};
use parity_scale_codec::Encode;
use sp_std::prelude::*;
use xcm::prelude::*;
pub use xcm_ce_primitives::{
    Command::{self, *},
    PreparedExecution, ValidateSendInput, ValidatedSend,
    XcmCeError::{self, *},
    XCM_EXTENSION_ID,
};

type RuntimeCallOf<T> = <T as SysConfig>::RuntimeCall;

macro_rules! unwrap {
    ($val:expr) => {
        match $val {
            Ok(inner) => inner,
            Err(e) => {
                let err: XcmCeError = e.into();
                return Ok(RetVal::Converging(err.into()));
            }
        }
    };
    ($val:expr, $err:expr) => {
        match $val {
            Ok(inner) => inner,
            Err(_) => return Ok(RetVal::Converging($err.into())),
        }
    };
}

impl<T: Config> From<PalletError<T>> for XcmCeError {
    fn from(value: PalletError<T>) -> Self {
        match value {
            PalletError::BadVersion => BadVersion,
            PalletError::InvalidOrigin => InvalidOrigin,
            PalletError::NotSupported => NotSupported,
            PalletError::SendValidateFailed => SendValidateFailed,
            PalletError::CannotWeigh => CannotWeigh,
            PalletError::InvalidQuerier => InvalidQuerier,
            _ => RuntimeError,
        }
    }
}

#[derive(DefaultNoBound)]
pub struct XCMExtension<T: Config, W: CEWeightInfo> {
    prepared_execute: Option<PreparedExecution<RuntimeCallOf<T>>>,
    validated_send: Option<ValidatedSend>,
    _w: PhantomData<W>,
}

impl<T: Config, W: CEWeightInfo> ChainExtension<T> for XCMExtension<T, W>
where
    <T as SysConfig>::AccountId: AsRef<[u8; 32]>,
{
    fn call<E>(&mut self, env: Environment<E, InitState>) -> DispatchResult<RetVal>
    where
        E: Ext<T = T>,
    {
        match unwrap!(env.func_id().try_into(), InvalidCommand) {
            PrepareExecute => self.prepare_execute(env),
            Execute => self.execute(env),
            ValidateSend => self.validate_send(env),
            Send => self.send(env),
            NewQuery => self.new_query(env),
            TakeResponse => self.take_response(env),
            PalletAccountId => self.pallet_account_id(env),
        }
    }
}

impl<T: Config, W: CEWeightInfo> XCMExtension<T, W> {
    /// Returns the weight for given XCM and saves it (in CE, per-call scratch buffer) for
    /// execution
    fn prepare_execute<E: Ext<T = T>>(
        &mut self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        // input parsing
        let len = env.in_len();
        let input: VersionedXcm<RuntimeCallOf<T>> = env.read_as_unbounded(len)?;

        // charge weight
        env.charge_weight(W::prepare_execute(len))?;

        let origin = RawOrigin::Signed(env.ext().address().clone());
        let (xcm, weight) = unwrap!(Pallet::<T>::prepare_execute(origin.into(), Box::new(input)));

        // save the prepared xcm
        self.prepared_execute = Some(PreparedExecution { xcm, weight });
        // write the output to buffer
        weight.using_encoded(|w| env.write(w, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    /// Execute the XCM that was prepared earlier
    fn execute<E: Ext<T = T>>(
        &mut self,
        mut env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let PreparedExecution { xcm, weight } = unwrap!(
            self.prepared_execute.as_ref().take().ok_or(()),
            PreparationMissing
        );
        // charge weight
        let charged = env.charge_weight(W::execute().saturating_add(*weight))?;
        // TODO: find better way to get origin
        //       https://github.com/paritytech/substrate/pull/13708
        let origin = RawOrigin::Signed(env.ext().address().clone());
        let outcome = unwrap!(
            Pallet::<T>::execute(
                origin.into(),
                Box::new(VersionedXcm::V3(xcm.clone())),
                *weight,
            ),
            // TODO: mapp pallet error 1-1 with CE errors
            InvalidOrigin
        );
        // adjust with actual weights used
        env.adjust_weight(charged, outcome.weight_used().saturating_add(W::execute()));
        // revert for anything but a complete execution
        match outcome {
            Outcome::Complete(_) => Ok(RetVal::Converging(Success.into())),
            _ => Ok(RetVal::Converging(ExecutionFailed.into())),
        }
    }

    /// Returns the fee required to send XCM and saves
    /// it for sending
    fn validate_send<E: Ext<T = T>>(
        &mut self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        let len = env.in_len();
        let ValidateSendInput { dest, xcm } = env.read_as_unbounded(len)?;
        // charge weight
        env.charge_weight(W::validate_send(len))?;

        let origin = RawOrigin::Signed(env.ext().address().clone());
        // validate and get fees required to send
        let (xcm, dest, fees) = unwrap!(Pallet::<T>::validate_send(
            origin.into(),
            Box::new(dest.clone()),
            Box::new(xcm.clone())
        ));
        // save the validated input
        self.validated_send = Some(ValidatedSend { dest, xcm });
        // write the fees to output
        VersionedMultiAssets::from(fees).using_encoded(|a| env.write(a, true, None))?;
        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    /// Send the validated XCM
    fn send<E: Ext<T = T>>(
        &mut self,
        mut env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let input = unwrap!(
            self.validated_send.as_ref().take().ok_or(()),
            PreparationMissing
        );
        // charge weight
        let base_weight = <T as pallet_xcm::Config>::WeightInfo::send();
        env.charge_weight(base_weight.saturating_add(W::send()))?;

        // TODO: find better way to get origin
        //       https://github.com/paritytech/substrate/pull/13708
        let origin = RawOrigin::Signed(env.ext().address().clone());
        // send the xcm
        unwrap!(
            XcmPallet::<T>::send(
                origin.into(),
                Box::new(input.dest.into()),
                Box::new(xcm::VersionedXcm::V3(input.xcm.clone())),
            ),
            SendFailed
        );

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    /// Register the new query
    fn new_query<E: Ext<T = T>>(&self, env: Environment<E, InitState>) -> DispatchResult<RetVal>
    where
        <T as SysConfig>::AccountId: AsRef<[u8; 32]>,
    {
        let mut env = env.buf_in_buf_out();
        let len = env.in_len();
        let (query_config, dest): (
            QueryConfig<T::AccountId, T::BlockNumber>,
            VersionedMultiLocation,
        ) = env.read_as_unbounded(len)?;
        // charge weight
        // NOTE: we only charge the weight associated with query registration and processing of
        //       calllback only. This does not include the CALLBACK weights
        env.charge_weight(W::new_query())?;

        let origin = RawOrigin::Signed(env.ext().address().clone());
        // register the query
        let query_id: u64 = Pallet::<T>::new_query(origin.into(), query_config, Box::new(dest))?;
        // write the query_id to buffer
        query_id.using_encoded(|q| env.write(q, true, None))?;
        Ok(RetVal::Converging(Success.into()))
    }

    /// Take the response for query if available
    /// TODO: figure out weights
    fn take_response<E: Ext<T = T>>(
        &self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        let query_id: u64 = env.read_as()?;
        // charge weight
        env.charge_weight(W::take_response())?;
        // TODO: find better way to get origin
        //       https://github.com/paritytech/substrate/pull/13708
        let origin = RawOrigin::Signed(env.ext().address().clone());
        let response = unwrap!(
            unwrap!(Pallet::<T>::take_response(origin.into(), query_id))
                .map(|r| r.0)
                .ok_or(()),
            NoResponse
        );

        VersionedResponse::from(response).using_encoded(|r| env.write(r, true, None))?;
        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    /// Get the pallet account id which will call the contract callback
    /// TODO: figure out weights
    fn pallet_account_id<E: Ext<T = T>>(
        &self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        // charge weight
        env.charge_weight(W::account_id())?;

        Pallet::<T>::account_id().using_encoded(|r| env.write(r, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }
}
