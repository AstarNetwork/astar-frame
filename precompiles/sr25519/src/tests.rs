use hex_literal::hex;
use std::assert_matches::assert_matches;

use crate::mock::*;
use crate::*;

use fp_evm::PrecompileFailure;
use pallet_evm::PrecompileSet;
use precompile_utils::{Bytes, EvmDataWriter};
use sp_core::{sr25519, Pair, H256, U256};

fn precompiles() -> TestPrecompileSet<Runtime> {
    PrecompilesValue::get()
}

#[test]
fn wrong_argument_count_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        // This selector is only three bytes long when four are required.
        let bogus_selector = vec![1u8, 2u8, 3u8];

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &bogus_selector,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert { output, ..}))
                if output == b"tried to parse selector out of bounds",
        );

        let pair = sr25519::Pair::from_seed(b"12345678901234567890123456789012");
        let public = pair.public();
        let input = EvmDataWriter::new_with_selector(Action::Verify)
            .write(H256::from(public))
            .build();

        assert_matches!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Err(PrecompileFailure::Revert { output, ..}))
                if output == b"input doesn't match expected length",
        );
    });
}

#[test]
fn wrong_signature_length_returns_false() {
    ExtBuilder::default().build().execute_with(|| {
        let pair = sr25519::Pair::from_seed(b"12345678901234567890123456789012");
        let public = pair.public();
        let signature = hex!["0042"];
        let message = hex!["00"];

        let input = EvmDataWriter::new_with_selector(Action::Verify)
            .write(H256::from(public))
            .write(Bytes::from(&signature[..]))
            .write(Bytes::from(&message[..]))
            .build();

        let output = PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: EvmDataWriter::new().write(U256::from(0u64)).build(),
            cost: Default::default(),
            logs: Default::default(),
        };

        assert_eq!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Ok(output)),
        );
    });
}

#[test]
fn bad_signature_returns_false() {
    ExtBuilder::default().build().execute_with(|| {
        let pair = sr25519::Pair::from_seed(b"12345678901234567890123456789012");
        let public = pair.public();
        let message = hex!("2f8c6129d816cf51c374bc7f08c3e63ed156cf78aefb4a6550d97b87997977ee00000000000000000200d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a4500000000000000");
        let signature = pair.sign(&message[..]);
        assert!(sr25519::Pair::verify(&signature, &message[..], &public));

        let bad_message = hex!["00"];

        let input = EvmDataWriter::new_with_selector(Action::Verify)
            .write(H256::from(public))
            .write(Bytes::from(<sr25519::Signature as AsRef<[u8]>>::as_ref(&signature)))
            .write(Bytes::from(&bad_message[..]))
            .build();

        let output = PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: EvmDataWriter::new().write(U256::from(0u64)).build(),
            cost: Default::default(),
            logs: Default::default(),
        };

        assert_eq!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Ok(output)),
        );
    });
}

#[test]
fn substrate_test_vector_works() {
    ExtBuilder::default().build().execute_with(|| {
        let pair = sr25519::Pair::from_seed(b"12345678901234567890123456789012");
        let public = pair.public();
        assert_eq!(
            public,
            sr25519::Public::from_raw(hex!(
                "741c08a06f41c596608f6774259bd9043304adfa5d3eea62760bd9be97634d63"
            ))
        );
        let message = hex!("2f8c6129d816cf51c374bc7f08c3e63ed156cf78aefb4a6550d97b87997977ee00000000000000000200d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a4500000000000000");
        let signature = pair.sign(&message[..]);
        assert!(sr25519::Pair::verify(&signature, &message[..], &public));

        let input = EvmDataWriter::new_with_selector(Action::Verify)
            .write(H256::from(public))
            .write(Bytes::from(<sr25519::Signature as AsRef<[u8]>>::as_ref(&signature)))
            .write(Bytes::from(&message[..]))
            .build();

        let output = PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            output: EvmDataWriter::new().write(U256::from(1u64)).build(),
            cost: Default::default(),
            logs: Default::default(),
        };

        assert_eq!(
            precompiles().execute(
                TestAccount::Precompile.into(),
                &input,
                None,
                &evm_test_context(),
                false,
            ),
            Some(Ok(output)),
        );
    });
}
