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

//! Pallet to handle XCM Callbacks.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//! - [`Event`]
//!
//! ## Overview
//!
//! The pallet provides functionality for xcm query management and handling callbacks:
//!
//! - Registering a new query
//! - Taking response of query (if available)
//! - Handling the pallet_xcm's OnResponse notify
//!
//! ### Terminology
//!
//! - **Callback:** When recieving the XCM response from pallet_xcm notify routing that
//!   response to desired destination (like wasm contract)
//! - **Pallet XCM Notify:** The pallet_xcm OnResponse handler can call (notify) custom
//!   disptach on recieving the XCM response given that query is registered before
//!   hand.
//! - **Manual Polling:** Instead of callback, the response is saved for the user to manually
//!   poll it.
//!
//! To use it in your runtime, you need to implement the pallet's [`Config`].
//!
//! ### Implementation
//!
//! The pallet provides implementations for the following traits.
//! - [`OnCallback`](pallet_xcm_transactor::OnCallback): Functions for dealing when a
//! callback is recieved.
//!
//! ### Goals
//! The callback system is designed to make following possible:
//!
//! - Registeration of new query which can either be manual polling or callbacks
//! - Allow query owners to take the response in case of manual polling
//! - Handle the incoming pallet_xcm's notify and route it with help of `CallbackHandler`
//!
//! ## Interface
//!
//! ### Permissioned Functions
//! - `on_callback_recieved`: Accepts the XCM Response and invoke the `CallbackHandler`, can only
//!   be called in a response to XCM.
//!
//! ### Public Functions
//! - `new_query`: Registers a new query and returns the query id
//! - `account_id`: Get the account id associated with this pallet that will be the origin
//!   of the callback
//! - `take_response`: Take the response if available

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, PalletId};
use frame_system::{pallet_prelude::*, Config as SysConfig};
pub use pallet::*;
use pallet_contracts::Pallet as PalletContracts;
use pallet_xcm::Pallet as PalletXcm;
use sp_core::H160;
use sp_runtime::traits::{AccountIdConversion, Zero};
use sp_std::prelude::*;
use xcm::prelude::*;
use xcm_executor::traits::WeightBounds;

