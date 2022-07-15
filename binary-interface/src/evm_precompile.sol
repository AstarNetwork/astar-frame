pragma solidity ^0.8.0;

interface XVM {
    
    /**
    * @dev Call a function in another virtual machine
    * @param vm_index - index of the virtual machine to which the call is delegated
    * @param contract_address - address of the contract being called
    * @param func - string representation of the function being called
    * @param input_args - input arguments encoded in XVM intermediary format where each index represents one encoded argument
    */
    function call(uint8 vm_index, bytes calldata contract_address, string calldata func, bytes[] calldata input_args) external returns (bool);

    /**
    * @dev Encodes bool value into XVM intermediary format
    * @param arg - bool to encode
    */
    function encode_bool(bool arg) pure external returns (bytes memory);

    /**
    * @dev Encodes address value into XVM intermediary format
    * @param arg - address to encode
    */
    function encode_address(address arg) pure external returns (bytes memory);

    /**
    * @dev Encodes uint32 value into XVM intermediary format
    * @param arg - uint32 to encode
    */
    function encode_uint32(uint32 arg) pure external returns (bytes memory);
}