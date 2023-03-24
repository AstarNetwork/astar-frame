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
use sp_runtime::{DispatchError, traits::Verify};
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use parity_scale_codec::Encode;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;
use signing_chain_extension_types::{Outcome, SigType};

enum Func {
    Verify,
}

impl TryFrom<u16> for Func {
    type Error = DispatchError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Func::Verify),
            _ => Err(DispatchError::Other(
                "CryptoExtension: Unimplemented func_id",
            )),
        }
    }
}

/// Crypto signing chain extension.
pub struct SigningExtension<T>(PhantomData<T>);

impl<T> Default for SigningExtension<T> {
    fn default() -> Self {
        SigningExtension(PhantomData)
    }
}

impl<T> ChainExtension<T> for SigningExtension<T>
where
    T: pallet_contracts::Config,
    <T as SysConfig>::AccountId: From<[u8; 32]>,
{
    fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = T>,
    {
        let func_id = env.func_id().try_into()?;
        let mut env = env.buf_in_buf_out();

        match func_id {
            Func::Verify => {
                let (sig_type, signature, msg, pubkey): (SigType, Vec<u8>, Vec<u8>, Vec<u8>) =
                    env.read_as_unbounded(env.in_len())?;

                let result = match sig_type {
                    SigType::Sr25519 => {
                        // 64 bytes
                        let sig = match sp_core::sr25519::Signature::try_from(signature.as_slice()) {
                            Ok(sig) => sig,
                            Err(_) => return Ok(RetVal::Converging(Outcome::InvalidSignature as u32)),
                        };
                        // 32 bytes
                        let pubkey = match sp_core::sr25519::Public::try_from(pubkey.as_slice()) {
                            Ok(pubkey) => pubkey,
                            Err(_) => return Ok(RetVal::Converging(Outcome::InvalidPubkey as u32)),
                        };
                        sig.verify(msg.as_slice(), &pubkey)
                    }
                    SigType::Ed25519 => {
                        // 64 bytes
                        let sig = match sp_core::ed25519::Signature::try_from(signature.as_slice()) {
                            Ok(sig) => sig,
                            Err(_) => return Ok(RetVal::Converging(Outcome::InvalidSignature as u32)),
                        };
                        // 32 bytes
                        let pubkey = match sp_core::ed25519::Public::try_from(pubkey.as_slice()) {
                            Ok(pubkey) => pubkey,
                            Err(_) => return Ok(RetVal::Converging(Outcome::InvalidPubkey as u32)),
                        };
                        sig.verify(msg.as_slice(), &pubkey)
                    }
                    SigType::Ecdsa => {
                        // 65 bytes
                        let sig = match sp_core::ecdsa::Signature::try_from(signature.as_slice()) {
                            Ok(sig) => sig,
                            Err(_) => return Ok(RetVal::Converging(Outcome::InvalidSignature as u32)),
                        };
                        // 33 bytes
                        let pubkey = match sp_core::ecdsa::Public::try_from(pubkey.as_slice()) {
                            Ok(pubkey) => pubkey,
                            Err(_) => return Ok(RetVal::Converging(Outcome::InvalidPubkey as u32)),
                        };
                        sig.verify(msg.as_slice(), &pubkey)
                    },
                };
                env.write(&result.encode(), false, None)?;
            }
        }

        Ok(RetVal::Converging(Outcome::Success as u32))
    }
}
