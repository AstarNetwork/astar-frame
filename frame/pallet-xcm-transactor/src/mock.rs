// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

use frame_support::{
    construct_runtime, parameter_types,
    traits::{Currency, Everything, Nothing},
    weights::Weight,
};
use pallet_contracts::{chain_extension::RegisteredChainExtension, DefaultAddressGenerator, Frame};
use pallet_xcm::TestWeightInfo;
use parity_scale_codec::Encode;
use polkadot_parachain::primitives::Id as ParaId;
use polkadot_runtime_parachains::origin;
use sp_core::{ConstU32, ConstU64, H256};
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};
pub use sp_std::{cell::RefCell, fmt::Debug, marker::PhantomData};
use xcm::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, Case, ChildParachainAsNative, ChildParachainConvertsVia,
    ChildSystemParachainAsSuperuser, CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin,
    FixedRateOfFungible, FixedWeightBounds, IsConcrete, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_ce_primitives::XCM_EXTENSION_ID;
use xcm_executor::XcmExecutor;

use crate::{self as pallet_xcm_transactor, chain_extension::XCMExtension};

pub type AccountId = AccountId32;
pub type Balance = u128;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type BalanceOf<T> = <<T as pallet_contracts::Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::Balance;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
        ParasOrigin: origin::{Pallet, Origin},
        XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config},
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        XcmTransact: pallet_xcm_transactor::{Pallet, Call, Storage, Event<T>},
    }
);

impl pallet_insecure_randomness_collective_flip::Config for Test {}
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

thread_local! {
    pub static SENT_XCM: RefCell<Vec<(MultiLocation, Xcm<()>)>> = RefCell::new(Vec::new());
}
// pub(crate) fn sent_xcm() -> Vec<(MultiLocation, Xcm<()>)> {
//     SENT_XCM.with(|q| (*q.borrow()).clone())
// }
// pub(crate) fn take_sent_xcm() -> Vec<(MultiLocation, Xcm<()>)> {
//     SENT_XCM.with(|q| {
//         let mut r = Vec::new();
//         std::mem::swap(&mut r, &mut *q.borrow_mut());
//         r
//     })
// }
/// Sender that never returns error, always sends
pub struct TestSendXcm;
impl SendXcm for TestSendXcm {
    type Ticket = (MultiLocation, Xcm<()>);
    fn validate(
        dest: &mut Option<MultiLocation>,
        msg: &mut Option<Xcm<()>>,
    ) -> SendResult<(MultiLocation, Xcm<()>)> {
        let pair = (dest.take().unwrap(), msg.take().unwrap());
        Ok((pair, MultiAssets::new()))
    }
    fn deliver(pair: (MultiLocation, Xcm<()>)) -> Result<XcmHash, SendError> {
        let hash = fake_message_hash(&pair.1);
        SENT_XCM.with(|q| q.borrow_mut().push(pair));
        Ok(hash)
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type BlockWeights = ();
    type BlockLength = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = Everything;
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub const RelayLocation: MultiLocation = Here.into_location();
    pub const AnyNetwork: Option<NetworkId> = None;
    pub UniversalLocation: InteriorMultiLocation = Here;
    pub UnitWeightCost: u64 = 1_000;
}

pub type SovereignAccountOf = (
    ChildParachainConvertsVia<ParaId, AccountId>,
    AccountId32Aliases<AnyNetwork, AccountId>,
);

pub type LocalAssetTransactor =
    XcmCurrencyAdapter<Balances, IsConcrete<RelayLocation>, SovereignAccountOf, AccountId, ()>;

type LocalOriginConverter = (
    SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
    ChildParachainAsNative<origin::Origin, RuntimeOrigin>,
    SignedAccountId32AsNative<AnyNetwork, RuntimeOrigin>,
    ChildSystemParachainAsSuperuser<ParaId, RuntimeOrigin>,
);

parameter_types! {
    pub const BaseXcmWeight: Weight = Weight::from_parts(1_000, 1_000);
    pub CurrencyPerSecondPerByte: (AssetId, u128, u128) = (Concrete(RelayLocation::get()), 1, 1);
    pub TrustedAssets: (MultiAssetFilter, MultiLocation) = (All.into(), Here.into());
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

pub type Barrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    AllowKnownQueryResponses<XcmPallet>,
    AllowSubscriptionsFrom<Everything>,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = TestSendXcm;
    type AssetTransactor = LocalAssetTransactor;
    type OriginConverter = LocalOriginConverter;
    type IsReserve = ();
    type IsTeleporter = Case<TrustedAssets>;
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type Trader = FixedRateOfFungible<CurrencyPerSecondPerByte, ()>;
    type ResponseHandler = XcmPallet;
    type AssetTrap = XcmPallet;
    type AssetLocker = ();
    type AssetExchanger = ();
    type AssetClaims = XcmPallet;
    type SubscriptionService = XcmPallet;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, AnyNetwork>;

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<MultiLocation> = Some(Parachain(1000).into());
}

impl pallet_xcm::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = TestSendXcm;
    type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Everything;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type TrustedLockers = ();
    type SovereignAccountOf = AccountId32Aliases<(), AccountId32>;
    type Currency = Balances;
    type CurrencyMatcher = IsConcrete<RelayLocation>;
    type MaxLockers = frame_support::traits::ConstU32<8>;
    type WeightInfo = TestWeightInfo;
    #[cfg(feature = "runtime-benchmarks")]
    type ReachableDest = ReachableDest;
}

parameter_types! {
    pub const CallbackGasLimit: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);
    pub const DeletionWeightLimit: Weight = Weight::from_ref_time(500_000_000_000);
    pub static UnstableInterface: bool = true;
    pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
    pub static DepositPerByte: BalanceOf<Test> = 1;
    pub const DepositPerItem: BalanceOf<Test> = 1;
}

impl<W: pallet_xcm_transactor::CEWeightInfo> RegisteredChainExtension<Test>
    for XCMExtension<Test, W>
{
    const ID: u16 = XCM_EXTENSION_ID;
}

impl pallet_contracts::Config for Test {
    type Time = Timestamp;
    type Randomness = Randomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Nothing;
    type CallStack = [Frame<Self>; 5];
    type WeightPrice = ();
    type WeightInfo = ();
    type ChainExtension = XCMExtension<Self, pallet_xcm_transactor::ChainExtensionWeight<Self>>;
    type DeletionQueueDepth = ConstU32<1024>;
    type DeletionWeightLimit = DeletionWeightLimit;
    type Schedule = Schedule;
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type AddressGenerator = DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = UnstableInterface;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
}

impl pallet_xcm_transactor::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type CallbackHandler = XcmTransact;
    type RegisterQueryOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type MaxCallbackWeight = CallbackGasLimit;
    type WeightInfo = pallet_xcm_transactor::SubstrateWeight<Self>;
}

impl origin::Config for Test {}

pub(crate) fn new_test_ext_with_balances(
    balances: Vec<(AccountId, Balance)>,
) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> { balances }
        .assimilate_storage(&mut t)
        .unwrap();

    <pallet_xcm::GenesisConfig as frame_support::traits::GenesisBuild<Test>>::assimilate_storage(
        &pallet_xcm::GenesisConfig {
            safe_xcm_version: Some(2),
        },
        &mut t,
    )
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub(crate) fn fake_message_hash<T>(message: &Xcm<T>) -> XcmHash {
    message.using_encoded(sp_io::hashing::blake2_256)
}
