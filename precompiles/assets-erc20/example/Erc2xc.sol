// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

// Use this example if you have existing ERC20 token and you want to make it to be XC20

interface ERC20 {
    function balanceOf(address owner) external view returns (uint);
    function transferFrom(address from, address to, uint value) external returns (bool); 
}

interface IERC20Plus is ERC20 {
    function mint(address beneficiary, uint256 amount) external returns (bool);
    function burn(address who, uint256 amount) external returns (bool);
}

contract Erc2xc {
    ERC20 public ercToken;
    IERC20Plus public xcToken;
    uint256 public totalXCSupply;

    uint256 public thisContract_xcBalance;
    uint256 public thisContract_ercBalance;


    constructor(ERC20 _ercToken, IERC20Plus _xcToken) {
        ercToken = _ercToken;
        xcToken = _xcToken;
        thisContract_xcBalance = IERC20Plus(xcToken).balanceOf(address(this));
        thisContract_ercBalance = ERC20(ercToken).balanceOf(address(this));
    }

    function mintXcLockErc(uint256 _amount) public {
        require(ERC20(ercToken).balanceOf(msg.sender) >= _amount, "Low token balance");
        ERC20(ercToken).transferFrom(msg.sender, address(this), _amount);
        require(
            IERC20Plus(xcToken).mint(msg.sender, _amount), "Minting xc token failed"
        );
    }

    function getBalanceOfErcOnThisContract() public view returns (uint256) {
        return ERC20(ercToken).balanceOf(address(this));
    }
}