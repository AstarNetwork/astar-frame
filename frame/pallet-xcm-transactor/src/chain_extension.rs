use crate::{Config, Pallet, QueryConfig};
use frame_support::{traits::EnsureOrigin, DefaultNoBound};
use frame_system::RawOrigin;
// use log;
use pallet_contracts::chain_extension::{
    ChainExtension, Environment, Ext, InitState, Result as DispatchResult, RetVal, SysConfig,
};
use pallet_xcm::{Pallet as XcmPallet, WeightInfo};
use parity_scale_codec::Encode;
use sp_core::Get;
use sp_std::prelude::*;
use xcm::prelude::*;
pub use xcm_ce_primitives::{
    Command::{self, *},
    PreparedExecution, ValidateSendInput, ValidatedSend,
    XcmCeError::{self, *},
    XCM_EXTENSION_ID,
};
use xcm_executor::traits::WeightBounds;

type RuntimeCallOf<T> = <T as SysConfig>::RuntimeCall;

macro_rules! unwrap {
    ($val:expr, $err:expr) => {
        match $val {
            Ok(inner) => inner,
            Err(_) => return Ok(RetVal::Converging($err.into())),
        }
    };
}

#[derive(DefaultNoBound)]
pub struct XCMExtension<T: Config> {
    prepared_execute: Option<PreparedExecution<RuntimeCallOf<T>>>,
    validated_send: Option<ValidatedSend>,
}

impl<T: Config> ChainExtension<T> for XCMExtension<T>
where
    <T as SysConfig>::AccountId: AsRef<[u8; 32]>,
{
    fn enabled() -> bool {
        true
    }

    fn call<E>(&mut self, env: Environment<E, InitState>) -> DispatchResult<RetVal>
    where
        E: Ext<T = T>,
    {
        match unwrap!(env.func_id().try_into(), InvalidCommand) {
            PrepareExecute => self.prepare_execute(env),
            Execute => self.execute(env),
            ValidateSend => self.validate_send(env),
            Send => self.send(env),
            NewQuery => self.new_query(env),
            TakeResponse => self.take_response(env),
            PalletAccountId => self.pallet_account_id(env),
        }
    }
}

impl<T: Config> XCMExtension<T> {
    fn prepare_execute<E: Ext<T = T>>(
        &mut self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        // input parsing
        let len = env.in_len();
        let input: VersionedXcm<RuntimeCallOf<T>> = env.read_as_unbounded(len)?;

        let mut xcm = unwrap!(input.try_into(), BadVersion);
        // calculate the weight
        let weight = unwrap!(T::Weigher::weight(&mut xcm), CannotWeigh);

        // save the prepared xcm
        self.prepared_execute = Some(PreparedExecution { xcm, weight });
        // write the output to buffer
        weight.using_encoded(|w| env.write(w, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    fn execute<E: Ext<T = T>>(
        &mut self,
        mut env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let input = unwrap!(
            self.prepared_execute.as_ref().take().ok_or(()),
            PreparationMissing
        );
        // charge for xcm weight
        let charged = env.charge_weight(input.weight)?;

        // TODO: find better way to get origin
        //       https://github.com/paritytech/substrate/pull/13708
        let origin = RawOrigin::Signed(env.ext().address().clone());
        // ensure xcm execute origin
        let origin_location = unwrap!(
            T::ExecuteXcmOrigin::ensure_origin(origin.into()),
            BadVersion
        );

        let hash = input.xcm.using_encoded(sp_io::hashing::blake2_256);
        // execute XCM
        // NOTE: not using pallet_xcm::execute here because it does not return XcmError
        //       which is needed to ensure xcm execution success
        let outcome = T::XcmExecutor::execute_xcm_in_credit(
            origin_location,
            input.xcm.clone(),
            hash,
            input.weight,
            input.weight,
        );

        // adjust with actual weights used
        env.adjust_weight(charged, outcome.weight_used());
        // revert for anything but a complete execution
        match outcome {
            Outcome::Complete(_) => Ok(RetVal::Converging(Success.into())),
            _ => Ok(RetVal::Converging(ExecutionFailed.into())),
        }
    }

    fn validate_send<E: Ext<T = T>>(
        &mut self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        let len = env.in_len();
        let input: ValidateSendInput = env.read_as_unbounded(len)?;

        let dest = unwrap!(input.dest.try_into(), BadVersion);
        let xcm: Xcm<()> = unwrap!(input.xcm.try_into(), BadVersion);
        // validate and get fees required to send
        let (_, asset) = unwrap!(
            validate_send::<T::XcmRouter>(dest, xcm.clone()),
            SendValidateFailed
        );

        // save the validated input
        self.validated_send = Some(ValidatedSend { dest, xcm });
        // write the fees to output
        VersionedMultiAssets::from(asset).using_encoded(|a| env.write(a, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    fn send<E: Ext<T = T>>(
        &mut self,
        mut env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let input = unwrap!(
            self.validated_send.as_ref().take().ok_or(()),
            PreparationMissing
        );

        let base_weight = <T as pallet_xcm::Config>::WeightInfo::send();
        env.charge_weight(base_weight)?;

        // TODO: find better way to get origin
        //       https://github.com/paritytech/substrate/pull/13708
        let origin = RawOrigin::Signed(env.ext().address().clone());

        // send the xcm
        unwrap!(
            XcmPallet::<T>::send(
                origin.into(),
                Box::new(input.dest.into()),
                Box::new(xcm::VersionedXcm::V3(input.xcm.clone())),
            ),
            SendFailed
        );

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    fn new_query<E: Ext<T = T>>(&self, env: Environment<E, InitState>) -> DispatchResult<RetVal>
    where
        <T as SysConfig>::AccountId: AsRef<[u8; 32]>,
    {
        let mut env = env.buf_in_buf_out();
        let len = env.in_len();
        let (query_config, dest): (
            QueryConfig<T::AccountId, T::BlockNumber>,
            VersionedMultiLocation,
        ) = env.read_as_unbounded(len)?;

        let dest: MultiLocation = unwrap!(dest.try_into(), BadVersion);

        // TODO: find better way to get origin
        //       https://github.com/paritytech/substrate/pull/13708
        let origin = RawOrigin::Signed(env.ext().address().clone());
        // ensure origin is allowed to make queries
        unwrap!(
            T::RegisterQueryOrigin::ensure_origin(origin.into()),
            InvalidOrigin
        );

        // register the query
        let query_id: u64 = Pallet::<T>::new_query(
            query_config,
            AccountId32 {
                id: *env.ext().address().as_ref(),
                network: T::Network::get(),
            },
            dest,
        )?;

        // write the query_id to buffer
        query_id.using_encoded(|q| env.write(q, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    fn take_response<E: Ext<T = T>>(
        &self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        let query_id: u64 = env.read_as()?;
        let response = unwrap!(
            pallet_xcm::Pallet::<T>::take_response(query_id)
                .map(|ret| ret.0)
                .ok_or(()),
            XcmCeError::NoResponse
        );
        VersionedResponse::from(response).using_encoded(|r| env.write(r, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }

    fn pallet_account_id<E: Ext<T = T>>(
        &self,
        env: Environment<E, InitState>,
    ) -> DispatchResult<RetVal> {
        let mut env = env.buf_in_buf_out();
        Pallet::<T>::account_id().using_encoded(|r| env.write(r, true, None))?;

        Ok(RetVal::Converging(XcmCeError::Success.into()))
    }
}
