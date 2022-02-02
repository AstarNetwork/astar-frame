//! Dapps staking FRAME Pallet.

use super::*;
use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        Currency, ExistenceRequirement, Get, Imbalance, LockIdentifier, LockableCurrency,
        OnUnbalanced, ReservableCurrency, WithdrawReasons,
    },
    weights::Weight,
    PalletId,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use sp_runtime::{
    traits::{AccountIdConversion, CheckedAdd, Saturating, Zero},
    ArithmeticError, Perbill,
};
use sp_std::convert::From;

const STAKING_ID: LockIdentifier = *b"dapstake";

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// The balance type of this pallet.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    #[pallet::generate_store(pub(crate) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    // Negative imbalance type of this pallet.
    type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;

    impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
        fn on_nonzero_unbalanced(block_reward: NegativeImbalanceOf<T>) {
            let dapps_part = T::DeveloperRewardPercentage::get() * block_reward.peek();
            let stakers_part = block_reward.peek().saturating_sub(dapps_part);

            BlockRewardAccumulator::<T>::mutate(|accumulated_reward| {
                accumulated_reward.dapps = accumulated_reward.dapps.saturating_add(dapps_part);
                accumulated_reward.stakers =
                    accumulated_reward.stakers.saturating_add(stakers_part);
            });

            T::Currency::resolve_creating(&Self::account_id(), block_reward);
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The staking balance.
        type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>
            + ReservableCurrency<Self::AccountId>;

        // type used for Accounts on EVM and on Substrate
        type SmartContract: IsContract + Parameter + Member;

        /// Number of blocks per era.
        #[pallet::constant]
        type BlockPerEra: Get<BlockNumberFor<Self>>;

        /// Minimum bonded deposit for new contract registration.
        #[pallet::constant]
        type RegisterDeposit: Get<BalanceOf<Self>>;

        /// Percentage of reward paid to developer.
        #[pallet::constant]
        type DeveloperRewardPercentage: Get<Perbill>;

        /// Maximum number of unique stakers per contract.
        #[pallet::constant]
        type MaxNumberOfStakersPerContract: Get<u32>;

        /// Minimum amount user must stake on contract.
        /// User can stake less if they already have the minimum staking amount staked on that particular contract.
        #[pallet::constant]
        type MinimumStakingAmount: Get<BalanceOf<Self>>;

        /// Dapps staking pallet Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Minimum amount that should be left on staker account after staking.
        #[pallet::constant]
        type MinimumRemainingAmount: Get<BalanceOf<Self>>;

        /// Max number of unlocking chunks per account Id <-> contract Id pairing.
        /// If value is zero, unlocking becomes impossible.
        #[pallet::constant]
        type MaxUnlockingChunks: Get<u32>;

        /// Number of eras that need to pass until unstaked value can be withdrawn.
        /// Current era is always counted as full era (regardless how much blocks are remaining).
        /// When set to `0`, it's equal to having no unbonding period.
        #[pallet::constant]
        type UnbondingPeriod: Get<u32>;

        /// Max number of unique `EraStake` values that can exist for a `(staker, contract)` pairing.
        /// When stakers claims rewards, they will either keep the number of `EraStake` values the same or they will reduce them by one.
        /// Stakers cannot add an additional `EraStake` value by calling `bond&stake` or `unbond&unstake` if they've reached the max number of values.
        ///
        /// This ensures that history doesn't grow indefinitely - if there are too many chunks, stakers should first claim their former rewards
        /// before adding additional `EraStake` values.
        #[pallet::constant]
        type MaxEraStakeValues: Get<u32>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::storage]
    #[pallet::getter(fn migration_state_v2)]
    pub type MigrationStateV2<T: Config> =
        StorageValue<_, migrations::v2::MigrationState, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn migration_state_v3)]
    pub type MigrationStateV3<T: Config> =
        StorageValue<_, migrations::v3::MigrationState, ValueQuery>;

    #[pallet::storage]
    pub type MigrationUndergoingUnbonding<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pallet_disabled)]
    pub type PalletDisabled<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Bonded amount for the staker
    #[pallet::storage]
    #[pallet::getter(fn ledger)]
    pub type Ledger<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountLedger<BalanceOf<T>>, ValueQuery>;

    /// The current era index.
    #[pallet::storage]
    #[pallet::getter(fn current_era)]
    pub type CurrentEra<T> = StorageValue<_, EraIndex, ValueQuery>;

    /// Accumulator for block rewards during an era. It is reset at every new era
    #[pallet::storage]
    #[pallet::getter(fn block_reward_accumulator)]
    pub type BlockRewardAccumulator<T> = StorageValue<_, RewardInfo<BalanceOf<T>>, ValueQuery>;

    #[pallet::type_value]
    pub fn ForceEraOnEmpty() -> Forcing {
        Forcing::NotForcing
    }

    /// Mode of era forcing.
    #[pallet::storage]
    #[pallet::getter(fn force_era)]
    pub type ForceEra<T> = StorageValue<_, Forcing, ValueQuery, ForceEraOnEmpty>;

    /// Registered developer accounts points to coresponding contract
    #[pallet::storage]
    #[pallet::getter(fn registered_contract)]
    pub(crate) type RegisteredDevelopers<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::SmartContract>;

    /// Registered dapp points to the developer who registered it
    #[pallet::storage]
    #[pallet::getter(fn dapp_info)]
    pub(crate) type RegisteredDapps<T: Config> =
        StorageMap<_, Blake2_128Concat, T::SmartContract, DAppInfo<T::AccountId>>;

    /// Legacy, don't use.
    /// TODO: remove in future upgrades
    #[pallet::storage]
    pub type EraRewardsAndStakes<T: Config> =
        StorageMap<_, Twox64Concat, EraIndex, migrations::v3::OldEraRewardAndStake<BalanceOf<T>>>;

    /// Total staked, locked & rewarded for a paticular era
    #[pallet::storage]
    #[pallet::getter(fn general_era_info)]
    pub type GeneralEraInfo<T: Config> =
        StorageMap<_, Twox64Concat, EraIndex, EraInfo<BalanceOf<T>>>;

    /// Stores amount staked and stakers for a contract per era
    #[pallet::storage]
    #[pallet::getter(fn contract_era_stake)]
    pub type ContractEraStake<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::SmartContract,
        Twox64Concat,
        EraIndex,
        EraStakingPoints<BalanceOf<T>>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn staker_info)]
    pub(crate) type StakersInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::SmartContract,
        StakerInfo<BalanceOf<T>>,
        ValueQuery,
    >;

    /// Stores the current pallet storage version.
    #[pallet::storage]
    #[pallet::getter(fn storage_version)]
    pub(crate) type StorageVersion<T> = StorageValue<_, Version, ValueQuery>;

    #[pallet::type_value]
    pub(crate) fn PreApprovalOnEmpty() -> bool {
        false
    }

    /// Enable or disable pre-approval list for new contract registration
    #[pallet::storage]
    #[pallet::getter(fn pre_approval_is_enabled)]
    pub(crate) type PreApprovalIsEnabled<T> = StorageValue<_, bool, ValueQuery, PreApprovalOnEmpty>;

    /// List of pre-approved developers
    #[pallet::storage]
    #[pallet::getter(fn pre_approved_developers)]
    pub(crate) type PreApprovedDevelopers<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, (), ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account has bonded and staked funds on a smart contract.
        BondAndStake(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has unbonded & unstaked some funds. Unbonding process begins.
        UnbondAndUnstake(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has fully withdrawn all staked amount from an unregistered contract.
        WithdrawFromUnregistered(T::AccountId, T::SmartContract, BalanceOf<T>),
        /// Account has withdrawn unbonded funds.
        Withdrawn(T::AccountId, BalanceOf<T>),
        /// New contract added for staking.
        NewContract(T::AccountId, T::SmartContract),
        /// Contract removed from dapps staking.
        ContractRemoved(T::AccountId, T::SmartContract),
        /// New dapps staking era. Distribute era rewards to contracts.
        NewDappStakingEra(EraIndex),
        /// Reward paid to staker or developer.
        Reward(T::AccountId, T::SmartContract, EraIndex, BalanceOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Disabled
        Disabled,
        /// Upgrade is too heavy, reduce the weight parameter.
        UpgradeTooHeavy,
        /// Can not stake with zero value.
        StakingWithNoValue,
        /// Can not stake with value less than minimum staking value
        InsufficientValue,
        /// Number of stakers per contract exceeded.
        MaxNumberOfStakersExceeded,
        /// Targets must be operated contracts
        NotOperatedContract,
        /// Contract isn't staked.
        NotStakedContract,
        /// Contract isn't unregistered.
        NotUnregisteredContract,
        /// Unstaking a contract with zero value
        UnstakingWithNoValue,
        /// There are no previously unbonded funds that can be unstaked and withdrawn.
        NothingToWithdraw,
        /// The contract is already registered by other account
        AlreadyRegisteredContract,
        /// User attempts to register with address which is not contract
        ContractIsNotValid,
        /// This account was already used to register contract
        AlreadyUsedDeveloperAccount,
        /// Smart contract not owned by the account id.
        NotOwnedContract,
        /// Report issue on github if this is ever emitted
        UnknownEraReward,
        /// Report issue on github if this is ever emitted
        UnexpectedStakeInfoEra,
        /// Contract has too many unlocking chunks. Withdraw the existing chunks if possible
        /// or wait for current chunks to complete unlocking process to withdraw them.
        TooManyUnlockingChunks,
        /// Contract already claimed in this era and reward is distributed
        AlreadyClaimedInThisEra,
        /// Era parameter is out of bounds
        EraOutOfBounds,
        /// Too many active `EraStake` values for (staker, contract) pairing.
        /// Claim existing rewards to fix this problem.
        TooManyEraStakeValues,
        /// To register a contract, pre-approval is needed for this address
        RequiredContractPreApproval,
        /// Developer's account is already part of pre-approved list
        AlreadyPreApprovedDeveloper,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // As long as pallet is disabled, we shouldn't allow any storage modifications.
            // This means we might prolong an era but it's acceptable.
            // Runtime upgrade should be timed so we ensure that we complete it before
            // a new era is triggered. This code is just a safety net to ensure nothing is broken
            // if we fail to do that.
            if Self::pallet_disabled() {
                return T::DbWeight::get().reads(1);
            }

            let force_new_era = Self::force_era().eq(&Forcing::ForceNew);
            let blocks_per_era = T::BlockPerEra::get();
            let previous_era = Self::current_era();

            // Value is compared to 1 since genesis block is ignored
            if now % blocks_per_era == BlockNumberFor::<T>::from(1u32)
                || force_new_era
                || previous_era.is_zero()
            {
                let next_era = previous_era + 1;
                CurrentEra::<T>::put(next_era);

                let reward = BlockRewardAccumulator::<T>::take();
                Self::reward_balance_snapshoot(previous_era, reward);

                if force_new_era {
                    ForceEra::<T>::put(Forcing::NotForcing);
                }

                Self::deposit_event(Event::<T>::NewDappStakingEra(next_era));
            }

            T::DbWeight::get().writes(5)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(weight_limit.unwrap_or(T::BlockWeights::get().max_block / 5 * 3))]
        pub fn do_upgrade(
            origin: OriginFor<T>,
            weight_limit: Option<Weight>,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let weight_limit = weight_limit.unwrap_or(T::BlockWeights::get().max_block / 5 * 3); // e.g. 60%

            // A sanity check to prevent too heavy upgrade
            ensure!(
                weight_limit < T::BlockWeights::get().max_block / 4 * 3,
                Error::<T>::UpgradeTooHeavy
            );

            let consumed_weight = migrations::v3::stateful_migrate::<T>(weight_limit);

            Ok(Some(consumed_weight).into())
        }

        /// register contract into staking targets.
        /// contract_id should be ink! or evm contract.
        ///
        /// Any user can call this function.
        /// However, caller have to have deposit amount.
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);

            let developer = ensure_signed(origin)?;

            ensure!(
                !RegisteredDevelopers::<T>::contains_key(&developer),
                Error::<T>::AlreadyUsedDeveloperAccount,
            );
            ensure!(
                !RegisteredDapps::<T>::contains_key(&contract_id),
                Error::<T>::AlreadyRegisteredContract,
            );
            ensure!(contract_id.is_valid(), Error::<T>::ContractIsNotValid);

            if Self::pre_approval_is_enabled() {
                ensure!(
                    PreApprovedDevelopers::<T>::contains_key(&developer),
                    Error::<T>::RequiredContractPreApproval,
                );
            }

            T::Currency::reserve(&developer, T::RegisterDeposit::get())?;

            RegisteredDapps::<T>::insert(contract_id.clone(), DAppInfo::new(developer.clone()));
            RegisteredDevelopers::<T>::insert(&developer, contract_id.clone());

            Self::deposit_event(Event::<T>::NewContract(developer, contract_id));

            Ok(().into())
        }

        /// Unregister existing contract from dapps staking
        ///
        /// This must be called by the developer who registered the contract.
        ///
        /// Warning: After this action contract can not be assigned again.
        #[pallet::weight(T::WeightInfo::unregister())]
        pub fn unregister(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            ensure_root(origin)?;

            let mut dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            ensure!(
                dapp_info.state == DAppState::Registered,
                Error::<T>::NotOperatedContract
            );
            let developer = dapp_info.developer.clone();

            let current_era = Self::current_era();
            dapp_info.state = DAppState::Unregistered(current_era);
            RegisteredDapps::<T>::insert(&contract_id, dapp_info);

            T::Currency::unreserve(&developer, T::RegisterDeposit::get());

            Self::deposit_event(Event::<T>::ContractRemoved(developer, contract_id));

            Ok(().into())
        }

        /// Withdraw locked funds from a contract that was unregistered.
        /// Funds don't need to undergo the unbonding period - they are returned immediately.
        #[pallet::weight(T::WeightInfo::withdraw_from_unregistered())]
        pub fn withdraw_from_unregistered(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            let staker = ensure_signed(origin)?;

            // dApp must exist and it has to be unregistered
            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            ensure!(
                dapp_info.state != DAppState::Registered,
                Error::<T>::NotUnregisteredContract
            );

            let current_era = Self::current_era();

            // There should be some leftover staked amount
            let mut staker_info = Self::staker_info(&staker, &contract_id);
            let staked_value = staker_info.latest_staked_value();
            ensure!(staked_value > Zero::zero(), Error::<T>::NotStakedContract);
            staker_info
                .unstake(current_era, staked_value)
                .map_err(|_| Error::<T>::UnexpectedStakeInfoEra)?;
            if let DAppState::Unregistered(unregistered_era) = dapp_info.state {
                staker_info.unregistered_era_adjust(unregistered_era);
            }

            // Unlock the staked amount immediately. No unbonding period for this scenario.
            let mut ledger = Self::ledger(&staker);
            ledger.locked = ledger.locked.saturating_sub(staked_value);
            Self::update_ledger(&staker, ledger);

            Self::update_staker_info(&staker, &contract_id, staker_info);
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(staked_value);
                    x.locked = x.locked.saturating_sub(staked_value);
                }
            });

            Self::deposit_event(Event::<T>::WithdrawFromUnregistered(
                staker,
                contract_id,
                staked_value,
            ));

            Ok(().into())
        }

        /// Lock up and stake balance of the origin account.
        ///
        /// `value` must be more than the `minimum_balance` specified by `T::Currency`
        /// unless account already has bonded value equal or more than 'minimum_balance'.
        ///
        /// The dispatch origin for this call must be _Signed_ by the staker's account.
        ///
        /// Effects of staking will be felt at the beginning of the next era.
        ///
        #[pallet::weight(T::WeightInfo::bond_and_stake())]
        pub fn bond_and_stake(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            let staker = ensure_signed(origin)?;

            // Check that contract is ready for staking.
            ensure!(
                Self::is_active(&contract_id),
                Error::<T>::NotOperatedContract
            );

            // Get the staking ledger or create an entry if it doesn't exist.
            let mut ledger = Self::ledger(&staker);
            let available_balance = Self::available_staking_balance(&staker, &ledger);
            let value_to_stake = value.min(available_balance);
            ensure!(
                value_to_stake > Zero::zero(),
                Error::<T>::StakingWithNoValue
            );

            let current_era = Self::current_era();
            let mut staking_info = Self::staking_info(&contract_id, current_era);
            let mut staker_info = Self::staker_info(&staker, &contract_id);

            ensure!(
                !staker_info.latest_staked_value().is_zero()
                    || staking_info.number_of_stakers < T::MaxNumberOfStakersPerContract::get(),
                Error::<T>::MaxNumberOfStakersExceeded
            );
            if staker_info.latest_staked_value().is_zero() {
                staking_info.number_of_stakers = staking_info.number_of_stakers.saturating_add(1);
            }

            staker_info
                .stake(current_era, value_to_stake)
                .map_err(|_| Error::<T>::UnexpectedStakeInfoEra)?;
            ensure!(
                staker_info.len() <= T::MaxEraStakeValues::get(),
                Error::<T>::TooManyEraStakeValues
            );
            ensure!(
                staker_info.latest_staked_value() >= T::MinimumStakingAmount::get(),
                Error::<T>::InsufficientValue,
            );

            // Increment ledger and total staker value for contract. Overflow shouldn't be possible but the check is here just for safety.
            ledger.locked = ledger
                .locked
                .checked_add(&value_to_stake)
                .ok_or(ArithmeticError::Overflow)?;
            staking_info.total = staking_info
                .total
                .checked_add(&value_to_stake)
                .ok_or(ArithmeticError::Overflow)?;

            // Update storage
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_add(value_to_stake);
                    x.locked = x.locked.saturating_add(value_to_stake);
                }
            });

            Self::update_ledger(&staker, ledger);
            Self::update_staker_info(&staker, &contract_id, staker_info);
            ContractEraStake::<T>::insert(&contract_id, current_era, staking_info);

            Self::deposit_event(Event::<T>::BondAndStake(
                staker,
                contract_id,
                value_to_stake,
            ));
            Ok(().into())
        }

        /// Start unbonding process and unstake balance from the contract.
        ///
        /// The unstaked amount will no longer be eligible for rewards but still won't be unlocked.
        /// User needs to wait for the unbonding period to finish before being able to withdraw
        /// the funds via `withdraw_unbonded` call.
        ///
        /// In case remaining staked balance on contract is below minimum staking amount,
        /// entire stake for that contract will be unstaked.
        ///
        #[pallet::weight(T::WeightInfo::unbond_and_unstake())]
        pub fn unbond_and_unstake(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            let staker = ensure_signed(origin)?;

            ensure!(value > Zero::zero(), Error::<T>::UnstakingWithNoValue);
            ensure!(
                Self::is_active(&contract_id),
                Error::<T>::NotOperatedContract,
            );

            // Get the latest era staking points for the contract.
            let mut staker_info = Self::staker_info(&staker, &contract_id);
            let staked_value = staker_info.latest_staked_value();
            ensure!(staked_value > Zero::zero(), Error::<T>::NotStakedContract);

            let current_era = Self::current_era();
            let mut contract_stake_info = Self::staking_info(&contract_id, current_era);

            // Calculate the value which will be unstaked.
            let remaining = staked_value.saturating_sub(value);
            let value_to_unstake = if remaining < T::MinimumStakingAmount::get() {
                contract_stake_info.number_of_stakers =
                    contract_stake_info.number_of_stakers.saturating_sub(1);
                staked_value
            } else {
                value
            };
            contract_stake_info.total = contract_stake_info.total.saturating_sub(value_to_unstake);

            // Sanity check
            ensure!(
                value_to_unstake > Zero::zero(),
                Error::<T>::UnstakingWithNoValue
            );

            staker_info
                .unstake(current_era, value_to_unstake)
                .map_err(|_| Error::<T>::UnexpectedStakeInfoEra)?;
            ensure!(
                staker_info.len() <= T::MaxEraStakeValues::get(),
                Error::<T>::TooManyEraStakeValues
            );

            // Update the chunks and write them to storage
            let mut ledger = Self::ledger(&staker);
            ledger.unbonding_info.add(UnlockingChunk {
                amount: value_to_unstake,
                unlock_era: current_era + T::UnbondingPeriod::get(),
            });
            // This should be done AFTER insertion since it's possible for chunks to merge
            ensure!(
                ledger.unbonding_info.len() <= T::MaxUnlockingChunks::get(),
                Error::<T>::TooManyUnlockingChunks
            );

            Self::update_ledger(&staker, ledger);

            // Update total staked value in era.
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.staked = x.staked.saturating_sub(value_to_unstake)
                }
            });
            Self::update_staker_info(&staker, &contract_id, staker_info);
            ContractEraStake::<T>::insert(&contract_id, current_era, contract_stake_info);

            Self::deposit_event(Event::<T>::UnbondAndUnstake(
                staker,
                contract_id,
                value_to_unstake,
            ));

            Ok(().into())
        }

        /// Withdraw all funds that have completed the unbonding process.
        ///
        /// If there are unbonding chunks which will be fully unbonded in future eras,
        /// they will remain and can be withdrawn later.
        ///
        #[pallet::weight(T::WeightInfo::withdraw_unbonded())]
        pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            let staker = ensure_signed(origin)?;

            let mut ledger = Self::ledger(&staker);
            let current_era = Self::current_era();

            let (valid_chunks, future_chunks) = ledger.unbonding_info.partition(current_era);
            let withdraw_amount = valid_chunks.sum();

            ensure!(!withdraw_amount.is_zero(), Error::<T>::NothingToWithdraw);

            // Get the staking ledger and update it
            ledger.locked = ledger.locked.saturating_sub(withdraw_amount);
            ledger.unbonding_info = future_chunks;

            Self::update_ledger(&staker, ledger);
            GeneralEraInfo::<T>::mutate(&current_era, |value| {
                if let Some(x) = value {
                    x.locked = x.locked.saturating_sub(withdraw_amount)
                }
            });

            Self::deposit_event(Event::<T>::Withdrawn(staker, withdraw_amount));

            Ok(().into())
        }

        // TODO: do we need to add force methods or at least methods that allow others to claim for someone else?

        /// Claim earned staker rewards for the oldest era.
        #[pallet::weight(T::WeightInfo::claim_staker())]
        pub fn claim_staker(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            let staker = ensure_signed(origin)?;

            // Ensure we have something to claim
            let mut staker_info = Self::staker_info(&staker, &contract_id);
            let (era, staked) = staker_info.claim();
            ensure!(staked > Zero::zero(), Error::<T>::NotStakedContract);

            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;
            if let DAppState::Unregistered(unregister_era) = dapp_info.state {
                ensure!(era < unregister_era, Error::<T>::NotOperatedContract);
            }

            let current_era = Self::current_era();
            ensure!(era < current_era, Error::<T>::EraOutOfBounds);

            let staking_info = Self::staking_info(&contract_id, era);
            let reward_and_stake =
                Self::general_era_info(era).ok_or(Error::<T>::UnknownEraReward)?;

            let (_, stakers_joint_reward) =
                Self::dev_stakers_split(&staking_info, &reward_and_stake);
            let staker_reward =
                Perbill::from_rational(staked, staking_info.total) * stakers_joint_reward;

            // Withdraw reward funds from the dapps staking pot
            let reward_imbalance = T::Currency::withdraw(
                &Self::account_id(),
                staker_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;

            T::Currency::resolve_creating(&staker, reward_imbalance);
            Self::update_staker_info(&staker, &contract_id, staker_info);

            Self::deposit_event(Event::<T>::Reward(
                staker.clone(),
                contract_id.clone(),
                era,
                staker_reward,
            ));

            Ok(().into())
        }

        /// Claim earned dapp rewards for the specified era.
        #[pallet::weight(T::WeightInfo::claim_dapp())]
        pub fn claim_dapp(
            origin: OriginFor<T>,
            contract_id: T::SmartContract,
            #[pallet::compact] era: EraIndex,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            let _ = ensure_signed(origin)?;

            let dapp_info =
                RegisteredDapps::<T>::get(&contract_id).ok_or(Error::<T>::NotOperatedContract)?;

            let current_era = Self::current_era();
            if let DAppState::Unregistered(unregister_era) = dapp_info.state {
                ensure!(era < unregister_era, Error::<T>::NotOperatedContract);
            }
            ensure!(era < current_era, Error::<T>::EraOutOfBounds);

            let mut contract_stake_info = Self::staking_info(&contract_id, era);
            ensure!(
                !contract_stake_info.contract_reward_claimed,
                Error::<T>::AlreadyClaimedInThisEra,
            );
            ensure!(
                contract_stake_info.total > Zero::zero(),
                Error::<T>::NotStakedContract,
            );

            let reward_and_stake =
                Self::general_era_info(era).ok_or(Error::<T>::UnknownEraReward)?;

            // Calculate the contract reward for this era.
            let (dapp_reward, _) = Self::dev_stakers_split(&contract_stake_info, &reward_and_stake);

            // Withdraw reward funds from the dapps staking
            let reward_imbalance = T::Currency::withdraw(
                &Self::account_id(),
                dapp_reward,
                WithdrawReasons::TRANSFER,
                ExistenceRequirement::AllowDeath,
            )?;

            T::Currency::resolve_creating(&dapp_info.developer, reward_imbalance);
            Self::deposit_event(Event::<T>::Reward(
                dapp_info.developer.clone(),
                contract_id.clone(),
                era,
                dapp_reward,
            ));

            // updated counter for total rewards paid to the contract
            contract_stake_info.contract_reward_claimed = true;
            ContractEraStake::<T>::insert(&contract_id, era, contract_stake_info);

            Ok(().into())
        }

        /// Force there to be a new era at the end of the next block. After this, it will be
        /// reset to normal (non-forced) behaviour.
        ///
        /// The dispatch origin must be Root.
        ///
        ///
        /// # <weight>
        /// - No arguments.
        /// - Weight: O(1)
        /// - Write ForceEra
        /// # </weight>
        #[pallet::weight(T::WeightInfo::force_new_era())]
        pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            ensure_root(origin)?;
            ForceEra::<T>::put(Forcing::ForceNew);
            Ok(())
        }

        /// add contract address to the pre-approved list.
        /// contract_id should be ink! or evm contract.
        ///
        /// Sudo call is required
        #[pallet::weight(T::WeightInfo::developer_pre_approval())]
        pub fn developer_pre_approval(
            origin: OriginFor<T>,
            developer: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            ensure_root(origin)?;

            ensure!(
                !PreApprovedDevelopers::<T>::contains_key(&developer),
                Error::<T>::AlreadyPreApprovedDeveloper
            );
            PreApprovedDevelopers::<T>::insert(developer, ());

            Ok(().into())
        }

        /// Enable or disable adding new contracts to the pre-approved list
        ///
        /// Sudo call is required
        #[pallet::weight(T::WeightInfo::enable_developer_pre_approval())]
        pub fn enable_developer_pre_approval(
            origin: OriginFor<T>,
            enabled: bool,
        ) -> DispatchResultWithPostInfo {
            ensure!(!Self::pallet_disabled(), Error::<T>::Disabled);
            ensure_root(origin)?;
            PreApprovalIsEnabled::<T>::put(enabled);
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get AccountId assigned to the pallet.
        fn account_id() -> T::AccountId {
            T::PalletId::get().into_account()
        }

        /// Update the ledger for a staker. This will also update the stash lock.
        /// This lock will lock the entire funds except paying for further transactions.
        fn update_ledger(staker: &T::AccountId, ledger: AccountLedger<BalanceOf<T>>) {
            if ledger.is_empty() {
                Ledger::<T>::remove(&staker);
                T::Currency::remove_lock(STAKING_ID, &staker);
            } else {
                T::Currency::set_lock(STAKING_ID, &staker, ledger.locked, WithdrawReasons::all());
                Ledger::<T>::insert(staker, ledger);
            }
        }

        /// Update the staker info for the `(staker, contract_id)` pairing.
        /// If staker_info is empty, remove it from the DB. Otherwise, store it.
        fn update_staker_info(
            staker: &T::AccountId,
            contract_id: &T::SmartContract,
            staker_info: StakerInfo<BalanceOf<T>>,
        ) {
            if staker_info.is_empty() {
                StakersInfo::<T>::remove(staker, contract_id)
            } else {
                StakersInfo::<T>::insert(staker, contract_id, staker_info)
            }
        }

        /// The block rewards are accumulated on the pallets's account during an era.
        /// This function takes a snapshot of the pallet's balance accrued during current era
        /// and stores it for future distribution
        ///
        /// This is called just at the beginning of an era.
        fn reward_balance_snapshoot(era: EraIndex, rewards: RewardInfo<BalanceOf<T>>) {
            // Get the reward and stake information for previous era
            let mut era_info = Self::general_era_info(era).unwrap_or_default();

            // Prepare info for the next era
            GeneralEraInfo::<T>::insert(
                era + 1,
                EraInfo {
                    rewards: Default::default(),
                    staked: era_info.staked.clone(),
                    locked: era_info.locked.clone(),
                },
            );

            // Set the reward for the previous era.
            era_info.rewards = rewards;
            GeneralEraInfo::<T>::insert(era, era_info);
        }

        /// This helper returns `EraStakingPoints` for given era if possible or latest stored data
        /// or finally default value if storage have no data for it.
        pub fn staking_info(
            contract_id: &T::SmartContract,
            era: EraIndex,
        ) -> EraStakingPoints<BalanceOf<T>> {
            // By checking current and previus era, we will avoid key prefix iteration in most of the cases.
            // It is safe to assume that contract era stake will change each era - for this to occur, either dapp rewards
            // need to be claimed or active stake amount needs to change (highly likely when automatic reward restaking introduced).
            if ContractEraStake::<T>::contains_key(contract_id, era) {
                ContractEraStake::<T>::get(contract_id, era).unwrap()
            } else if ContractEraStake::<T>::contains_key(contract_id, era.saturating_sub(1)) {
                let mut staking_points =
                    ContractEraStake::<T>::get(contract_id, era.saturating_sub(1)).unwrap();
                staking_points.contract_reward_claimed = false;
                staking_points
            } else {
                let avail_era = ContractEraStake::<T>::iter_key_prefix(&contract_id)
                    .filter(|x| *x <= era)
                    .max()
                    .unwrap_or(Zero::zero());

                let mut staking_points =
                    ContractEraStake::<T>::get(contract_id, avail_era).unwrap_or_default();
                // Needs to be reset since otherwise it might seem as if rewards were already claimed for this era.
                staking_points.contract_reward_claimed = false;
                staking_points
            }
        }

        /// Returns available staking balance for the potential staker
        fn available_staking_balance(
            staker: &T::AccountId,
            ledger: &AccountLedger<BalanceOf<T>>,
        ) -> BalanceOf<T> {
            // Ensure that staker has enough balance to bond & stake.
            let free_balance =
                T::Currency::free_balance(&staker).saturating_sub(T::MinimumRemainingAmount::get());

            // Remove already locked funds from the free balance
            free_balance.saturating_sub(ledger.locked)
        }

        /// `true` if contract is active, `false` if it has been unregistered
        fn is_active(contract_id: &T::SmartContract) -> bool {
            RegisteredDapps::<T>::get(contract_id)
                .map_or(false, |dapp_info| dapp_info.state == DAppState::Registered)
        }

        /// Calculate reward split between developer and stakers.
        ///
        /// Returns (developer reward, joint stakers reward)
        pub(crate) fn dev_stakers_split(
            contract_info: &EraStakingPoints<BalanceOf<T>>,
            era_info: &EraInfo<BalanceOf<T>>,
        ) -> (BalanceOf<T>, BalanceOf<T>) {
            let contract_stake_portion =
                Perbill::from_rational(contract_info.total, era_info.staked);

            let developer_reward_part = contract_stake_portion * era_info.rewards.dapps;
            let stakers_joint_reward = contract_stake_portion * era_info.rewards.stakers;

            (developer_reward_part, stakers_joint_reward)
        }
    }
}
