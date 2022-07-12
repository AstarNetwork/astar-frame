#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{PrecompileHandle, PrecompileOutput};
use pallet_evm::Precompile;
use sp_core::{crypto::UncheckedFrom, sr25519, H256};
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use precompile_utils::{
    succeed, Bytes, EvmDataWriter, EvmResult, FunctionModifier, PrecompileHandleExt,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    Verify = "verify(bytes32,bytes,bytes)",
}

/// A precompile to wrap substrate sr25519 functions.
pub struct Sr25519Precompile<Runtime>(PhantomData<Runtime>);

impl<Runtime: pallet_evm::Config> Precompile for Sr25519Precompile<Runtime> {
    fn execute(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "sr25519-precompile", "In sr25519 precompile");

        let selector = handle.read_selector()?;

        handle.check_function_modifier(FunctionModifier::View)?;

        match selector {
            // Dispatchables
            Action::Verify => Self::verify(handle),
        }
    }
}

impl<Runtime: pallet_evm::Config> Sr25519Precompile<Runtime> {
    fn verify(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = handle.read_input()?;
        input.expect_arguments(3)?;

        // Parse arguments
        let public: sr25519::Public = sr25519::Public::unchecked_from(input.read::<H256>()?);
        let signature_bytes: Vec<u8> = input.read::<Bytes>()?.into();
        let message: Vec<u8> = input.read::<Bytes>()?.into();

        // Parse signature
        let signature_opt = sr25519::Signature::from_slice(&signature_bytes[..]);

        let signature = if let Some(sig) = signature_opt {
            sig
        } else {
            // Return `false` if signature length is wrong
            return Ok(succeed(EvmDataWriter::new().write(false).build()));
        };

        log::trace!(
            target: "sr25519-precompile",
            "Verify signature {:?} for public {:?} and message {:?}",
            signature, public, message,
        );

        let is_confirmed = sp_io::crypto::sr25519_verify(&signature, &message[..], &public);

        log::trace!(
            target: "sr25519-precompile",
            "Verified signature {:?} is {:?}",
            signature, is_confirmed,
        );

        Ok(succeed(EvmDataWriter::new().write(is_confirmed).build()))
    }
}