pub mod chain_extension;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::Config as SysConfig;
    use pallet_xcm::ensure_response;
    pub use xcm_ce_primitives::{QueryConfig, QueryType};

    /// Response info
    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct ResponseInfo<AccountId> {
        pub query_id: QueryId,
        pub query_type: QueryType<AccountId>,
        pub response: Response,
    }

    /// Query infor
    #[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct QueryInfo<AccountId> {
        pub query_type: QueryType<AccountId>,
        pub querier: Junctions,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_xcm::Config + pallet_contracts::Config {
        /// The overarching event type.
        type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

        /// The overarching call type.
        type RuntimeCall: Parameter
            + From<Call<Self>>
            + IsType<<Self as pallet_xcm::Config>::RuntimeCall>;

        /// The overaching origin type
        type RuntimeOrigin: Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>
            + IsType<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Query Handler for creating quries and handling response
        type CallbackHandler: OnCallback<
            AccountId = Self::AccountId,
            BlockNumber = Self::BlockNumber,
        >;

        /// Required origin for registering new queries. If successful, it resolves to `MultiLocation`
        /// which exists as an interior location within this chain's XCM context.
        type RegisterQueryOrigin: EnsureOrigin<
            <Self as SysConfig>::RuntimeOrigin,
            Success = MultiLocation,
        >;

        /// Max weight for callback
        #[pallet::constant]
        type MaxCallbackWeight: Get<Weight>;
    }

    /// Mapping of ongoing queries and thier type
    #[pallet::storage]
    #[pallet::getter(fn callback_query)]
    pub(super) type CallbackQueries<T: Config> =
        StorageMap<_, Blake2_128Concat, QueryId, QueryInfo<T::AccountId>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// successfully handled callback
        CallbackSuccess(QueryType<T::AccountId>),
        CallbackFailed {
            query_type: QueryType<T::AccountId>,
            query_id: QueryId,
        },
        /// new query registered
        QueryPrepared {
            query_type: QueryType<T::AccountId>,
            query_id: QueryId,
        },
        /// query response taken
        ResponseTaken(QueryId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The version of the Versioned value used is not able to be interpreted.
        BadVersion,
        /// Origin not allow for registering queries
        InvalidOrigin,
        /// Query not found in storage
        UnexpectedQueryResponse,
        /// Does not support the given query type
        NotSupported,
        /// Querier mismatch
        InvalidQuerier,
        /// Failed to weigh XCM message
        CannotWeigh,
        /// Failed to validate xcm for sending
        SendValidateFailed,
        /// Callback out of gas
        /// TODO: use it
        OutOfGas,
        /// WASM Contract reverted
        WASMContractReverted,
        /// EVM Contract reverted
        EVMContractReverted,
        /// callback failed due to unkown reasons
        /// TODO: split this error into known errors
        CallbackFailed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Dispatch for recieving callback from pallet_xcm's notify
        /// and handle their routing
        /// TODO: Weights,
        ///       (max callback weight) + 1 DB read + 1 event + some extra (from benchmarking)
        #[pallet::call_index(0)]
        #[pallet::weight(T::MaxCallbackWeight::get())]
        pub fn on_callback_recieved(
            origin: OriginFor<T>,
            query_id: QueryId,
            response: Response,
        ) -> DispatchResult {
            // ensure the origin is a response
            let responder = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;
            // fetch the query
            let QueryInfo { query_type, .. } =
                CallbackQueries::<T>::get(query_id).ok_or(Error::<T>::UnexpectedQueryResponse)?;
            // handle the response routing
            // TODO: in case of error, maybe save the response for manual
            // polling as fallback. This will require taking into weight of storing
            // response in the weights of `prepare_new_query` dispatch
            if let Err(e) = T::CallbackHandler::on_callback(
                responder,
                ResponseInfo {
                    query_id,
                    query_type: query_type.clone(),
                    response,
                },
            ) {
                Self::deposit_event(Event::<T>::CallbackFailed {
                    query_type,
                    query_id,
                });
                return Err(e.into());
            }

            // remove query from storage
            CallbackQueries::<T>::remove(query_id);

            // deposit success event
            Self::deposit_event(Event::<T>::CallbackSuccess(query_type));
            Ok(())
        }
    }
}

/// Handle the incoming xcm notify callback from ResponseHandler (pallet_xcm)
pub trait OnCallback {
    /// error type, that can be converted to dispatch error
    type Error: Into<DispatchError>;
    /// account id type
    type AccountId;
    /// blocknumber type
    type BlockNumber;

    // TODO: Query type itself should be generic like
    //
    // type QueryType: Member + Parameter + MaybeSerializeDeserialize + MaxEncodedLen + Convert<Self, Weight>
    // type CallbackHandler: OnResponse<QueryType = T::QueryType>
    //
    // #[derive(RuntimeDebug, Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen)]
    // enum MyQueryType {}
    //
    // impl Convert<Self, Weight> for MyQueryType {}

    /// Check whether query type is supported or not
    fn can_handle(query_type: &QueryType<Self::AccountId>) -> bool;

    /// handle the xcm response
    fn on_callback(
        responder: impl Into<MultiLocation>,
        response_info: ResponseInfo<Self::AccountId>,
    ) -> Result<Weight, Self::Error>;
}

impl<T: Config> OnCallback for Pallet<T> {
    type AccountId = T::AccountId;
    type BlockNumber = T::BlockNumber;
    type Error = Error<T>;

    fn can_handle(query_type: &QueryType<Self::AccountId>) -> bool {
        match query_type {
            QueryType::NoCallback => true,
            QueryType::WASMContractCallback { .. } => true,
            // TODO: add support for evm contracts
            QueryType::EVMContractCallback { .. } => false,
        }
    }

