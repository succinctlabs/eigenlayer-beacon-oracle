// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import "../src/EigenLayerBeaconOracle.sol";

contract TestBeaconRootsPrecompile is Script {
    function run() public returns (bytes32) {
        vm.createSelectFork("mainnet");

        address BEACON_ROOTS = 0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02;

        console.log(block.number);

        // Sourced from: https://beaconcha.in/slot/8822288 (missing slot).
        uint64 missingSlotTimestamp = 1712691479;

        (bool success, bytes memory result) = BEACON_ROOTS.staticcall(
            abi.encode(missingSlotTimestamp)
        );
        console.logBool(success);
        if (success && result.length > 0) {
            return abi.decode(result, (bytes32));
        }

        return bytes32(0);
    }
}
