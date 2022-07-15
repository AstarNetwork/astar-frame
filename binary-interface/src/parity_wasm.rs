use crate::primitives::*;

pub struct ParityWasmVM;
impl ParityWasmVM {
    pub fn call(
        contract_address: AccountId,
        func: String,
        input_args: Vec<XvmEncoding>,
        remaining_allowed_gas: u128,
    ) {
        // convert params to `pallet_contracts` format and call it
        let binary_input_args: Vec<u8>; // = input_args.encode_for_parity_wasm();

        // pallet_contracts::call(..., binary_input_args)
    }
}
