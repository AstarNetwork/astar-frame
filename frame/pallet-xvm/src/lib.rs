//! # XVM pallet
//!
//! ## Overview
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//!
//! ### Other
//!
//!

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{traits::Member, RuntimeDebug};
use sp_std::prelude::*;

pub mod pallet;
pub use pallet::pallet::*;

/// EVM call adapter.
#[cfg(feature = "evm")]
pub mod evm;

/// Wasm call adapter.
#[cfg(feature = "wasm")]
pub mod wasm;

/// Unique VM identifier.
type VmId = u8;

/// XVM context consist of unique ID and optional execution arguments.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, scale_info::TypeInfo)]
pub struct XvmContext {
    /// Identifier (should be unique for each VM in tuple).
    pub id: VmId,
    /// Encoded VM execution environment.
    pub env: Option<Vec<u8>>,
}

/// The engine that support synchronous smart contract execution.
/// For example, EVM.
pub trait SyncVM<AccountId> {
    /// Unique VM identifier.
    fn id() -> VmId;

    /// Make a call to VM contract and return result or error.
    ///
    ///
    fn xvm_call(
        context: XvmContext,
        from: AccountId,
        to: Vec<u8>,
        input: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>>;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl<AccountId: Member> SyncVM<AccountId> for Tuple {
    fn id() -> VmId {
        Default::default()
    }

    fn xvm_call(
        context: XvmContext,
        from: AccountId,
        to: Vec<u8>,
        input: Vec<u8>,
    ) -> Result<Vec<u8>, Vec<u8>> {
        for_tuples!( #(
            if Tuple::id() == context.id {
                log::trace!(
                    target: "xvm::SyncVm::xvm_call",
                    "VM found, run XVM call: {:?}, {:?}, {:?}, {:?}",
                    context, from, to, input,
                );
                return Tuple::xvm_call(context, from, to, input)
            }
        )* );
        log::trace!(
            target: "xvm::SyncVm::xvm_call",
            "VM with ID {:?} not found", context.id
        );
        Err(b"VM is not found".to_vec())
    }
}

/// The engine that support asynchronous smart contract execution.
/// For example, XCVM.
pub trait AsyncVM<AccountId> {
    /// Unique VM identifier.
    fn id() -> VmId;

    /// Send a message.
    fn xvm_send(context: XvmContext, from: AccountId, to: Vec<u8>, message: Vec<u8>)
        -> bool;

    /// Query for incoming messages.
    fn xvm_query(context: XvmContext, inbox: AccountId) -> Vec<Vec<u8>>;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl<AccountId: Member> AsyncVM<AccountId> for Tuple {
    fn id() -> VmId {
        Default::default()
    }

    fn xvm_send(
        context: XvmContext,
        from: AccountId,
        to: Vec<u8>,
        message: Vec<u8>,
    ) -> bool {
        for_tuples!( #(
            if Tuple::id() == context.id {
                log::trace!(
                    target: "xvm::AsyncVM::xvm_send",
                    "VM found, send message: {:?}, {:?}, {:?}, {:?}",
                    context, from, to, message,
                );
                return Tuple::xvm_send(context, from, to, message)
            }
        )* );
        log::trace!(
            target: "xvm::AsyncVM::xvm_send",
            "VM with ID {:?} not found", context.id
        );
        false
    }

    fn xvm_query(context: XvmContext, inbox: AccountId) -> Vec<Vec<u8>> {
        for_tuples!( #(
            if Tuple::id() == context.id {
                log::trace!(
                    target: "xvm::AsyncVM::xvm_query",
                    "VM found, query messages: {:?} {:?}",
                    context, inbox,
                );
                return Tuple::xvm_query(context, inbox)
            }
        )* );
        log::trace!(
            target: "xvm::AsyncVM::xvm_query",
            "VM with ID {:?} not found", context.id
        );
        Default::default()
    }
}
