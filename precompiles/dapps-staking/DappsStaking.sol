// SPDX-License-Identifier: BSD-3-Clause

pragma solidity >=0.7.0;

/// Interface to the precompiled contract on Shibuya/Shiden/Astar
/// Predeployed at the address 0x0000000000000000000000000000000000005001
interface DappsStaking {

    // Storage getters

    /// @notice Read current era.
    /// @return era, The current era
    function read_current_era() external view returns (uint256);

    /// @notice Read unbonding period constant.
    /// @return period, The unbonding period in eras
    function read_unbonding_period() external view returns (uint256);

    /// @notice Read Total network reward for the given era
    /// @return reward, Total network reward for the given era
    function read_era_reward(uint32 era) external view returns (uint128);

    /// @notice Read Total staked amount for the given era
    /// @return staked, Total staked amount for the given era
    function read_era_staked(uint32 era) external view returns (uint128);

    /// @notice Read Staked amount for the staker
    /// @param staker in form of 20 or 32 hex bytes
    /// @return amount, Staked amount by the staker
    function read_staked_amount(bytes calldata staker) external view returns (uint128);

    /// @notice Read Staked amount on a given contract for the staker
    /// @param contract_id contract evm address
    /// @param staker in form of 20 or 32 hex bytes
    /// @return amount, Staked amount by the staker
    function read_staked_amount_on_contract(address contract_id, bytes calldata staker) external view returns (uint128);

    /// @notice Read the staked amount from the era when the amount was last staked/unstaked
    /// @return total, The most recent total staked amount on contract
    function read_contract_stake(address contract_id) external view returns (uint128);


    // Extrinsic calls

    /// @notice Register provided contract.
    function register(address) external;

    /// @notice Stake provided amount on the contract.
    function bond_and_stake(address, uint128) external;

    /// @notice Start unbonding process and unstake balance from the contract.
    function unbond_and_unstake(address, uint128) external;

    /// @notice Withdraw all funds that have completed the unbonding process.
    function withdraw_unbonded() external;

    /// @notice Claim one era of unclaimed staker rewards for the specifeid contract.
    ///         Staker account is derived from the caller address.
    function claim_staker(address) external;

    /// @notice Claim one era of unclaimed dapp rewards for the specified contract and era.
    function claim_dapp(address, uint128) external;

    /// Instruction how to handle reward payout for staker.
    /// `FreeBalance` - Reward will be paid out to the staker (free balance).
    /// `StakeBalance` - Reward will be paid out to the staker and is immediately restaked (locked balance)
    enum RewardDestination {FreeBalance, StakeBalance}

    /// @notice Set reward destination for staker rewards
    /// @param reward_destination instruction on how the reward payout should be handled
    function set_reward_destination(RewardDestination reward_destination) external;
    
    /// @notice Withdraw staked funds from an unregistered contract.
    /// @param smart_contract smart contract address
    function withdraw_from_unregistered(address smart_contract) external;

    /// @notice Transfer part or entire nomination from origin smart contract to target smart contract
    /// @param origin_smart_contract origin smart contract address
    /// @param amount amount to transfer from origin to target
    /// @param target_smart_contract target smart contract address
    function nomination_transfer(address origin_smart_contract, uint128 amount, address target_smart_contract) external;
}
