pragma solidity ^0.8.0;

/**
 * @title XCM interface.
 */
interface XCM {
    /**
     * @dev Withdraw assets using PalletXCM call.
     * @param asset_id - list of XC20 asset addresses
     * @param asset_amount - list of transfer amounts (must match with asset addresses above)
     * @param recipient_account_id - AccountId of destination account
     * @param is_relay - set `true` for using relay chain as reserve
     * @param parachain_id - set parachain id of reserve parachain (when is_relay set to false)
     * @param fee_index - index of asset_id item that should be used as a XCM fee
     * @return A boolean confirming whether the XCM message sent.
     *
     * How method check that assets list is valid:
     * - all assets resolved to multi-location (on runtime level)
     * - all assets has corresponded amount (lenght of assets list matched to amount list)
     */
    function assets_withdraw(
        address[] calldata asset_id,
        uint256[] calldata asset_amount,
        bytes32   recipient_account_id,
        bool      is_relay,
        uint256   parachain_id,
        uint256   fee_index
    ) external returns (bool);
}
