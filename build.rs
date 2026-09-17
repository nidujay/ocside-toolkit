// SPDX-License-Identifier: LGPL-3.0-or-later
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/foreign");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let profile = env::var("PROFILE").unwrap();

    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));

    let output_dir = target_dir.join(&profile).join("include");

    fs::create_dir_all(&output_dir).unwrap();

    cbindgen::Builder::new()
        .with_crate(&manifest_dir)
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(output_dir.join("win32_impl.h"));
}
