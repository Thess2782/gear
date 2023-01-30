//! build script for gear-program cli
#![allow(dead_code)]

use frame_metadata::RuntimeMetadataPrefixed;
use parity_scale_codec::{Decode, Encode};
use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
use subxt_codegen::DerivesRegistry;
use syn::ItemMod;

const GENERATED_TITLE: &str = r#"
// Auto generated by subxt-cli
//
// subxt codegen | rustfmt --edition=2021
"#;

/// Generate api
fn codegen(mut encoded: &[u8], item_mod: ItemMod) -> String {
    let metadata =
        <RuntimeMetadataPrefixed as Decode>::decode(&mut encoded).expect("decode metadata failed");

    // Genreate code.
    let crate_path = Default::default();
    let generator = subxt_codegen::RuntimeGenerator::new(metadata);
    generator
        .generate_runtime(item_mod, DerivesRegistry::new(&crate_path), crate_path)
        .to_string()
}

/// Write API to disk
fn write_api(api: &str, path: PathBuf) {
    // format generated code
    let mut rustfmt = Command::new("rustfmt");
    let mut code = rustfmt
        .args(["--edition=2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // pipe api to rustfmt
    write!(code.stdin.as_mut().unwrap(), "{api}").unwrap();
    let output = code.wait_with_output().unwrap();

    // write api to disk
    fs::write(
        path,
        GENERATED_TITLE.to_owned().trim().to_owned()
            + "\n"
            + &String::from_utf8_lossy(&output.stdout),
    )
    .expect("write api failed");
}

/// Update runtime api
fn update_api() {
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not found")).join("metadata.rs");

    println!("{path:?}");

    #[cfg(any(
        all(feature = "gear", not(feature = "vara")),
        all(feature = "gear", feature = "vara")
    ))]
    {
        write_api(
            &codegen(
                &gear_runtime::Runtime::metadata().encode(),
                syn::parse_quote!(
                    pub mod gear_metadata {}
                ),
            ),
            path,
        );
    }

    #[cfg(all(feature = "vara", not(feature = "gear")))]
    {
        write_api(
            &codegen(
                &vara_runtime::Runtime::metadata().encode(),
                syn::parse_quote!(
                    pub mod vara_metadata {}
                ),
            ),
            path,
        );
    }

    // # NOTE
    //
    // post format code since `cargo +nightly fmt` doesn't support pipe
    let mut cargo = Command::new("cargo");
    cargo
        .args(["+nightly", "fmt"])
        .status()
        .expect("Format code failed.");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../runtime");
    println!("cargo:rerun-if-changed=../pallets/gear");

    update_api();
}
