// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

contract EigenLayerBeaconOracle {
    
    /// @notice The address of the beacon roots precompile.
    /// @dev https://eips.ethereum.org/EIPS/eip-4788
    address internal constant BEACON_ROOTS = 0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02;

    /// @notice The maximum number of slots to search through.
    /// @dev This is 1 day worth of slots.
    uint256 internal constant MAX_SLOT_ATTEMPTS = 7200;

    /// @notice The block number to state root mapping.
    mapping(uint256 => bytes32) public timestampToBlockRoot;

    /// @notice The genesis block timestamp.
    uint256 public immutable GENESIS_BLOCK_TIMESTAMP;

    event EigenLayerBeaconOracleUpdate(uint256 slot, uint256 timestamp, bytes32 blockRoot);

    /// @notice Block timestamp does not correspond to a valid slot.
    error InvalidBlockTimestamp();

    constructor(
        uint256 _genesisBlockTimestamp
    ) {
        // Set the genesis block timestamp.
        GENESIS_BLOCK_TIMESTAMP = _genesisBlockTimestamp;
    }

    function addTimestamp(uint256 _targetTimestamp) external {
        // If _targetTimestamp corresponds to slot n, then the block root for slot n - 1 is returned.
        (bool success, ) = BEACON_ROOTS.staticcall(abi.encode(_targetTimestamp));

        if (!success) {
            revert InvalidBlockTimestamp();
        }

        uint256 slot = (_targetTimestamp - GENESIS_BLOCK_TIMESTAMP) / 12;

        // Find the block root for the target timestamp.
        bytes32 blockRoot = findBlockRoot(uint64(slot));

        timestampToBlockRoot[_targetTimestamp] = blockRoot;

        // Emit the event.
        emit EigenLayerBeaconOracleUpdate(slot, _targetTimestamp, blockRoot); 
    }

    /// @notice findBlockRoot takes a valid slot _targetSlot and returns the block root corresponding to _targetSlot.
    /// @param _targetSlot The slot to start searching from.
    /// @return blockRoot The beacon root of the first available slot found.
    /// @dev BEACON_ROOTS returns a block root for a given parent block's timestamp, e.g. to get the block root for slot
    ///      1000, you use the timestamp of block 1001.
    function findBlockRoot(uint64 _targetSlot)
        public
        view
        returns (bytes32 blockRoot)
    {
        uint64 currSlot = _targetSlot + 1;
        bool success;
        bytes memory result;

        for (uint64 i = 0; i < MAX_SLOT_ATTEMPTS; i++) {
            uint256 currTimestamp = GENESIS_BLOCK_TIMESTAMP + (currSlot * 12);
            (success, result) = BEACON_ROOTS.staticcall(abi.encode(currTimestamp));
            if (success && result.length > 0) {
                return (abi.decode(result, (bytes32)));
            }

            currSlot++;
        }

        revert("No available slot found");
    }
}
