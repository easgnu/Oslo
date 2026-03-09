# Oslo Network

Welcome to the **Oslo Network**, a high-performance Substrate-based blockchain node specifically tailored for robust consensus and decentralized application scaling.

## Overview

Oslo provides a secure and scalable environment for deploying and interacting with smart contracts and blockchain primitives. Built on the powerful [Polkadot SDK](https://github.com/paritytech/polkadot-sdk), Oslo integrates both standard Substrate pallets and the Frontier EVM compatibility layer, ensuring developers can use familiar tooling like Truffle, Hardhat, and standard Web3 libraries out of the box.

## Getting Started

### Prerequisites

Please ensure you have completed the standard Substrate environment setup. Refer to the [official Substrate documentation](https://docs.substrate.io/install/) for installing Rust and the required dependencies.

### Building the Node

Use Cargo to compile the node. This process might take a while depending on your hardware:

```sh
cargo build --release
```

*Tip: Add `-j <threads>` to speed up compilation if you have a multi-core CPU.*

### Running the Node

To run a temporary, single-node development chain (where state is discarded upon exit):

```sh
./target/release/oslo-network --dev
```

To persist state across runs, you can specify a base path, such as the local `node-storage` directory:

```sh
./target/release/oslo-network --dev --base-path ./node-storage
```

## Architecture

- **Node:** The core executable that handles networking (libp2p), consensus (Aura & GRANDPA), and RPC endpoints. Configuration is located in `node/src/chain_spec.rs` and `node/src/service.rs`.
- **Runtime:** The state transition function (STF) defining the business logic of the blockchain. Oslo utilizes FRAME to compose multiple pallets into a unified runtime (`runtime/src/lib.rs`).
- **Pallets:** Individual modules handling specific functionality, such as balances, staking, EVM execution, and more.

## Development and Testing

Oslo includes a comprehensive TypeScript test suite for its EVM compatibility layer.

To run the TypeScript integration tests:
```sh
cd ts-tests
npm install
npm run test
```

## Contributing

Contributions are welcome! Please ensure that all code is properly formatted (`cargo fmt` and `npm run prettier`) before submitting a pull request.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
