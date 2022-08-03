#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;

// TODO: add RuntimeDebug? Maybe only use it when not building for runtime?
/// Virtual machine identifier in the XVM context
#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo)]
pub enum VmId {
    Unsupported,
    FrontierEvm,
    ParityWasm,
}

impl Default for VmId {
    fn default() -> Self {
        VmId::Unsupported
    }
}
