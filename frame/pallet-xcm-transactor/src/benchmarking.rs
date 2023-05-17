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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::traits::Contains;
use sp_std::prelude::*;
use xcm::latest::prelude::*;

fn mock_xcm_response<T: Config>(
    response_info: QueryResponseInfo,
    querier: impl Into<MultiLocation>,
    response: Response,
    weight_limit: Weight,
) {
    let QueryResponseInfo {
        destination,
        query_id,
        max_weight,
    } = response_info;
    let querier = Some(querier.into());
    let response_xcm = Xcm(vec![QueryResponse {
        querier,
        query_id,
        max_weight,
        response,
    }]);
    let hash = response_xcm.using_encoded(sp_io::hashing::blake2_256);
    T::XcmExecutor::execute_xcm_in_credit(
        destination,
        response_xcm,
        hash,
        weight_limit,
        weight_limit,
    );
}

/// Assert that the last event equals the provided one.
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
    frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks(where <T as SysConfig>::AccountId: From<[u8; 32]>)]
mod benchmarks_module {

    use sp_runtime::traits::Bounded;

    use super::*;

    #[benchmark]
    fn account_id() {
        #[block]
        {
            let _ = Pallet::<T>::account_id();
        }
    }

    #[benchmark]
    fn prepare_execute() -> Result<(), BenchmarkError> {
        let execute_origin =
            T::ExecuteXcmOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
        let msg = Xcm(vec![ClearOrigin]);
        let versioned_msg = VersionedXcm::from(msg);

        #[block]
        {
            let _ = Pallet::<T>::prepare_execute(execute_origin, Box::new(versioned_msg)).unwrap();
        }

        Ok(())
    }

    #[benchmark]
    fn execute() -> Result<(), BenchmarkError> {
        let execute_origin =
            T::ExecuteXcmOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
        let origin_location = T::ExecuteXcmOrigin::try_origin(execute_origin.clone())
            .map_err(|_| BenchmarkError::Override(BenchmarkResult::from_weight(Weight::MAX)))?;
        let msg = Xcm(vec![ClearOrigin]);
        if !T::XcmExecuteFilter::contains(&(origin_location, msg.clone())) {
            return Err(BenchmarkError::Override(BenchmarkResult::from_weight(
                Weight::MAX,
            )));
        }
        let versioned_msg = VersionedXcm::from(msg);

        #[block]
        {
            let _ = Pallet::<T>::execute(execute_origin, Box::new(versioned_msg), Weight::zero())
                .unwrap();
        }

        Ok(())
    }

    #[benchmark]
    fn validate_send() -> Result<(), BenchmarkError> {
        let send_origin =
            T::SendXcmOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
        if T::SendXcmOrigin::try_origin(send_origin.clone()).is_err() {
            return Err(BenchmarkError::Override(BenchmarkResult::from_weight(
                Weight::MAX,
            )));
        }
        let msg = Xcm(vec![ClearOrigin]);
        let versioned_dest: VersionedMultiLocation = T::ReachableDest::get()
            .ok_or(BenchmarkError::Override(BenchmarkResult::from_weight(
                Weight::MAX,
            )))?
            .into();
        let versioned_msg = VersionedXcm::from(msg);

        #[block]
        {
            let _ = Pallet::<T>::validate_send(
                send_origin,
                Box::new(versioned_dest),
                Box::new(versioned_msg),
            )
            // not a good idea to unwrap here but it's the only way to
            // check if it worked
            .unwrap();
        }

        Ok(())
    }

    #[benchmark]
    fn take_response() -> Result<(), BenchmarkError> {
        let query_origin = T::RegisterQueryOrigin::try_successful_origin()
            .map_err(|_| BenchmarkError::Weightless)?;
        if T::RegisterQueryOrigin::try_origin(query_origin.clone()).is_err() {
            return Err(BenchmarkError::Override(BenchmarkResult::from_weight(
                Weight::MAX,
            )));
        }

        let responder = (Parent, Parachain(1000));
        let weight_limit = Weight::from_parts(100_000_000_000, 1024 * 1024);
        //register query
        let query_id = Pallet::<T>::new_query(
            query_origin.clone(),
            QueryConfig {
                query_type: QueryType::NoCallback,
                timeout: Bounded::max_value(),
            },
            Box::new(responder.into()),
        )
        .map_err(|_| BenchmarkError::Stop("Failed to register new query"))?;
        // mock response
        mock_xcm_response::<T>(
            QueryResponseInfo {
                destination: responder.into(),
                query_id,
                max_weight: Weight::zero(),
            },
            Here,
            Response::Null,
            weight_limit,
        );

        #[block]
        {
            let _ = Pallet::<T>::take_response(query_origin.clone(), query_id);
        }

        // make sure response is taken
        assert_eq!(pallet_xcm::Pallet::<T>::query(query_id), None);
        assert_last_event::<T>(Event::<T>::ResponseTaken(query_id).into());
        Ok(())
    }

    #[benchmark]
    fn new_query() -> Result<(), BenchmarkError> {
        let query_origin = T::RegisterQueryOrigin::try_successful_origin()
            .map_err(|_| BenchmarkError::Weightless)?;
        if T::RegisterQueryOrigin::try_origin(query_origin.clone()).is_err() {
            return Err(BenchmarkError::Override(BenchmarkResult::from_weight(
                Weight::MAX,
            )));
        }

        let query_type = QueryType::WASMContractCallback {
            contract_id: [0u8; 32].into(),
            selector: [0u8; 4],
        };
        let dest = (Parent, Parachain(1000)).into();

        #[block]
        {
            let _ = Pallet::<T>::new_query(
                query_origin,
                QueryConfig {
                    query_type: query_type.clone(),
                    timeout: Bounded::max_value(),
                },
                Box::new(dest),
            );
        }

        assert_last_event::<T>(
            Event::<T>::QueryPrepared {
                query_type,
                // we are sure this will be the first query, so it fine to hardcode here
                query_id: 0,
            }
            .into(),
        );
        Ok(())
    }

    #[benchmark]
    fn on_callback_recieved() {
        let origin: <T as Config>::RuntimeOrigin = <T as pallet_xcm::Config>::RuntimeOrigin::from(
            pallet_xcm::Origin::Response((Parent, Parachain(1000)).into()),
        )
        .into();
        let query_id = 123;
        let query_type = QueryType::NoCallback;
        CallbackQueries::<T>::insert(
            query_id,
            QueryInfo {
                query_type: query_type.clone(),
                // no use of querier, so any arbitary value will be fine
                querier: Here,
            },
        );

        #[block]
        {
            let _ = Pallet::<T>::on_callback_recieved(origin.into(), query_id, Response::Null);
        }

        assert_eq!(CallbackQueries::<T>::get(query_id), None);
        assert_last_event::<T>(
            Event::<T>::CallbackSuccess {
                query_type,
                query_id,
                weight: Weight::zero(),
            }
            .into(),
        );
    }

    impl_benchmark_test_suite! {
        Pallet,
        crate::mock::new_test_ext_with_balances(Vec::new()),
        crate::mock::Test
    }
}
