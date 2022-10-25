#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{DispatchError, ModuleError};
use sp_std::vec::Vec;

#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug)]
pub enum XvmExecutionResult {
    /// Success
    Success = 0,
    // TODO: expand this with concrete XVM errors
    /// Error not (yet) covered by a dedidacted code
    UnknownError = 255,
}

impl TryFrom<DispatchError> for XvmExecutionResult {
    type Error = DispatchError;

    fn try_from(input: DispatchError) -> Result<Self, Self::Error> {
        let _error_text = match input {
            DispatchError::Module(ModuleError { message, .. }) => message,
            _ => Some("No module error Info"),
        };

        // TODO: expand this with concrete XVM errors (see dapps-staking types for example)
        Ok(XvmExecutionResult::UnknownError)
    }
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, Debug)]
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