    fn on_callback(
        responder: impl Into<MultiLocation>,
        response_info: ResponseInfo<Self::AccountId>,
    ) -> Result<Weight, Self::Error> {
        let ResponseInfo {
            query_id,
            query_type,
            response,
        } = response_info;

        match query_type {
            QueryType::NoCallback => {
                // TODO: Nothing to do, maybe error?
                Ok(Weight::zero())
            }
            QueryType::WASMContractCallback {
                contract_id,
                selector,
            } => Self::call_wasm_contract_method(
                contract_id,
                selector,
                query_id,
                responder.into(),
                response,
            ),
            QueryType::EVMContractCallback {
                contract_id,
                selector,
            } => Self::call_evm_contract_method(
                contract_id,
                selector,
                query_id,
                responder.into(),
                response,
            ),
        }
    }
}

/// Public methods
impl<T: Config> Pallet<T> {
    /// The account ID of the pallet.
    pub fn account_id() -> T::AccountId {
        const ID: PalletId = PalletId(*b"py/xcmnt");
        AccountIdConversion::<T::AccountId>::into_account_truncating(&ID)
    }

    /// Weigh the XCM to prepare for execution
    pub fn prepare_execute(
        origin: OriginFor<T>,
        xcm: Box<VersionedXcm<<T as SysConfig>::RuntimeCall>>,
    ) -> Result<(Xcm<<T as SysConfig>::RuntimeCall>, Weight), Error<T>> {
        T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_| Error::InvalidOrigin)?;

        let mut xcm = (*xcm).try_into().map_err(|_| Error::BadVersion)?;
        let weight = T::Weigher::weight(&mut xcm).map_err(|_| Error::CannotWeigh)?;
        Ok((xcm, weight))
    }

    pub fn execute(
        origin: OriginFor<T>,
        xcm: Box<VersionedXcm<<T as SysConfig>::RuntimeCall>>,
        weight_limit: Weight,
    ) -> Result<Outcome, Error<T>> {
        let origin_location =
            T::ExecuteXcmOrigin::ensure_origin(origin).map_err(|_| Error::InvalidOrigin)?;
        let xcm: Xcm<<T as SysConfig>::RuntimeCall> =
            (*xcm).try_into().map_err(|_| Error::BadVersion)?;

        // execute XCM
        // NOTE: not using pallet_xcm::execute here because it does not return XcmError
        //       which is needed to ensure xcm execution success
        let hash = xcm.using_encoded(sp_io::hashing::blake2_256);
        Ok(T::XcmExecutor::execute_xcm_in_credit(
            origin_location,
            xcm.clone(),
            hash,
            weight_limit,
            weight_limit,
        ))
    }

    pub fn validate_send(
        origin: OriginFor<T>,
        dest: Box<VersionedMultiLocation>,
        xcm: Box<VersionedXcm<()>>,
    ) -> Result<(Xcm<()>, MultiLocation, VersionedMultiAssets), Error<T>> {
        T::SendXcmOrigin::ensure_origin(origin).map_err(|_| Error::InvalidOrigin)?;
        let xcm: Xcm<()> = (*xcm).try_into().map_err(|_| Error::BadVersion)?;
        let dest: MultiLocation = (*dest).try_into().map_err(|_| Error::BadVersion)?;

        let (_, fees) = validate_send::<T::XcmRouter>(dest.clone(), xcm.clone())
            .map_err(|_| Error::SendValidateFailed)?;
        Ok((xcm, dest, VersionedMultiAssets::V3(fees)))
    }

    /// Take the response if available and querier matches
    pub fn take_response(
        origin: OriginFor<T>,
        query_id: QueryId,
    ) -> Result<Option<(Response, T::BlockNumber)>, Error<T>> {
        // ensure origin is allowed to make queries
        let origin_location: Junctions = T::RegisterQueryOrigin::ensure_origin(origin)
            .map_err(|_| Error::InvalidOrigin)?
            .try_into()
            .map_err(|_| Error::InvalidOrigin)?;

        let response = Self::do_take_response(origin_location, query_id)?;
        Self::deposit_event(Event::ResponseTaken(query_id));
        Ok(response)
    }

    /// Register new query originating from querier to dest
    pub fn new_query(
        origin: OriginFor<T>,
        config: QueryConfig<T::AccountId, T::BlockNumber>,
        dest: Box<VersionedMultiLocation>,
    ) -> Result<QueryId, Error<T>> {
        let origin_location =
            T::RegisterQueryOrigin::ensure_origin(origin).map_err(|_| Error::InvalidOrigin)?;
        let interior: Junctions = origin_location
            .try_into()
            .map_err(|_| Error::<T>::InvalidOrigin)?;
        let query_type = config.query_type.clone();
        let dest = MultiLocation::try_from(*dest).map_err(|()| Error::<T>::BadVersion)?;

        // register query
        let query_id = Self::do_new_query(config, interior, dest)?;
        Self::deposit_event(Event::<T>::QueryPrepared {
            query_type,
            query_id,
        });
        Ok(query_id)
    }
}

