#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

use ink_env::{DefaultEnvironment, Environment};
use ink_prelude::vec::Vec;

#[derive(Clone, PartialEq, Eq, scale::Encode, scale::Decode, Debug)]
pub struct XvmCallArgs {
    /// virtual machine identifier
    pub vm_id: u8,
    /// Call destination (e.g. address)
    pub to: Vec<u8>,
    /// Encoded call params
    pub input: Vec<u8>,
}

pub const FRONTIER_VM_ID: u8 = 0x0F;
pub const PARITY_WASM_VM_ID: u8 = 0x1F;

#[ink::chain_extension]
pub trait XvmChainExtension {
    type ErrorCode = ExtensionError;

    // Not possible for chain extension to depend on associated type, it has to be concrete it seems? TODO
    #[ink(extension = 1)]
    fn xvm_call(params: XvmCallArgs) -> Result<(), ExtensionError>;
}

#[derive(Debug, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ExtensionError {
    XvmCallFailed,
    EncodingFailed,
}

impl ink_env::chain_extension::FromStatusCode for ExtensionError {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            _ => Err(Self::XvmCallFailed), // TODO: how to be more precise here?
        }
    }
}

impl From<scale::Error> for ExtensionError {
    fn from(_: scale::Error) -> Self {
        Self::EncodingFailed
    }
}

pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize = <DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <DefaultEnvironment as Environment>::AccountId;
    type Balance = <DefaultEnvironment as Environment>::Balance;
    type Hash = <DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = XvmChainExtension;
}

/// Now we need to tell our contract to use our custom environment.
///
/// This will give us access to the chain extension that we've defined.
#[ink::contract(env = crate::CustomEnvironment)]
mod xvm_example {

    use super::*;
    use scale::Encode;

    #[ink(storage)]
    pub struct XvmChainExtensionContract {}

    impl XvmChainExtensionContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {}
        }

        #[ink(message)]
        pub fn call_evm(&mut self, address: AccountId) -> Result<(), ExtensionError> {
            self.env()
                .extension()
                .xvm_call(XvmCallArgs {
                    vm_id: FRONTIER_VM_ID,
                    to: address.encode(),
                    input: Default::default(),
                })
                .map_err(|_| ExtensionError::XvmCallFailed)?;

            Ok(())
        }
    }
}
