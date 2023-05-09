// This file is part of Astar.

// Copyright (C) 2019-2023 Stake Technologies Pte.Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later

// Astar is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Astar is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Astar. If not, see <http://www.gnu.org/licenses/>.

/// build.rs

use std::{
    fs,
    path::{Path, PathBuf},
};

use contract_build::{
    BuildArtifacts, BuildMode, Features, ManifestPath, Network, OptimizationPasses, OutputType,
    Target, UnstableFlags, Verbosity,
};

/// Execute the clousre with given directory as current dir
fn with_directory<T, F: FnOnce() -> T>(f: F, dir: &Path) -> T {
    let curr_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(dir).unwrap();
    let res = f();
    std::env::set_current_dir(curr_dir).unwrap();

    res
}

/// Build the contracts and copy the artifacts to fixtures dir
fn build_contract(fixtures_dir: &Path, dir: &Path, name: &str) {
    println!("[build.rs] Building Contract - {name}");

    let build = with_directory(
        || {
            let manifest_path = ManifestPath::new("Cargo.toml").unwrap();

            let args = contract_build::ExecuteArgs {
                manifest_path,
                verbosity: Verbosity::Verbose,
                build_mode: BuildMode::Debug,
                features: Features::default(),
                network: Network::Online,
                build_artifact: BuildArtifacts::All,
                unstable_flags: UnstableFlags::default(),
                optimization_passes: Some(OptimizationPasses::default()),
                keep_debug_symbols: true,
                lint: false,
                output_type: OutputType::HumanReadable,
                skip_wasm_validation: false,
                target: Target::Wasm,
            };

            contract_build::execute(args).expect(&format!("Failed to build contract at - {dir:?}"))
        },
        dir,
    );

    // copy wasm artifact
    fs::copy(
        build.dest_wasm.unwrap(),
        fixtures_dir.join(format!("{name}.wasm")),
    )
    .unwrap();

    // copy metadata
    fs::copy(
        build.metadata_result.unwrap().dest_metadata,
        fixtures_dir.join(format!("{name}.json")),
    )
    .unwrap();
}

fn setup() -> (PathBuf, PathBuf) {
    let fixture_env = std::env::var("CB_FIXTURES_DIR").unwrap_or("fixtures".to_string());
    let fixtures_dir = Path::new(&fixture_env);

    let contracts_env =
        std::env::var("CB_CONTRACTS_DIR").unwrap_or("../contract-examples".to_string());
    let contracts_dir = Path::new(&contracts_env);

    // create fixtures dir if not exists
    fs::create_dir_all(fixtures_dir).unwrap();

    (fixtures_dir.to_path_buf(), contracts_dir.to_path_buf())
}

fn main() {
    let (fixtures_dir, contracts_dir) = setup();
    build_contract(
        &fixtures_dir,
        &contracts_dir.join("basic-flip"),
        "basic_flip",
    );

    println!("cargo:rerun-if-changed={}", contracts_dir.to_str().unwrap());
}
