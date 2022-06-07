use super::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{Currency, OnFinalize, OnInitialize},
    PalletId,
};
use pallet_dapps_staking::weights;
use pallet_evm::{
    AddressMapping, EnsureAddressNever, EnsureAddressRoot, PrecompileResult, PrecompileSet,
};
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32,
};
extern crate alloc;

pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type EraIndex = u32;
pub(crate) const MILLIAST: Balance = 1_000_000_000_000_000;
pub(crate) const AST: Balance = 1_000 * MILLIAST;
pub(crate) const TEST_CONTRACT: [u8; 20] = H160::repeat_byte(0x09).to_fixed_bytes();

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const MAX_NUMBER_OF_STAKERS: u32 = 4;
/// Value shouldn't be less than 2 for testing purposes, otherwise we cannot test certain corner cases.
pub(crate) const MINIMUM_STAKING_AMOUNT: Balance = 10 * AST;
pub(crate) const MINIMUM_REMAINING_AMOUNT: Balance = 1;
pub(crate) const MAX_UNLOCKING_CHUNKS: u32 = 4;
pub(crate) const UNBONDING_PERIOD: EraIndex = 3;
pub(crate) const MAX_ERA_STAKE_VALUES: u32 = 10;

// Do note that this needs to at least be 3 for tests to be valid. It can be greater but not smaller.
pub(crate) const BLOCKS_PER_ERA: BlockNumber = 3;

pub(crate) const REGISTER_DEPOSIT: Balance = 10 * AST;

pub(crate) const STAKER_BLOCK_REWARD: Balance = 531911;
pub(crate) const DAPP_BLOCK_REWARD: Balance = 773333;

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Encode,
    Decode,
    Debug,
    MaxEncodedLen,
    Serialize,
    Deserialize,
    derive_more::Display,
    scale_info::TypeInfo,
)]

pub enum TestAccount {
    Empty,
    Alex,
    Bobo,
    Dino,
}

impl Default for TestAccount {
    fn default() -> Self {
        Self::Empty
    }
}

// needed for associated type in pallet_evm
impl AddressMapping<AccountId32> for TestAccount {
    fn into_account_id(h160_account: H160) -> AccountId32 {
        match h160_account {
            a if a == H160::repeat_byte(0x01) => TestAccount::Alex.into(),
            a if a == H160::repeat_byte(0x02) => TestAccount::Bobo.into(),
            a if a == H160::repeat_byte(0x03) => TestAccount::Dino.into(),
            _ => TestAccount::Empty.into(),
        }
    }
}

impl TestAccount {
    pub(crate) fn to_h160(&self) -> H160 {
        match self {
            Self::Empty => Default::default(),
            Self::Alex => H160::repeat_byte(0x01),
            Self::Bobo => H160::repeat_byte(0x02),
            Self::Dino => H160::repeat_byte(0x03),
        }
    }
}

trait H160Conversion {
    fn to_h160(&self) -> H160;
}

impl H160Conversion for AccountId32 {
    fn to_h160(&self) -> H160 {
        let x = self.encode()[31];
        H160::repeat_byte(x)
    }
}

impl From<TestAccount> for AccountId32 {
    fn from(x: TestAccount) -> Self {
        match x {
            TestAccount::Alex => AccountId32::from([1u8; 32]),
            TestAccount::Bobo => AccountId32::from([2u8; 32]),
            TestAccount::Dino => AccountId32::from([3u8; 32]),
            _ => AccountId32::from([0u8; 32]),
        }
    }
}

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
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<AccountId32>;
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
    pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for TestRuntime {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 4];
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

pub fn precompile_address() -> H160 {
    H160::from_low_u64_be(0x5001)
}

#[derive(Debug, Clone, Copy)]
pub struct DappPrecompile<R>(PhantomData<R>);

impl<R> PrecompileSet for DappPrecompile<R>
where
    R: pallet_evm::Config,
    DappsStakingWrapper<R>: Precompile,
{
    fn execute(
        &self,
        address: H160,
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> Option<PrecompileResult> {
        match address {
            a if a == precompile_address() => Some(DappsStakingWrapper::<R>::execute(
                input, target_gas, context, is_static,
            )),
            _ => None,
        }
    }

    fn is_precompile(&self, address: sp_core::H160) -> bool {
        address == precompile_address()
    }
}

parameter_types! {
    pub PrecompilesValue: DappPrecompile<TestRuntime> = DappPrecompile(Default::default());
}

impl pallet_evm::Config for TestRuntime {
    type FeeCalculator = ();
    type GasWeightMapping = ();
    type CallOrigin = EnsureAddressRoot<AccountId32>;
    type WithdrawOrigin = EnsureAddressNever<AccountId32>;
    type AddressMapping = TestAccount;
    type Currency = Balances;
    type Event = Event;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type PrecompilesType = DappPrecompile<TestRuntime>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ();
    type OnChargeTransaction = ();
    type BlockGasLimit = ();
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type FindAuthor = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, Debug, scale_info::TypeInfo)]
pub enum MockSmartContract<AccountId32> {
    Evm(sp_core::H160),
    Wasm(AccountId32),
}

