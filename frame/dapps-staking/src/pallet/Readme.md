# Reward Beneficiary Delegation

## New functions

In mod.rs, three functions have been added : 

- pub fn set_delegation(origin: OriginFor<T>, contract_id: T::SmartContract, delegated_account : T::AccountId,) : set the delegation of origin for a contract id to delegated_account.
- pub fn remove_delegation(origin: OriginFor<T>,contract_id: T::SmartContract,) : remove the delegation of origin for a contract id.
- pub fn set_delegate_third_account(origin: OriginFor<T>, contract_id: T::SmartContract, active_third_account: bool,) : for a staker and a contract id, if active_third_account is true, we delegate
the rewards to the delegate's delegated_account.

    For those functions, new weight function have beed added based on read/write.

And one modified : 
- pub fn claim_staker (origin: OriginFor<T>, contract_id: T::SmartContract,) :  we update the reward account if there is delegation.

## Details

In order to implement those functions we update testing_utils : 

- assert_claim_staker   -> take a new argument (Option<AccountId>),
                        -> take the free balance before and after the claim of the delegated account,
- assert_restake_reward -> if there is a delegated account, we check if the free balances are correct, 
                        -> orelse, we check the balance of the staker.

## Tests

- Modification of assert_claim_staker to add None of pre-existing tests.

Tests created : 
- set_delegation : we assert the claim when we create a delegation,
- remove_delegation : we remove the delegation and assert the claim,
- remove_delegation_then_set_new_delegation : set a second delegation,
- delegate_third_account : check that we can delegate to delegate's delegated account,
- delegate_third_account_not_activated : check we can't delegate if it is not actived,
- delegate_back_to_himself : delegate back to the staker,
- two_delegate_for_two_contract_delegation : delegation of two different contract for two different delegated account.

## Ameliorations

To get more precisions of the weight, we could have made a benchmark.
