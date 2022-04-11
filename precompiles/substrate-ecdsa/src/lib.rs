#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{Context, ExitSucceed, PrecompileOutput};
use pallet_evm::Precompile;
use sp_core::ecdsa;
use sp_std::marker::PhantomData;
use sp_std::prelude::*;

use precompile_utils::{
    Bytes, EvmDataReader, EvmDataWriter, EvmResult, FunctionModifier, Gasometer,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    Verify = "verify(bytes,bytes,bytes)",
}

/// A precompile to wrap substrate ecdsa functions.
pub struct SubstrateEcdsaPrecompile<Runtime>(PhantomData<Runtime>);

impl<Runtime: pallet_evm::Config> Precompile for SubstrateEcdsaPrecompile<Runtime> {
    fn execute(
        input: &[u8], //Reminder this is big-endian
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> EvmResult<PrecompileOutput> {
        log::trace!(target: "substrate-ecdsa-precompile", "In SubstrateEcdsa precompile");

        let mut gasometer = Gasometer::new(target_gas);
        let gasometer = &mut gasometer;

        let (mut input, selector) = EvmDataReader::new_with_selector(gasometer, input)?;
        let input = &mut input;

        gasometer.check_function_modifier(context, is_static, FunctionModifier::View)?;

        match selector {
            // Dispatchables
            Action::Verify => Self::verify(input, gasometer, context),
        }
    }
}

impl<Runtime: pallet_evm::Config> SubstrateEcdsaPrecompile<Runtime> {
    fn verify(
        input: &mut EvmDataReader,
        gasometer: &mut Gasometer,
        _: &Context,
    ) -> EvmResult<PrecompileOutput> {
        // Bound check
        input.expect_arguments(gasometer, 3)?;

        // Parse arguments
        let public_bytes: Vec<u8> = input.read::<Bytes>(gasometer)?.into();
        let signature_bytes: Vec<u8> = input.read::<Bytes>(gasometer)?.into();
        let message: Vec<u8> = input.read::<Bytes>(gasometer)?.into();

        // Parse public key 
        let public = if let Ok(public) = ecdsa::Public::try_from(&public_bytes[..]) {
            public
        } else {
            // Return `false` if public key length is wrong
            return Ok(PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                cost: gasometer.used_gas(),
                output: EvmDataWriter::new().write(false).build(),
                logs: Default::default(),
            });
        };

        // Parse signature
        if signature_bytes.len() != 65 {
            // Return `false` if signature length is wrong
            return Ok(PrecompileOutput {
                exit_status: ExitSucceed::Returned,
                cost: gasometer.used_gas(),
                output: EvmDataWriter::new().write(false).build(),
                logs: Default::default(),
            });
        }
        let signature = ecdsa::Signature::from_slice(&signature_bytes[..]);

        log::trace!(
            target: "substrate-ecdsa-precompile",
            "Verify signature {:?} for public {:?} and message {:?}",
            signature, public, message,
        );

        let is_confirmed = sp_io::crypto::ecdsa_verify(&signature, &message[..], &public);

        log::trace!(
            target: "substrate-ecdsa-precompile",
            "Verified signature {:?} is {:?}",
            signature, is_confirmed,
        );

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: gasometer.used_gas(),
            output: EvmDataWriter::new().write(is_confirmed).build(),
            logs: Default::default(),
        })
    }
}
