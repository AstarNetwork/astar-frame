// SPDX-License-Identifier: MIT

pragma solidity ^0.8.7;

// DISCLAIMER: This is just an example how to convert existing ERC20 token to become XC20.
// Do your own research on Wrapping token before deploying to production

import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Wrapper.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/draft-ERC20Permit.sol";
import "./Burrito.sol";

interface IERC20Plus is IERC20 {
    function mint(address beneficiary, uint256 amount) external returns (bool);
    function burn(address who, uint256 amount) external returns (bool);
    function decimals() external view returns (uint8);
}

contract XcBurrito is ERC20Wrapper, ERC20Permit{
    IERC20Plus public xcBurrito;

    constructor(IERC20 _burrito, IERC20Plus _xcBurrito)
        ERC20("Wrapped Burrito", "xcBUR")
        ERC20Permit("Wrapped Burrito")
        ERC20Wrapper(_burrito)
    {
        xcBurrito = _xcBurrito;
    }

	function decimals() public view override(ERC20Wrapper, ERC20) returns (uint8)
    {
        return IERC20Plus(xcBurrito).decimals();
	}

    function _mint(address _to, uint256 _amount)
        internal
        override(ERC20)
    {
        // add here your pre-mint hooks hooks if needed

        require(
            IERC20Plus(xcBurrito).mint(_to, _amount), "Minting xc token failed"
        );

        // add here your post-mint hooks hooks if needed
    }

    function _burn(address _account, uint256 _amount)
        internal
        override(ERC20)
    {
        require(
            IERC20Plus(xcBurrito).burn(_account, _amount), "Burning xc token failed"
        );
    }

    receive() external payable{}
}
