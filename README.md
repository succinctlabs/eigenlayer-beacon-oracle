# Eigenlayer Beacon Oracle

This repository contains the smart contract for the Eigenlayer Beacon Oracle. It uses the beacon roots precompile added in [EIP-4788](https://eips.ethereum.org/EIPS/eip-4788) to find the block root for a given timestamp.

## Deployments

Currently, the Eigenlayer Beacon Oracle is deployed on the following chains: 

- [Ethereum](https://etherscan.io/address/0x343907185b71adf0eba9567538314396aa985442)
- [Holesky](https://holesky.etherscan.io/address/0x4C116BB629bff7A8373c2378bBd919f8349B8f25#events)

Example transaction:
- Transaction: https://etherscan.io/tx/0x9cd868f8a939a9a35fcb08a5f711c1477ad357b32c196be807f990a7d7a14d57#eventlog
- Reference Slot: https://goerli.beaconcha.in/slot/8791805

## Contracts

To deploy the contract on a chain, run the following command:

```shell
$ cd contracts
$ forge script script/DeployEigenLayerBeaconOracle.s.sol:DeployEigenLayerBeaconOracle --rpc-url <RPC_URL> --private-key <PRIVATE_KEY> --verifier etherscan --etherscan-api-key <ETHERSCAN_API_KEY> --verify --broadcast
```

## Operator Script

Make sure to set the environment variables in `.env` before running the operator. If you're planning to self-relay (not use the Succinct relayer), set `RELAYER_PRIVATE_KEY` in `.env`.

To run the script which periodically updates the oracle, run the following command:

```shell
$ RUST_LOG=info cargo run --bin operator --release
```

## Cost
To get the Ethereum block corresponding to a date, do the following:

To compute the cost of requesting beacon block roots over the past month, run the following command:

```shell
cargo run --bin cost -- --rpc-url <RPC_URL> --relayer-address <RELAYER_ADDRESS> --contract-address <CONTRACT_ADDRESS> --start-timestamp <START_TIMESTAMP> --end-timestamp <END_TIMESTAMP>
```

Ex. To get the total costs from relayer on Mainnet, run the following:
```shell
cargo run --bin cost -- --rpc-url <MAINNET_RPC_URL> --relayer-address 0x44eB418A966ff47f5AF6f48AEa6Afde0bf193a8d --contract-address 0x343907185b71adf0eba9567538314396aa985442 --start-timestamp 2024-03-01T00:00:00+00:00 --end-timestamp 2024-04-01T00:00:00+00:00
```
