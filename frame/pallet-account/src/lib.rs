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
//! An accout abstraction pallet makes it possible to derive new blockchain based
//! account for your existed external owned account (seed phrase based). The onchain
//! account could be derived to multiple address spaces: H160 and SS58. For example,
//! it makes possible predictable interaction between substrate native account and
//! EVM smart contracts.
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//! * new_origin() - create new origin for account
//! * proxy_call() - make proxy call with derived account as origin
//! * meta_call() - make meta call with dedicated payer account
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub mod origins;
pub use origins::*;

pub mod pallet;
pub use pallet::pallet::*;

pub mod weights;
pub use weights::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
