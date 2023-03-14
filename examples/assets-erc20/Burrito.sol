// SPDX-License-Identifier: MIT

// Deployed on Shiden 0xDCD47F46bd061cb0ee1cCDef77FEF8e5d80c80a3
pragma solidity ^0.8.7;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract BurritoToken is ERC20 {
    constructor() ERC20("Burrito Token", "BUR") {
        _mint(msg.sender, 100000 * 10 ** decimals());
    }
}
