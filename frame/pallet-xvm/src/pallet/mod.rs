//! # XVM pallet
//!
//! ## Overview
//!
//!
//! ## Interface
//!
//! ### Dispatchable Function
//!
//!
//! ### Other
//!
//!

#[frame_support::pallet]
#[allow(clippy::module_inception)]
pub mod pallet {
    use crate::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Supported synchronous VM list, for example (EVM, WASM)
        type SyncVM: SyncVM<Self::AccountId>;
        /// Supported asynchronous VM list.
        type AsyncVM: AsyncVM<Self::AccountId>;
        /// General event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        XvmCall { result: Result<Vec<u8>, Vec<u8>> },
        XvmSend { result: bool },
        XvmQuery { result: Vec<Vec<u8>> },
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(100_000)]
        pub fn xvm_call(
            origin: OriginFor<T>,
            context: XvmContext,
            to: Vec<u8>,
            input: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let result = T::SyncVM::xvm_call(context, from, to, input);

            Self::deposit_event(Event::<T>::XvmCall { result });
            Ok(().into())
        }

        #[pallet::weight(100_000)]
        pub fn xvm_send(
            origin: OriginFor<T>,
            context: XvmContext,
            to: Vec<u8>,
            message: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let result = T::AsyncVM::xvm_send(context, from, to, message);

            Self::deposit_event(Event::<T>::XvmSend { result });
            Ok(().into())
        }

        #[pallet::weight(100_000)]
        pub fn xvm_query(
            origin: OriginFor<T>,
            context: XvmContext,
        ) -> DispatchResultWithPostInfo {
            let inbox = ensure_signed(origin)?;
            let result = T::AsyncVM::xvm_query(context, inbox);

            Self::deposit_event(Event::<T>::XvmQuery { result });
            Ok(().into())
        }
    }
}
