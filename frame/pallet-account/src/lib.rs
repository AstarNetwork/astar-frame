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

//! # Account abstraction pallet
//!
//! ## Overview
//!
//! An accout abstraction pallet make possible to derive new blockchain based
//! account for your existed external owned account (seed phrase based). The onchain
//! account could be drived to multiple address spaces: H160 and SS58. For example,
//! it makes possible predictable interaction between substrate native account and
//! EVM smart contracts.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! * proxy() - make proxy call with derived account as origin
//!
//!
//! ### Other
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub mod pallet;
pub use pallet::pallet::*;

pub mod weights;
pub use weights::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/*
/// A method to derive new account from existed one
pub trait AccountDeriving<AccountId> {
    /// Derive new account from existed one
    fn derive(&self, source: &AccountId) -> AccountId;
}

/// Use simple salt and Blake2 hash for account deriving.
#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, scale_info::TypeInfo, RuntimeDebug)]
pub struct SimpleSalt(pub u32);

impl<AccountId: AsRef<[u8]> + From<[u8; 32]>> AccountDeriving<AccountId> for SimpleSalt {
    fn derive(&self, source: &AccountId) -> AccountId {
        let salted_source = [source.as_ref(), &self.encode()[..]].concat();
        sp_core::blake2_256(&salted_source).into()
    }
}
*/
