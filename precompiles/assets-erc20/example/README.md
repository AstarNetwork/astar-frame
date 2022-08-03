# How to Convert Existing ERC20 Token Into XC20 Asset
Prerequisite: you already have a live project based on ERC20 token and you want your ERC20 token to become cross-chain-ready

## Developer actions
### Create XC20 asset using Polkadot.js
Follow the documentation on how to [Create XC20 Assets](https://docs.astar.network/xcm/building-with-xcm/create-xc20-assets)

### Deploy Erc2XC smart contract
To deploy Erc2XC you need 2 input parameters
- ERC20 token address (H160)
- XC20 asset address (H160)
    - Follow instructions to [Generate Mintable XC20 Precompile Address](https://docs.astar.network/xcm/building-with-xcm/create-xc20-assets#generate-mintable-xc20-precompile-address)
### Transfer token ownership to the Erc2XC smart contract
Now that you have Erc2XC contract address users can start with the token convertor. 

## User action
To convert ERC20 token into XC20 assets, users will need to do two actions.
- User Approves Erc2XC smart contract to transfer ERC20 tokens from user's balance to the Erc2XC contract
- User calls Erc2XC `mintXcLockErc(amount)`

This will result into following
- User's balance for ERC20 is decreased for `amount`
- Erc2XC locks additional `amount` of ERC20 token
- User's balance for XC20 is increased for `amount`
## Result