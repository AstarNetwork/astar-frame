# How to Convert Existing ERC20 Token Into XC20 Asset
This an example how you can enhance already deployed ERC20 token to cross-chain-ready XC20 asset.

## Wrapped token in General
A wrapped token is a token whose value is tied to an underlying cryptocurrency. An amount of the original token is locked in a digital vault, and in return this allows an equivalent amount of wrapped tokens to be minted.

* To extend the functionalities of an existing token in conjunction with other ERC20 modules.
* Allow a native cryptocurrency to behave like an ERC20, e.g. Wrapped ether (WETH).
* Allow the use of currencies outside its native blockchain, e.g. Wrapped bitcoin (WBTC).

In our example we will do a mix of 1st and 3rd use case. We are wrapping existing ERC20 token to become XC20.

##
Underlaying token will be existing ERC20, let's name it BURRITO. We want to wrap this BURRITO token and transport it to another chain. Using standard [ERC20Wrapper](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/token/ERC20/extensions/ERC20Wrapper.sol) token spec from OpenZeppelin will not be enough. Therefore we will override some of the  ERC20Wrapper functions to use XC20+ functions.
Let's call this new wrapped token xcBURITTO.
xcBURITTO takes a parameter for the address of the underlaying token (BURITTO) as a constructor parameter. And we’ll setvalues for all the other required parameters, notice that we have to include ERC20Permit constructor call because xcBURITTO is now a parent for BURITTO.
```
constructor(IERC20 buritto)
   ERC20("Wrapped Buritto", "xcBUR")
   ERC20Permit("Wrapped Buritto")
   ERC20Wrapper(buritto)
{}
```
Since we can't use ERC20Wrapper out of box we will override it to use XC20+ interface
```
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Wrapper.sol";

contract XcBuritto is Xc20Plus, ERC20Wrapper, BURITTO{
    constructor(IERC20 buritto)
    Xc20Plus("Wrapped Buritto", "xcBUR")
    ERC20Permit("Wrapped Buritto")
    ERC20Wrapper(buritto)
    {}

    function _afterTokenTransfer(address from, address to, uint256 amount)
        internal
        override(ERC20, ERC20Votes)
    {
        super._afterTokenTransfer(from, to, amount);
    }

    function _mint(address to, uint256 amount)
        internal
        override(ERC20, ERC20Votes)
    {
        super._mint(to, amount);
    }

    function _burn(address account, uint256 amount)
        internal
        override(ERC20, ERC20Votes)
    {
        super._burn(account, amount);
    }
}
```
## Procedures
### 1. Create XC20 asset using Polkadot.js
Follow the documentation on how to [Create XC20 Assets](https://docs.astar.network/xcm/building-with-xcm/create-xc20-assets)

### 2. Deploy Erc2XC smart contract
To deploy Erc2XC you need 2 input parameters
- ERC20 token address (H160)
- XC20 asset address (H160)
    - Follow instructions to [Generate Mintable XC20 Precompile Address](https://docs.astar.network/xcm/building-with-xcm/create-xc20-assets#generate-mintable-xc20-precompile-address)
### 3. Transfer token ownership to the Erc2XC smart contract
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