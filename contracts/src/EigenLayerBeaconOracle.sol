// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IBeaconChainOracle} from "./IBeaconChainOracle.sol";

/// @title EigenLayerBeaconOracle
/// @author Succinct Labs
contract EigenLayerBeaconOracle is IBeaconChainOracle {
    /// @notice The address of the beacon roots precompile.
    /// @dev https://eips.ethereum.org/EIPS/eip-4788
    address internal constant BEACON_ROOTS =
        0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02;

    /// @notice The maximum number of slots to search through to handle skipped slots.
    /// @dev This is 1 day worth of slots.
    uint256 internal constant MAX_SLOT_ATTEMPTS = 7200;

    /// @notice The size of the beacon block root ring buffer.
    uint256 internal constant BUFFER_LENGTH = 8191;

    /// @notice The timestamp to block root mapping.
    mapping(uint256 => bytes32) public timestampToBlockRoot;

    /// @notice The genesis block timestamp.
    uint256 public immutable GENESIS_BLOCK_TIMESTAMP;

    /// @notice The event emitted when a new block is added to the oracle.
    event EigenLayerBeaconOracleUpdate(
        uint256 slot,
        uint256 timestamp,
        bytes32 blockRoot
    );

    /// @notice Block timestamp does not correspond to a valid slot.
    error InvalidBlockTimestamp();

    /// @notice Timestamp out of range.
    error TimestampOutOfRange();

    constructor(uint256 _genesisBlockTimestamp) {
        // Set the genesis block timestamp.
        GENESIS_BLOCK_TIMESTAMP = _genesisBlockTimestamp;
    }

    function addTimestamp(uint256 _targetTimestamp) external {
        // If the targetTimestamp is not guaranteed to be within the beacon block root ring buffer, revert.
        if ((block.timestamp - _targetTimestamp) >= (BUFFER_LENGTH * 12)) {
            revert TimestampOutOfRange();
        }

        // If _targetTimestamp corresponds to slot n, then the block root for slot n - 1 is returned.
        (bool success, ) = BEACON_ROOTS.staticcall(
            abi.encode(_targetTimestamp)
        );

        if (!success) {
            revert InvalidBlockTimestamp();
        }

        uint256 slot = (_targetTimestamp - GENESIS_BLOCK_TIMESTAMP) / 12;

        // Find the block root for the target timestamp.
        bytes32 blockRoot = findBlockRoot(uint64(slot));

        // Add the block root to the mapping.
        timestampToBlockRoot[_targetTimestamp] = blockRoot;

        // Emit the event.
        emit EigenLayerBeaconOracleUpdate(slot, _targetTimestamp, blockRoot);
    }

    /// @notice Attempts to find the block root for the given slot.
    /// @param _targetSlot The slot to get the block root for.
    /// @return blockRoot The beacon block root of the given slot.
    /// @dev BEACON_ROOTS returns a block root for a given parent block's timestamp. To get the block root for slot
    ///      N, you use the timestamp of slot N+1. If N+1 is not avaliable, you use the timestamp of slot N+2, and
    //       so on.
    function findBlockRoot(
        uint64 _targetSlot
    ) public view returns (bytes32 blockRoot) {
        uint256 currBlockTimestamp = GENESIS_BLOCK_TIMESTAMP +
            ((_targetSlot + 1) * 12);

        while (currBlockTimestamp <= block.timestamp) {
            (bool success, bytes memory result) = BEACON_ROOTS.staticcall(
                abi.encode(currBlockTimestamp)
            );
            if (success && result.length > 0) {
                return abi.decode(result, (bytes32));
            }

            unchecked {
                currBlockTimestamp += 12;
            }
        }

        revert("No available slot found");
    }
}
