use crate::primitives::*;
use crate::xvm::*;

use codec::{Decode, Encode};

/// Just a simplifaction of EVM precompile, for the sake of prototype.
///
/// We can read array of bytes and then parse it into concrete XVM encoding types.
pub struct EvmPrecompile;
impl EvmPrecompile {
    /// This will be used to handle the `call` from smart contract
    pub fn call() {
        // just for the sake of example, these will be parsed as concrete args
        let vm_selector = VmSelector::FrontierEvm;
        let contract_address = ContractAddress::EVM(0xffff1230);
        let func_descriptor = "other_vm_function(int,bool,bytes)".to_string();

        // calculate from allowed gas and already consumed gas
        let remaining_allowed_gas = 123456789_u128;

        let input_args: Vec<XvmEncoding> = Default::default(); // will parse the binary input from the contract

        // call to other VM
        XVM::call(
            vm_selector,
            contract_address,
            func_descriptor,
            input_args,
            remaining_allowed_gas,
        );
    }

    /// Used by smart contract to get encoding for `bool` value.
    /// Should be simpler than implementing this logic in Solidity (which would also inflate the size of contract)
    pub fn encode_bool(arg: bool) -> Vec<u8> {
        XvmEncoding::Bool(arg).encode()
    }

    pub fn encode_address(arg: H160) -> Vec<u8> {
        XvmEncoding::H160(arg).encode()
    }

    pub fn encode_uint32(arg: u32) -> Vec<u8> {
        XvmEncoding::U32(arg).encode()
    }
}
