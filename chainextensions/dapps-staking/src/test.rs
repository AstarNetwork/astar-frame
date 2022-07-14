use super::*;

use frame_support::{
    construct_runtime, parameter_types,
    assert_ok,
    traits::{ConstU32, Nothing},
    weights::{constants::WEIGHT_PER_SECOND, RuntimeDbWeight, Weight},
    PalletId,
};
use frame_system::limits::{BlockLength, BlockWeights};
use sp_core::{H160, H256, Bytes};

use codec::{Decode, Encode};
use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup, Hash},
    AccountId32, Perbill,
};

/// Contract extension for Local Chain-Extension
// use pallet_chain_extension_dapps_staking::DappsStakingExtension;
use chain_extension_traits::ChainExtensionExec;
use pallet_contracts::{
    chain_extension::{
        ChainExtension, Environment, Ext, InitState, RetVal, SysConfig, UncheckedFrom,
    },
    weights::WeightInfo,
    DefaultContractAccessWeight,
};
use std::cell::RefCell;

use pallet_dapps_staking::weights;

pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type EraIndex = u32;
pub(crate) const MILLIAST: Balance = 1_000_000_000_000_000;
pub(crate) const AST: Balance = 1_000 * MILLIAST;

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

pub const READ_WEIGHT: u64 = 3;
pub const WRITE_WEIGHT: u64 = 7;

pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2u8; 32]);

pub const GAS_LIMIT: Weight = 100_000_000_000;

/// Charge fee for stored bytes and items.
pub const fn deposit(items: u32, bytes: u32) -> Balance {
    (items as Balance + bytes as Balance) * MILLIAST / 1_000_000
}

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

parameter_types! {
    // pub const BlockHashCount: u64 = 250;
    pub const TestWeights: RuntimeDbWeight = RuntimeDbWeight {
        read: READ_WEIGHT,
        write: WRITE_WEIGHT,
    };

    pub const BlockHashCount: BlockNumber = 2400;
    /// We allow for 1 seconds of compute with a 2 second average block time.
    pub RuntimeBlockWeights: BlockWeights = BlockWeights
        ::with_sensible_defaults(1 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockLength: BlockLength = BlockLength
        ::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
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
    type DbWeight = TestWeights;
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

impl pallet_randomness_collective_flip::Config for TestRuntime {}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for TestRuntime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
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
    pub const DepositPerItem: Balance = deposit(1, 0);
    pub const DepositPerByte: Balance = deposit(0, 1);
    pub const MaxValueSize: u32 = 16 * 1024;
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
        RuntimeBlockWeights::get().max_block;
    // The weight needed for decoding the queue should be less or equal than a fifth
    // of the overall weight dedicated to the lazy deletion.
    pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
        <TestRuntime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1)
        -
        <TestRuntime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0))) / 5) as u32;
    pub Schedule: pallet_contracts::Schedule<TestRuntime> = Default::default();
}

impl pallet_contracts::Config for TestRuntime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type Event = Event;
    type Call = Call;
    /// The safest default is to allow no calls at all.
    ///
    /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
    /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
    /// change because that would break already deployed contracts. The `Call` structure itself
    /// is not allowed to change the indices of existing pallets, too.
    type CallFilter = Nothing;
    type DepositPerItem = DepositPerItem;
    type DepositPerByte = DepositPerByte;
    type CallStack = [pallet_contracts::Frame<Self>; 31];
    type WeightPrice = ();
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = TestExtension;
    type DeletionQueueDepth = DeletionQueueDepth;
    type DeletionWeightLimit = DeletionWeightLimit;
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type ContractAccessWeight = DefaultContractAccessWeight<RuntimeBlockWeights>;
    type MaxCodeLen = ConstU32<{ 128 * 1024 }>;
    type RelaxedMaxCodeLen = ConstU32<{ 256 * 1024 }>;
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

    // pub(crate) fn with_balances(mut self, balances: Vec<(AccountId32, Balance)>) -> Self {
    //     self.balances = balances;
    //     self
    // }
}


