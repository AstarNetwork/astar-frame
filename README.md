![astar-cover](https://user-images.githubusercontent.com/40356749/135799652-175e0d24-1255-4c26-87e8-447b192fd4b2.gif)

<div align="center">

[![Integration Action](https://github.com/PlasmNetwork/Astar/workflows/Integration/badge.svg)](https://github.com/AstarNetwork/astar-frame/actions)
[![GitHub tag (latest by date)](https://img.shields.io/github/v/tag/PlasmNetwork/Astar)](https://github.com/AstarNetwork/astar-frame/tags)
[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-brightgreen?logo=Parity%20Substrate)](https://substrate.dev/)
[![License](https://img.shields.io/github/license/PlasmNetwork/Astar?color=green)](https://github.com/AstarNetwork/astar-frame/LICENSE)
 <br />
[![Twitter URL](https://img.shields.io/twitter/follow/AstarNetwork?style=social)](https://twitter.com/AstarNetwork)
[![Twitter URL](https://img.shields.io/twitter/follow/ShidenNetwork?style=social)](https://twitter.com/ShidenNetwork)
[![YouTube](https://img.shields.io/youtube/channel/subscribers/UC36JgEF6gqatVSK9xlzzrvQ?style=social)](https://www.youtube.com/channel/UC36JgEF6gqatVSK9xlzzrvQ)
[![Docker](https://img.shields.io/docker/pulls/staketechnologies/astar-collator?logo=docker)](https://hub.docker.com/r/staketechnologies/astar-collator)
[![Discord](https://img.shields.io/badge/Discord-gray?logo=discord)](https://discord.gg/Z3nC9U4)
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

## Further Reading

* [Official Documentation](https://docs.astar.network/)
* [Whitepaper](https://github.com/PlasmNetwork/plasmdocs/blob/master/wp/en.pdf)
* [Whitepaper(JP)](https://github.com/PlasmNetwork/plasmdocs/blob/master/wp/jp.pdf)
* [Subtrate Developer Hub](https://substrate.dev/docs/en/)
* [Substrate Glossary](https://substrate.dev/docs/en/knowledgebase/getting-started/glossary)
* [Substrate Client Library Documentation](https://polkadot.js.org/docs/)
