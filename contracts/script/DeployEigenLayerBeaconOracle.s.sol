// SPDX-License-Identifier: MIT
import "forge-std/Script.sol";
import "../src/EigenLayerBeaconOracle.sol";

contract DeployEigenLayerBeaconOracle is Script {
    function run() public returns (address) {
        vm.broadcast();

        uint256 GENESIS_BLOCK_TIMESTAMP;
        if (block.chainid == 1) {
            GENESIS_BLOCK_TIMESTAMP = 1606824023;
        } else if (block.chainid == 5) {
            GENESIS_BLOCK_TIMESTAMP = 1616508000;
        } else if (block.chainid == 11155111) {
            GENESIS_BLOCK_TIMESTAMP = 1655733600;
        } else if (block.chainid == 17000) {
            GENESIS_BLOCK_TIMESTAMP = 1695902400;
        } else {
            revert("Unsupported chainId.");
        }

        EigenLayerBeaconOracle oracle = new EigenLayerBeaconOracle(GENESIS_BLOCK_TIMESTAMP);
        return address(oracle);
    }
}