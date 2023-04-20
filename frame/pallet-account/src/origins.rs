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

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{AccountId32, RuntimeDebug};

/// Derive new origin.
pub trait OriginDeriving<AccountId, Origin> {
    /// Derive new origin depend of account and index
    fn derive(&self, source: &AccountId, index: u32) -> Origin;
}

/// Origin that support native and EVM compatible options.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum NativeAndEVM {
    /// Substrate native origin.
    Native(AccountId32),
    /// The 20-byte length Ethereum like origin.
    H160(sp_core::H160),
}

impl TryInto<AccountId32> for NativeAndEVM {
    type Error = ();
    fn try_into(self) -> Result<AccountId32, Self::Error> {
        match self {
            NativeAndEVM::Native(a) => Ok(a),
            _ => Err(()),
        }
    }
}

impl TryInto<sp_core::H160> for NativeAndEVM {
    type Error = ();
    fn try_into(self) -> Result<sp_core::H160, Self::Error> {
        match self {
            NativeAndEVM::H160(a) => Ok(a),
            _ => Err(()),
        }
    }
}

/// Kind for NativeAndEVM origin.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum NativeAndEVMKind {
    Native,
    H160,
}

impl OriginDeriving<AccountId32, NativeAndEVM> for NativeAndEVMKind {
    fn derive(&self, source: &AccountId32, index: u32) -> NativeAndEVM {
        let salted_source = [source.as_ref(), &index.encode()[..]].concat();
        let derived = sp_core::blake2_256(&salted_source);
        match self {
            NativeAndEVMKind::Native => NativeAndEVM::Native(derived.into()),
            NativeAndEVMKind::H160 => NativeAndEVM::H160(sp_core::H160::from_slice(&derived[..20])),
        }
    }
}
