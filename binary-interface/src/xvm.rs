use crate::parity_wasm::*;
use crate::primitives::*;

pub struct XVM;
impl XVM {
    pub fn call(
        vm_selector: VmSelector,
        contract_address: ContractAddress,
        func: String,
        input_args: Vec<XvmEncoding>,
        remaining_allowed_gas: u128,
    ) {
        match vm_selector {
            VmSelector::ParityWasm => {
                let contract_address: AccountId = 0xDEADBEEF;
                ParityWasmVM::call(contract_address, func, input_args, remaining_allowed_gas)
            }
            _ => { /* not needed for prototype */ }
        }
    }
}
