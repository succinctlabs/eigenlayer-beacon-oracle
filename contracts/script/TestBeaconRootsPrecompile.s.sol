// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import "../src/EigenLayerBeaconOracle.sol";

contract TestBeaconRootsPrecompile is Script {
    function run() public returns (bytes32) {
        vm.createSelectFork("mainnet");

        console.log(block.number);

        uint64 _slot = 8822286;
        EigenLayerBeaconOracle oracle = EigenLayerBeaconOracle(
            0x343907185b71aDF0eBa9567538314396aa985442
        );
        bytes32 blockRoot = oracle.findBlockRoot(_slot);
        return blockRoot;
    }
}
