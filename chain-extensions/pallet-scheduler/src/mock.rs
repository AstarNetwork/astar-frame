use crate::weights::SubstrateWeight;
use crate::BalanceOf;
use crate::{weights, SchedulerExtension};
use frame_support::{ord_parameter_types, parameter_types, sp_io, traits::{ConstU32, ConstU64, EqualPrivilegeOnly, Nothing, OnFinalize, OnInitialize}, weights::{constants::RocksDbWeight, Weight}};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
use pallet_contracts::chain_extension::RegisteredChainExtension;
use pallet_contracts::{DefaultAddressGenerator, Frame};
use sp_core::crypto::AccountId32;
use sp_keystore::{testing::KeyStore, KeystoreExt};
use sp_runtime::testing::{Header, H256};
use sp_runtime::traits::{AccountIdLookup, BlakeTwo256, Convert, IdentityLookup};
use sp_runtime::{generic, Perbill};
use std::sync::Arc;
use frame_support::traits::{Contains, EitherOfDiverse};
use pallet_scheduler::WeightInfo;

pub type BlockNumber = u32;
pub type Balance = u128;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(
            Weight::from_ref_time(2_000_000_000_000).set_proof_size(u64::MAX),
        );
}
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = BlockWeights;
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Index = u32;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type RuntimeCall = RuntimeCall;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId32;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = generic::Header<u32, BlakeTwo256>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData =  pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_preimage::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Test>;
    type Currency = ();
    type ManagerOrigin = EnsureRoot<AccountId32>;
    type BaseDeposit = ();
    type ByteDeposit = ();
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        BlockWeights::get().max_block;
}

ord_parameter_types! {
	pub const One: u64 = 1;
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
    fn service_agendas_base() -> Weight {
        Weight::from_ref_time(0b0000_0001)
    }
    fn service_agenda_base(i: u32) -> Weight {
        Weight::from_ref_time((i << 8) as u64 + 0b0000_0010)
    }
    fn service_task_base() -> Weight {
        Weight::from_ref_time(0b0000_0100)
    }
    fn service_task_periodic() -> Weight {
        Weight::from_ref_time(0b0000_1100)
    }
    fn service_task_named() -> Weight {
        Weight::from_ref_time(0b0001_0100)
    }
    fn service_task_fetched(s: u32) -> Weight {
        Weight::from_ref_time((s << 8) as u64 + 0b0010_0100)
    }
    fn execute_dispatch_signed() -> Weight {
        Weight::from_ref_time(0b0100_0000)
    }
    fn execute_dispatch_unsigned() -> Weight {
        Weight::from_ref_time(0b1000_0000)
    }
    fn schedule(_s: u32) -> Weight {
        Weight::from_ref_time(50)
    }
    fn cancel(_s: u32) -> Weight {
        Weight::from_ref_time(50)
    }
    fn schedule_named(_s: u32) -> Weight {
        Weight::from_ref_time(50)
    }
    fn cancel_named(_s: u32) -> Weight {
        Weight::from_ref_time(50)
    }
}

impl pallet_scheduler::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureSigned<AccountId32>;
    type MaxScheduledPerBlock = ConstU32<10>;
    type WeightInfo = TestWeightInfo;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = Preimage;
}

impl pallet_randomness_collective_flip::Config for Test {}

parameter_types! {
    pub const DeletionWeightLimit: Weight = Weight::from_ref_time(500_000_000_000);
    pub static UnstableInterface: bool = true;
    pub Schedule: pallet_contracts::Schedule<Test> = Default::default();
    pub static DepositPerByte: BalanceOf<Test> = 1;
	pub const DepositPerItem: BalanceOf<Test> = 1;
}

impl pallet_contracts::Config for Test {
    type Time = Timestamp;
    type Randomness = Randomness;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type CallFilter = Nothing;
    type CallStack = [Frame<Self>; 31];
    type WeightPrice = Self;
    type WeightInfo = ();
    type ChainExtension = SchedulerExtension<Self, SubstrateWeight<Self>>;
    type DeletionQueueDepth = ConstU32<1024>;
    type DeletionWeightLimit = DeletionWeightLimit;
    type Schedule = Schedule;
    type DepositPerByte = DepositPerByte;
    type DepositPerItem = DepositPerItem;
    type AddressGenerator = DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 128 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = UnstableInterface;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
}

impl<W: weights::WeightInfo> RegisteredChainExtension<Test> for SchedulerExtension<Test, W> {
    const ID: u16 = 03;
}

parameter_types! {
    pub static ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<1>;
    type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Randomness: pallet_randomness_collective_flip::{Pallet, Storage},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
        Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
        Logger: logger::{Pallet, Call, Event<T>},
    }
);

pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const GAS_LIMIT: Weight = Weight::from_ref_time(100_000_000_000).set_proof_size(700000u64);

impl Convert<Weight, BalanceOf<Self>> for Test {
    fn convert(w: Weight) -> BalanceOf<Self> {
        w.ref_time().into()
    }
}

pub struct ExtBuilder {
    existential_deposit: u64,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            existential_deposit: ExistentialDeposit::get(),
        }
    }
}

impl ExtBuilder {
    pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
        self.existential_deposit = existential_deposit;
        self
    }
    pub fn set_associated_consts(&self) {
        EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
    }
    pub fn build(self) -> sp_io::TestExternalities {
        use env_logger::{Builder, Env};
        let env = Env::new().default_filter_or("runtime=debug");
        let _ = Builder::from_env(env).is_test(true).try_init();
        self.set_associated_consts();
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> { balances: vec![] }
            .assimilate_storage(&mut t)
            .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.register_extension(KeystoreExt(Arc::new(KeyStore::new())));
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub fn run_to_block(n: u32) {
    println!("entry: {:?}", System::block_number());
    while System::block_number() < n {
        println!("looop: {:?}", n);
        Scheduler::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        Scheduler::on_initialize(System::block_number());
    }
}

// ________________________________________

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    t.into()
}

pub fn root() -> OriginCaller { frame_system::RawOrigin::Root.into()
}

pub type LoggerCall = logger::Call<Test>;

// Logger module to track execution.
#[frame_support::pallet]
pub mod logger {
    use super::OriginCaller;
    use frame_support::{pallet_prelude::*, parameter_types};
    use frame_support::traits::OriginTrait;
    use frame_system::pallet_prelude::*;

    parameter_types! {
		static Log: Vec<(OriginCaller, u32)> = Vec::new();
	}
    pub fn log() -> Vec<(OriginCaller, u32)> {
        Log::get().clone()
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Logged(u32, Weight),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
        where
            <T as frame_system::Config>::RuntimeOrigin: OriginTrait<PalletsOrigin = OriginCaller>,
    {
        #[pallet::call_index(0)]
        #[pallet::weight(*weight)]
        pub fn log(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
            Self::deposit_event(Event::Logged(i, weight));
            Log::mutate(|log| {
                log.push((origin.caller().clone(), i));
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(*weight)]
        pub fn log_without_filter(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
            Self::deposit_event(Event::Logged(i, weight));
            Log::mutate(|log| {
                log.push((origin.caller().clone(), i));
            });
            Ok(())
        }
    }
}

impl logger::Config for Test {
    type RuntimeEvent = RuntimeEvent;
}

