pragma solidity ^0.8.0;

/**
 * @title XCM interface.
 */
interface XCM {
    /**
     * @dev Withdraw assets using PalletXCM call.
     * @return A boolean confirming whether the XCM message sent.
     */
    function assets_withdraw(
        address[] asset_id,
        uint256[] asset_amount,
        bytes32   recipient_account_id,
        bool      is_relay,
        uint256   parachain_id,
        uint256   fee_index,
    ) external view returns (bool);
}
