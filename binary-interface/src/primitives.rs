use codec::{Decode, Encode};

pub enum VmSelector {
    FrontierEvm,
    ParityWasm,
}

// incorrect but this is just for the sake of example
pub type H160 = u128;
pub type AccountId = u128;

pub enum ContractAddress {
    EVM(H160),
    ParityWasm(AccountId),
}

/// We encapsulate each time so we can unpack it once it's been encoded.
#[derive(Clone, PartialEq, Encode, Decode)]
pub enum XvmEncoding {
    U8(u8),
    U32(u32),
    Bool(bool),
    Bytes(Vec<u8>),
    H160(H160),
    // and so on...
}
