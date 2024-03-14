# Eigenlayer Beacon Oracle

This repository contains the smart contract for the Eigenlayer Beacon Oracle. It uses the beacon roots precompile added in [EIP-4788](https://eips.ethereum.org/EIPS/eip-4788) to find the block root for a given timestamp.

## Deployments

Currently, the Eigenlayer Beacon Oracle is deployed on the following chains: 

- [Goerli](https://goerli.etherscan.io/address/0x0B3b61251e8373bFb183C8C5aA1ED5Ac45c19400#events)
- [Holesky](https://holesky.etherscan.io/address/0x4C116BB629bff7A8373c2378bBd919f8349B8f25#events)

Example transaction:
- Transaction: https://goerli.etherscan.io/tx/0xe1189b57c2a12e3f224d169640c8f9cd8bd0d757d023b049bdfe790ab1cee08c#eventlog
- Reference Slot: https://goerli.beaconcha.in/slot/7770895

## Contracts

To deploy the contract on a chain, run the following command:

```shell
$ cd contracts
$ forge script script/DeployEigenLayerBeaconOracle.s.sol:DeployEigenLayerBeaconOracle --rpc-url <RPC_URL> --private-key <PRIVATE_KEY> --verifier etherscan --etherscan-api-key <ETHERSCAN_API_KEY> --verify --broadcast
```

## Operator Script

To run the script which periodically updates the oracle, run the following command:

```shell
$ cargo run --release
```

Make sure to set the enviroment variables in `.env` before running the script.

## Cost
To compute the cost of requesting beacon block roots over the past month, run the following command:

```shell
cargo run --bin cost <START_BLOCK> <END_BLOCK> -- --nocapture
