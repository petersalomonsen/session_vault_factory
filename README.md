# Session Vault Factory

A factory contract for deploying and managing session_vault vesting contracts on NEAR Protocol. This project addresses [neardevhub-treasury-dashboard#655](https://github.com/NEAR-DevHub/neardevhub-treasury-dashboard/issues/655).

## Overview

The Session Vault Factory enables automated detection and tracking of locked tokens in NEAR treasuries by providing a centralized factory for deploying session_vault contracts. This solves the visibility problem where treasuries cannot automatically detect locked tokens without a central repository of vesting contract instances.

## Features

- **Factory Pattern**: Deploy new session_vault contract instances through a single factory
- **Instance Tracking**: Maintain a registry of all deployed session_vault contracts
- **Pagination Support**: Efficiently list and query deployed instances with pagination
- **Code Hash Deployment**: Uses global contract deployment by code hash for consistency
- **Treasury Integration**: Enables NEAR treasuries to automatically detect and display locked fungible tokens

## How to Build Locally?

Install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near build
```

## How to Test Locally?

```bash
cargo test
```

## How to Deploy?

Deployment is automated with GitHub Actions CI/CD pipeline.
To deploy manually, install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near deploy build-reproducible-wasm <account-id>
```

## Useful Links

- [cargo-near](https://github.com/near/cargo-near) - NEAR smart contract development toolkit for Rust
- [near CLI](https://near.cli.rs) - Interact with NEAR blockchain from command line
- [NEAR Rust SDK Documentation](https://docs.near.org/sdk/rust/introduction)
- [NEAR Documentation](https://docs.near.org)
- [NEAR StackOverflow](https://stackoverflow.com/questions/tagged/nearprotocol)
- [NEAR Discord](https://near.chat)
- [NEAR Telegram Developers Community Group](https://t.me/neardev)
- NEAR DevHub: [Telegram](https://t.me/neardevhub), [Twitter](https://twitter.com/neardevhub)