/// Internal methods
impl<T: Config> Pallet<T> {
    /// Register new query originating from querier to dest
    fn do_new_query(
        QueryConfig {
            query_type,
            timeout,
        }: QueryConfig<T::AccountId, T::BlockNumber>,
        querier: impl Into<Junctions>,
        dest: impl Into<MultiLocation>,
    ) -> Result<QueryId, Error<T>> {
        let querier = querier.into();

        // check if with callback handler
        if !(T::CallbackHandler::can_handle(&query_type)) {
            return Err(Error::NotSupported);
        }

        Ok(match query_type.clone() {
            QueryType::NoCallback => PalletXcm::<T>::new_query(dest, timeout, querier),
            QueryType::WASMContractCallback { .. } | QueryType::EVMContractCallback { .. } => {
                let call: <T as Config>::RuntimeCall = Call::on_callback_recieved {
                    query_id: 0,
                    response: Response::Null,
                }
                .into();
                let id = PalletXcm::<T>::new_notify_query(dest, call, timeout, querier.clone());
                CallbackQueries::<T>::insert(
                    id,
                    QueryInfo {
                        query_type,
                        querier,
                    },
                );
                id
            }
        })
    }

    fn do_take_response(
        querier: impl Into<Junctions>,
        query_id: QueryId,
    ) -> Result<Option<(Response, T::BlockNumber)>, Error<T>> {
        let query_info =
            CallbackQueries::<T>::get(query_id).ok_or(Error::<T>::UnexpectedQueryResponse)?;

        if querier.into() == query_info.querier {
            let response = pallet_xcm::Pallet::<T>::take_response(query_id);
            Self::deposit_event(Event::ResponseTaken(query_id));
            Ok(response)
        } else {
            Err(Error::<T>::InvalidQuerier)
        }
    }

    fn call_wasm_contract_method(
        contract_id: T::AccountId,
        selector: [u8; 4],
        query_id: QueryId,
        responder: MultiLocation,
        response: Response,
    ) -> Result<Weight, Error<T>> {
        // TODO: Use responder to derieve a origin account id
        let outcome = PalletContracts::<T>::bare_call(
            Self::account_id(),
            contract_id,
            Zero::zero(),
            T::MaxCallbackWeight::get(),
            None,
            [selector.to_vec(), (query_id, responder, response).encode()].concat(),
            // TODO: should not be true
            true,
            pallet_contracts::Determinism::Deterministic,
        );

        let retval = outcome.result.map_err(|_| Error::CallbackFailed)?;
        if retval.did_revert() {
            Err(Error::WASMContractReverted)
        } else {
            Ok(outcome.gas_consumed)
        }
    }

    fn call_evm_contract_method(
        _contract_id: H160,
        _selector: [u8; 4],
        _query_id: QueryId,
        _responder: MultiLocation,
        _response: Response,
    ) -> Result<Weight, Error<T>> {
        Ok(Weight::zero())
    }
}
