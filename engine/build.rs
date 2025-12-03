use std::{
    env,
    path::{Path, PathBuf},
};

const NETWORK: &str = "network-108b5.bin";

fn main() {
    set_network_env_var();
    build_fathom();
}

fn set_network_env_var() {
    let mut path =
        env::var("EVALFILE").map_or_else(|_| Path::new("data").join(NETWORK), PathBuf::from);

    if path.is_relative() {
        path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Should be able to find engine/..")
            .join(path);
    }

    println!("cargo:rustc-env=NETWORK={}", path.display());
}

fn build_fathom() {
    println!("cargo:rerun-if-changed=src/engine/tablebases/fathom/src");

    cc::Build::new()
        .include("src/engine/tablebases/fathom/src")
        .file("src/engine/tablebases/fathom/src/tbprobe.c")
        .compile("fathom");
}