thread_local! {
	static TEST_EXTENSION: RefCell<TestExtension> = Default::default();
}

pub struct TestExtension {
	enabled: bool,
	last_seen_buffer: Vec<u8>,
	last_seen_inputs: (u32, u32, u32, u32),
}

impl TestExtension {
	fn disable() {
		TEST_EXTENSION.with(|e| e.borrow_mut().enabled = false)
	}

	fn last_seen_buffer() -> Vec<u8> {
		TEST_EXTENSION.with(|e| e.borrow().last_seen_buffer.clone())
	}

	fn last_seen_inputs() -> (u32, u32, u32, u32) {
		TEST_EXTENSION.with(|e| e.borrow().last_seen_inputs.clone())
	}
}

impl Default for TestExtension {
	fn default() -> Self {
		Self { enabled: true, last_seen_buffer: vec![], last_seen_inputs: (0, 0, 0, 0) }
	}
}

enum ExtensionId {
    DappsStaking = 34,
}

impl TryFrom<u32> for ExtensionId {
    type Error = DispatchError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            34 => return Ok(ExtensionId::DappsStaking),
            _ => return Err(DispatchError::Other("Unimplemented ChainExtension pallet")),
        }
    }
}

impl ChainExtension<TestRuntime> for TestExtension {
    fn call<E: Ext>(func_id: u32, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
    where
        E: Ext<T = TestRuntime>,
        <E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
    {
        let pallet_id = ExtensionId::try_from(func_id / 100)?;
        let func_id_matcher = func_id % 100;
        match pallet_id {
            ExtensionId::DappsStaking => {
                DappsStakingExtension::execute_func::<E>(func_id_matcher, env)?;
            }
        }
        Ok(RetVal::Converging(0))
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
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        DappsStaking: pallet_dapps_staking::{Pallet, Call, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
    }
);

// ----------------------- Helper functions ---------------------------------------
/// Load a given wasm module represented by a .wat file and returns a wasm binary contents along
/// with it's hash.
///
/// The fixture files are located under the `fixtures/` directory.
fn compile_module<T>(fixture_name: &str) -> wat::Result<(Vec<u8>, <T::Hashing as Hash>::Output)>
where
	T: frame_system::Config,
{
	let fixture_path = ["fixtures/", fixture_name, ".wat"].concat();
	let wasm_binary = wat::parse_file(fixture_path)?;
	let code_hash = T::Hashing::hash(&wasm_binary);
	Ok((wasm_binary, code_hash))
}

// ----------------------- T E S T ---------------------------------------

#[test]
fn current_era_is_ok() {
    ExternalityBuilder::default().build().execute_with(|| {
        // Set a block number in mid of an era
        System::set_block_number(2);

        assert_eq!(0u32, DappsStaking::current_era());
    })
}


#[test]
fn chain_extension_works() {
	let (code, hash) = compile_module::<TestRuntime>("dapps_staking").unwrap();
    ExternalityBuilder::default().build().execute_with(|| {
		let min_balance = <TestRuntime as pallet_contracts::Config>::Currency::minimum_balance();
		let _ = Balances::deposit_creating(&ALICE, 1000 * min_balance);
		assert_ok!(Contracts::instantiate_with_code(
			Origin::signed(ALICE),
			min_balance * 100,
			GAS_LIMIT,
			None,
			code,
			vec![],
			vec![],
		),);
		let addr = Contracts::contract_address(&ALICE, &hash, &[]);

		// The contract takes a up to 2 byte buffer where the first byte passed is used as
		// as func_id to the chain extension which behaves differently based on the
		// func_id.

		// 0 = read input buffer and pass it through as output
		let result =
			Contracts::bare_call(ALICE, addr.clone(), 0, GAS_LIMIT, None, vec![0, 99], false);
		let gas_consumed = result.gas_consumed;
		assert_eq!(TestExtension::last_seen_buffer(), vec![0, 99]);
		assert_eq!(result.result.unwrap().data, Bytes(vec![0, 99]));
	});
}