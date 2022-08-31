use crate::{self as pallet_block_reward, NegativeImbalanceOf};

use frame_support::{
    construct_runtime, parameter_types, sp_io::TestExternalities, traits::Currency, traits::Get,
    PalletId,
};

use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};

pub(crate) type AccountId = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const EXISTENTIAL_DEPOSIT: Balance = 2;

construct_runtime!(
    pub struct TestRuntime
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        Timestamp: pallet_timestamp,
        BlockReward: pallet_block_reward,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for TestRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Index = u64;
    type Call = Call;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MaxLocks: u32 = 4;
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for TestRuntime {
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 3;
}

impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// A fairly high block reward so we can detect slight changes in reward distribution
// due to TVL changes.
pub(crate) const BLOCK_REWARD: Balance = 1_000_000;

// This gives us enough flexibility to get valid percentages by controlling issuance.
pub(crate) const TVL: Balance = 1_000_000_000;

// Fake accounts used to simulate reward beneficiaries balances
pub(crate) const TREASURY_POT: PalletId = PalletId(*b"moktrsry");
pub(crate) const COLLATOR_POT: PalletId = PalletId(*b"mokcolat");
pub(crate) const STAKERS_POT: PalletId = PalletId(*b"mokstakr");
pub(crate) const DAPPS_POT: PalletId = PalletId(*b"mokdapps");

// Type used as TVL provider
pub struct TvlProvider();
impl Get<Balance> for TvlProvider {
    fn get() -> Balance {
        TVL
    }
}

// Type used as beneficiary payout handle
pub struct BeneficiaryPayout();
impl pallet_block_reward::BeneficiaryPayout<NegativeImbalanceOf<TestRuntime>>
    for BeneficiaryPayout
{
    fn treasury(reward: NegativeImbalanceOf<TestRuntime>) {
        Balances::resolve_creating(&TREASURY_POT.into_account_truncating(), reward);
    }

    fn collators(reward: NegativeImbalanceOf<TestRuntime>) {
        Balances::resolve_creating(&COLLATOR_POT.into_account_truncating(), reward);
    }

    fn dapps_staking(
        stakers: NegativeImbalanceOf<TestRuntime>,
        dapps: NegativeImbalanceOf<TestRuntime>,
    ) {
        Balances::resolve_creating(&STAKERS_POT.into_account_truncating(), stakers);
        Balances::resolve_creating(&DAPPS_POT.into_account_truncating(), dapps);
    }
}

parameter_types! {
    pub const RewardAmount: Balance = BLOCK_REWARD;
}

impl pallet_block_reward::Config for TestRuntime {
    type Event = Event;
    type Currency = Balances;
    type RewardAmount = RewardAmount;
    type DappsStakingTvlProvider = TvlProvider;
    type BeneficiaryPayout = BeneficiaryPayout;
    type WeightInfo = ();
}

pub struct ExternalityBuilder;

impl ExternalityBuilder {
    pub fn build() -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        // This will cause some initial issuance
        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: vec![(1, 9000), (2, 800), (3, 10000)],
        }
        .assimilate_storage(&mut storage)
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}
