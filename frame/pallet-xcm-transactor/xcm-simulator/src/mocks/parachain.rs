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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    match_types, parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, Currency, Everything, Imbalance,
        Nothing, OnUnbalanced,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_SECOND},
        Weight,
    },
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureSigned,
};
use pallet_contracts::chain_extension::RegisteredChainExtension;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use polkadot_core_primitives::BlakeTwo256;
use sp_core::{ConstBool, H256};
use sp_runtime::{
    generic::Header,
    traits::{AccountIdConversion, Convert, IdentityLookup},
    AccountId32, Perbill, RuntimeDebug,
};
use sp_std::prelude::*;

use super::msg_queue::*;
use xcm::latest::prelude::{AssetId as XcmAssetId, *};
use xcm_builder::{
    Account32Hash, AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, ConvertedConcreteId, CurrencyAdapter,
    EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, FungiblesAdapter, IsConcrete,
    NoChecking, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
    SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
    SovereignSignedViaLocation, TakeWeightCredit, WithComputedOrigin,
};
use xcm_executor::{traits::JustTry, XcmExecutor};

use xcm_primitives::{
    AssetLocationIdConverter, FixedRateOfForeignAsset, ReserveAssetFilter, XcmFungibleFeeHandler,
};

pub type AccountId = AccountId32;
pub type Balance = u128;
pub type AssetId = u128;

pub type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub type ShidenAssetLocationIdConverter = AssetLocationIdConverter<AssetId, XcAssetConfig>;

parameter_types! {
    pub const BlockHashCount: u32 = 250;
}

impl frame_system::Config for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u32;
    type Hash = H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
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

impl pallet_balances::Config for Runtime {
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

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = AssetId;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = ConstU128<10>;
    type MetadataDepositBase = ConstU128<10>;
    type MetadataDepositPerByte = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<10>;
    type ApprovalDeposit = ConstU128<10>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Extra = ();
    type RemoveItemsLimit = ConstU32<100>;
    type CallbackHandle = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

/// Constant values used within the runtime.
pub const MICROSDN: Balance = 1_000_000_000_000;
pub const MILLISDN: Balance = 1_000 * MICROSDN;
/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 0.5 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
    polkadot_primitives::MAX_POV_SIZE as u64,
);

parameter_types! {
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub SS58Prefix: u8 = 5;
}

// TODO: changing depost per item and per byte to `deposit` function will require storage migration it seems
parameter_types! {
    pub const DepositPerItem: Balance = MILLISDN / 1_000_000;
    pub const DepositPerByte: Balance = MILLISDN / 1_000_000;
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
        RuntimeBlockWeights::get().max_block;
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl Convert<Weight, Balance> for Runtime {
    fn convert(w: Weight) -> Balance {
        w.ref_time().into()
    }
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = Randomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    /// The safest default is to allow no calls at all.
    ///
    /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
    /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
    /// change because that would break already deployed contracts. The `Call` structure itself
    /// is not allowed to change the indices of existing pallets, too.
    type CallFilter = Nothing;
    type DepositPerItem = DepositPerItem;
    type DepositPerByte = DepositPerByte;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    /// We are not using the pallet_transaction_payment for simplicity
    type WeightPrice = Self;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = pallet_xcm_transactor::chain_extension::XCMExtension<Self>;
    type DeletionQueueDepth = ConstU32<128>;
    type DeletionWeightLimit = DeletionWeightLimit;
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
}

pub struct BurnFees;
impl OnUnbalanced<NegativeImbalance> for BurnFees {
    /// Payout tips but burn all the fees
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(mut fees_to_burn) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                fees_to_burn.subsume(tips)
            }
            drop(fees_to_burn);
        }
    }
}

#[derive(
    PartialEq, Eq, Copy, Clone, Encode, Decode, MaxEncodedLen, RuntimeDebug, scale_info::TypeInfo,
)]
pub enum SmartContract {
    Wasm(u32),
}

impl Default for SmartContract {
    fn default() -> Self {
        SmartContract::Wasm(0)
    }
}

parameter_types! {
    pub const DappsStakingPalletId: PalletId = PalletId(*b"py/dpsst");
    pub const MaxUnlockingChunks: u32 = 5;
    pub const UnbondingPeriod: u32 = 5;
    pub const MaxEraStakeValues: u32 = 5;
}

impl pallet_dapps_staking::Config for Runtime {
    type Currency = Balances;
    type BlockPerEra = ConstU32<5>;
    type SmartContract = SmartContract;
    type RegisterDeposit = ConstU128<1>;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_dapps_staking::weights::SubstrateWeight<Runtime>;
    type MaxNumberOfStakersPerContract = ConstU32<8>;
    type MinimumStakingAmount = ConstU128<1>;
    type PalletId = DappsStakingPalletId;
    type MinimumRemainingAmount = ConstU128<0>;
    type MaxUnlockingChunks = ConstU32<4>;
    type UnbondingPeriod = ConstU32<2>;
    type MaxEraStakeValues = ConstU32<4>;
    type UnregisteredDappRewardRetention = ConstU32<7>;
}

parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub TreasuryAccountId: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl pallet_xc_asset_config::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type XcAssetChanged = ();
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = pallet_xc_asset_config::weights::SubstrateWeight<Runtime>;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}
parameter_types! {
    pub const ReservedXcmpWeight: Weight = Weight::from_ref_time(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4));
    pub const ReservedDmpWeight: Weight = Weight::from_ref_time(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4));
}

parameter_types! {
    pub RelayNetwork: Option<NetworkId> = Some(NetworkId::Kusama);
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
    pub UniversalLocation: InteriorMultiLocation =
        X2(GlobalConsensus(RelayNetwork::get().unwrap()), Parachain(MsgQueue::parachain_id().into()));
    pub const ShidenLocation: MultiLocation = Here.into_location();
    pub DummyCheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the default `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<polkadot_parachain::primitives::Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
    // Derives a private `Account32` by hashing `("multiloc", received multilocation)`
    Account32Hash<RelayNetwork, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type CurrencyTransactor = CurrencyAdapter<
    // Use this currency:
    Balances,
    // Use this currency when it is a fungible asset matching the given location or name:
    IsConcrete<ShidenLocation>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Balances`.
    (),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
    // Use this fungibles implementation:
    Assets,
    // Use this currency when it is a fungible asset matching the given location or name:
    ConvertedConcreteId<AssetId, Balance, ShidenAssetLocationIdConverter, JustTry>,
    // Convert an XCM MultiLocation into a local account id:
    LocationToAccountId,
    // Our chain's account ID type (we can't get away without mentioning it explicitly):
    AccountId,
    // We don't track any teleports of `Assets`.
    NoChecking,
    // We don't track any teleports of `Assets`.
    DummyCheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
    // recognised.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognised.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
    // transaction from the Root origin.
    ParentAsSuperuser<RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    pallet_xcm::XcmPassthrough<RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
);

parameter_types! {
    pub const UnitWeightCost: Weight = Weight::from_ref_time(10);
    pub const MaxInstructions: u32 = 100;
    pub NativePerSecond: (XcmAssetId, u128, u128) = (Concrete(ShidenLocation::get()), 1_000_000_000_000, 1024 * 1024);
}

pub type XcmRouter = super::ParachainXcmRouter<MsgQueue>;

match_types! {
    pub type ParentOrParentsPlurality: impl Contains<MultiLocation> = {
        MultiLocation { parents: 1, interior: Here } |
        MultiLocation { parents: 1, interior: X1(Plurality { .. }) }
    };
}

pub type XcmBarrier = (
    TakeWeightCredit,
    AllowTopLevelPaidExecutionFrom<Everything>,
    // This will first calculate the derived origin, before checking it against the barrier implementation
    WithComputedOrigin<AllowTopLevelPaidExecutionFrom<Everything>, UniversalLocation, ConstU32<8>>,
    // Parent and its plurality get free execution
    AllowUnpaidExecutionFrom<ParentOrParentsPlurality>,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    // Subscriptions for version tracking are OK.
    AllowSubscriptionsFrom<Everything>,
);

// Used to handle XCM fee deposit into treasury account
pub type ShidenXcmFungibleFeeHandler = XcmFungibleFeeHandler<
    AccountId,
    ConvertedConcreteId<AssetId, Balance, ShidenAssetLocationIdConverter, JustTry>,
    Assets,
    TreasuryAccountId,
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = ReserveAssetFilter;
    type IsTeleporter = ();
    type UniversalLocation = UniversalLocation;
    type Barrier = XcmBarrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = (
        FixedRateOfFungible<NativePerSecond, ()>,
        FixedRateOfForeignAsset<XcAssetConfig, ShidenXcmFungibleFeeHandler>,
    );
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;

    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = ConstU32<64>;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = RuntimeCall;
    type SafeCallFilter = Everything;
}

impl mock_msg_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Everything;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;

    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<0>;
    type WeightInfo = pallet_xcm::TestWeightInfo;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
    pub const CallbackGasLimit: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);
}

impl pallet_xcm_transactor::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type CallbackHandler = XcmTransact;
    type RegisterQueryOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type MaxCallbackWeight = CallbackGasLimit;
}

impl RegisteredChainExtension<Runtime>
    for pallet_xcm_transactor::chain_extension::XCMExtension<Runtime>
{
    const ID: u16 = pallet_xcm_transactor::chain_extension::XCM_EXTENSION_ID;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        MsgQueue: mock_msg_queue::{Pallet, Storage, Event<T>},
        PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        XcAssetConfig: pallet_xc_asset_config::{Pallet, Call, Storage, Event<T>},
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
        DappsStaking: pallet_dapps_staking::{Pallet, Call, Event<T>},
        Randomness: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        XcmTransact: pallet_xcm_transactor::{Pallet, Call, Storage, Event<T>},
    }
);