impl<AccountId32> Default for MockSmartContract<AccountId32> {
    fn default() -> Self {
        MockSmartContract::Evm(H160::repeat_byte(0x00))
    }
}

impl<AccountId32> pallet_dapps_staking::IsContract for MockSmartContract<AccountId32> {
    fn is_valid(&self) -> bool {
        match self {
            MockSmartContract::Wasm(_account) => false,
            MockSmartContract::Evm(_account) => true,
        }
    }
}

parameter_types! {
    pub const RegisterDeposit: Balance = REGISTER_DEPOSIT;
    pub const BlockPerEra: BlockNumber = BLOCKS_PER_ERA;
    pub const MaxNumberOfStakersPerContract: u32 = MAX_NUMBER_OF_STAKERS;
    pub const MinimumStakingAmount: Balance = MINIMUM_STAKING_AMOUNT;
    pub const DappsStakingPalletId: PalletId = PalletId(*b"mokdpstk");
    pub const MinimumRemainingAmount: Balance = MINIMUM_REMAINING_AMOUNT;
    pub const MaxUnlockingChunks: u32 = MAX_UNLOCKING_CHUNKS;
    pub const UnbondingPeriod: EraIndex = UNBONDING_PERIOD;
    pub const MaxEraStakeValues: u32 = MAX_ERA_STAKE_VALUES;
}

impl pallet_dapps_staking::Config for TestRuntime {
    type Event = Event;
    type Currency = Balances;
    type BlockPerEra = BlockPerEra;
    type RegisterDeposit = RegisterDeposit;
    type SmartContract = MockSmartContract<AccountId32>;
    type WeightInfo = weights::SubstrateWeight<TestRuntime>;
    type MaxNumberOfStakersPerContract = MaxNumberOfStakersPerContract;
    type MinimumStakingAmount = MinimumStakingAmount;
    type PalletId = DappsStakingPalletId;
    type MinimumRemainingAmount = MinimumRemainingAmount;
    type MaxUnlockingChunks = MaxUnlockingChunks;
    type UnbondingPeriod = UnbondingPeriod;
    type MaxEraStakeValues = MaxEraStakeValues;
}

pub struct ExternalityBuilder {
    balances: Vec<(AccountId32, Balance)>,
}

impl Default for ExternalityBuilder {
    fn default() -> ExternalityBuilder {
        ExternalityBuilder { balances: vec![] }
    }
}

impl ExternalityBuilder {
    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<TestRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<TestRuntime> {
            balances: self.balances,
        }
        .assimilate_storage(&mut storage)
        .ok();

        let mut ext = TestExternalities::from(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }

    pub(crate) fn with_balances(mut self, balances: Vec<(AccountId32, Balance)>) -> Self {
        self.balances = balances;
        self
    }
}

construct_runtime!(
    pub enum TestRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Evm: pallet_evm::{Pallet, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        DappsStaking: pallet_dapps_staking::{Pallet, Call, Storage, Event<T>},
    }
);

/// Used to run to the specified block number
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        DappsStaking::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        // This is performed outside of dapps staking but we expect it before on_initialize
        payout_block_rewards();
        DappsStaking::on_initialize(System::block_number());
    }
}

/// Used to run the specified number of blocks
pub fn run_for_blocks(n: u64) {
    run_to_block(System::block_number() + n);
}

/// Advance blocks to the beginning of an era.
///
/// Function has no effect if era is already passed.
pub fn advance_to_era(n: EraIndex) {
    while DappsStaking::current_era() < n {
        run_for_blocks(1);
    }
}

/// Initialize first block.
/// This method should only be called once in a UT otherwise the first block will get initialized multiple times.
pub fn initialize_first_block() {
    // This assert prevents method misuse
    assert_eq!(System::block_number(), 1 as BlockNumber);

    // This is performed outside of dapps staking but we expect it before on_initialize
    payout_block_rewards();
    DappsStaking::on_initialize(System::block_number());
    run_to_block(2);
}

/// Returns total block rewards that goes to dapps-staking.
/// Contains both `dapps` reward and `stakers` reward.
pub fn joint_block_reward() -> Balance {
    STAKER_BLOCK_REWARD + DAPP_BLOCK_REWARD
}

/// Payout block rewards to stakers & dapps
fn payout_block_rewards() {
    DappsStaking::rewards(
        Balances::issue(STAKER_BLOCK_REWARD.into()),
        Balances::issue(DAPP_BLOCK_REWARD.into()),
    );
}

/// default evm context
pub fn default_context() -> fp_evm::Context {
    fp_evm::Context {
        address: Default::default(),
        caller: Default::default(),
        apparent_value: U256::zero(),
    }
}

/// returns call struct to be used with evm calls
pub fn evm_call(source: AccountId32, input: Vec<u8>) -> pallet_evm::Call<TestRuntime> {
    pallet_evm::Call::call {
        source: source.to_h160(),
        target: precompile_address(),
        input,
        value: U256::zero(),
        gas_limit: u64::max_value(),
        max_fee_per_gas: 0.into(),
        max_priority_fee_per_gas: Some(U256::zero()),
        nonce: None,
        access_list: Vec::new(),
    }
}
