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

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod contracts {
    use ink::{env::DefaultEnvironment, storage::Mapping};
    use xcm::{latest::Weight, prelude::*};
    pub use xcm_ce_sdk::{
        types::{QueryConfig, ValidateSendInput},
        Error as XcmCEError, XcmExtension as _XcmExtension,
    };

    type XcmExtension = _XcmExtension<DefaultEnvironment>;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Contracts {
        value: bool,
        expecting_query: Mapping<QueryId, bool>,
    }

    impl Contracts {
        #[ink(constructor, selector = 0xFFFFFFFF)]
        pub fn default() -> Self {
            Self {
                ..Default::default()
            }
        }

        #[ink(message, selector = 0x11111111)]
        pub fn execute(&mut self, xcm: VersionedXcm<()>) -> Result<Weight, XcmCEError> {
            let weight = XcmExtension::prepare_execute(xcm)?;
            ink::env::debug_println!("[1/2] Prepared XCM");

            XcmExtension::execute()?;
            ink::env::debug_println!("[2/2] Execute XCM");

            Ok(weight)
        }

        #[ink(message, selector = 0x22222222)]
        pub fn send(
            &mut self,
            input: ValidateSendInput,
        ) -> Result<VersionedMultiAssets, XcmCEError> {
            let fees = XcmExtension::validate_send(input)?;
            ink::env::debug_println!("[1/2] Validate Send XCM");

            XcmExtension::send()?;
            ink::env::debug_println!("[2/2] Send XCM");

            Ok(fees)
        }

        #[ink(message, selector = 0x33333333)]
        pub fn query(
            &mut self,
            config: QueryConfig<AccountId, BlockNumber>,
            dest: VersionedMultiLocation,
        ) -> Result<QueryId, XcmCEError> {
            ink::env::debug_println!("[1/3] Registering Query..., {config:?}");
            let query_id = XcmExtension::new_query(config, dest)?;
            ink::env::debug_println!("[2/3] Registered Query");

            self.expecting_query.insert(query_id, &true);
            ink::env::debug_println!("[3/3] Save Query");

            Ok(query_id)
        }

        #[ink(message, selector = 0x44444444)]
        pub fn poll_response(&mut self, query_id: QueryId) -> Result<Response, XcmCEError> {
            ink::env::debug_println!("[1/1] Response Recieved for QueryId - {query_id}");
            XcmExtension::take_response(query_id)
        }

        #[ink(message, selector = 0x55555555)]
        pub fn handle_response(
            &mut self,
            query_id: QueryId,
            _responder: MultiLocation,
            _response: Response,
        ) {
            ink::env::debug_println!("[1/1] Response Recieved for QueryId - {query_id}");
            assert!(XcmExtension::pallet_account_id() == self.env().caller());
            match self.expecting_query.get(query_id) {
                Some(expecting) if expecting == true => {
                    // NOTE: do not delete storage, because storage deposit
                    //       refund will fail.
                    // self.expecting_query.remove(query_id);
                    self.value = !self.value;
                }
                _ => {
                    panic!("Not expecting response");
                }
            }
        }

        #[ink(message, selector = 0x66666666)]
        pub fn get(&self) -> bool {
            self.value
        }
    }
}
