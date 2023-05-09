![astar-cover](https://user-images.githubusercontent.com/40356749/135799652-175e0d24-1255-4c26-87e8-447b192fd4b2.gif)

<div align="center">

[![Integration Action](https://github.com/AstarNetwork/Astar/workflows/Integration/badge.svg)](https://github.com/AstarNetwork/astar-frame/actions)
[![GitHub tag (latest by date)](https://img.shields.io/github/v/tag/AstarNetwork/Astar)](https://github.com/AstarNetwork/astar-frame/tags)
[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.dev/)
[![License](https://img.shields.io/github/license/AstarNetwork/Astar?color=green)](https://github.com/AstarNetwork/astar-frame/LICENSE)
 <br />
[![Twitter URL](https://img.shields.io/twitter/follow/AstarNetwork?style=social)](https://twitter.com/AstarNetwork)
[![Twitter URL](https://img.shields.io/twitter/follow/ShidenNetwork?style=social)](https://twitter.com/ShidenNetwork)
[![YouTube](https://img.shields.io/youtube/channel/subscribers/UC36JgEF6gqatVSK9xlzzrvQ?style=social)](https://www.youtube.com/channel/UC36JgEF6gqatVSK9xlzzrvQ)
[![Docker](https://img.shields.io/docker/pulls/staketechnologies/astar-collator?logo=docker)](https://hub.docker.com/r/staketechnologies/astar-collator)
[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/astarnetwork)
[![Telegram](https://img.shields.io/badge/Telegram-gray?logo=telegram)](https://t.me/PlasmOfficial)
[![Medium](https://img.shields.io/badge/Medium-gray?logo=medium)](https://medium.com/astar-network)

</div>

Astar Network is an interoperable blockchain based the Substrate framework and the hub for dApps within the Polkadot Ecosystem.
With Astar Network and Shiden Network, people can stake their tokens to a Smart Contract for rewarding projects that provide value to the network.

This repository only contains custom frame modules, for runtimes and node code please check [here](https://github.com/AstarNetwork/Astar/).

For contributing to this project, please read our [Contribution Guideline](./CONTRIBUTING.md).

## Versioning Schema

Repository doesn't have a dedicated master branch, instead the main branch is assigned to the branch of active polkadot version, e.g. `polkadot-v0.9.13`.
All deliveries should be made to the default branch unless they are intended for another temporary branch.

When a pallet has been modified (version in .toml is updated), a new release tag must be created.
Naming format for the tag is:
*pallet-name*-**toml-version**/polkadot-**version**

E.g. `pallet-dapps-staking-1.1.2/polkadot-v0.9.13`.

### dApps-staking Pallets

Since both `pallet-dapps-staking` and `pallet-precompile-dapps-staking` are tightly related, we use a direct dependency (**path**) from the precompile to FRAME pallet. Due to this, both pallet versions should be the same (incrementing one also means that other should be incremented).

When creating tags, it is sufficient to just create a single tag for `pallet-dapps-staking` and reuse it for the precompiles in `Astar` repo.

## Workspace Dependency Handling

All dependencies should be listed inside the workspace's root `Cargo.toml` file.
This allows us to easily change version of a crate used by the entire repo by modifying the version in a single place.

Right now, if **non_std** is required, `default-features = false` must be set in the root `Cargo.toml` file (related to this [issue](https://github.com/rust-lang/cargo/pull/11409)). Otherwise, it will have no effect, causing your compilation to fail.
Also `package` imports aren't properly propagated from root to sub-crates, so defining those should be avoided.

Defining _features_ in the root `Cargo.toml` is additive with the features defined in concrete crate's `Cargo.toml`.

**Adding Dependency**
1. Check if the dependency is already defined in the root `Cargo.toml`
    1. if **yes**, nothing to do, just take note of the enabled features
    2. if **no**, add it (make sure to use `default-features = false` if dependency is used in _no_std_ context)
2. Add `new_dependecy = { workspace = true }` to the required crate
3. In case dependency is defined with `default-features = false` but you need it in _std_ context, add `features = ["std"]` to the required crate.

## Further Reading

* [Official Documentation](https://docs.astar.network/)
* [Whitepaper](https://github.com/AstarNetwork/plasmdocs/blob/master/wp/en.pdf)
* [Whitepaper(JP)](https://github.com/AstarNetwork/plasmdocs/blob/master/wp/jp.pdf)
* [Subtrate Developer Hub](https://substrate.dev/docs/en/)
* [Substrate Glossary](https://substrate.dev/docs/en/knowledgebase/getting-started/glossary)
* [Substrate Client Library Documentation](https://polkadot.js.org/docs/)
