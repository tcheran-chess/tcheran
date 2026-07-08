use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::preprocessing::RawNetwork;

mod preprocessing;

// Make the network architecture definition available for preprocessing
include!("src/engine/eval/nnue/network.rs");

const NETWORK_FILE: &str = "v22-e81d7256.nnue";
const DOWNLOAD_BASE_URL: &str =
    "https://github.com/tcheran-chess/tcheran-networks/releases/download/networks";

fn main() {
    generate_engine_version();

    let network_file = setup_network();
    let network_file = preprocess_network(&network_file);
    println!("cargo:rustc-env=NETWORK={}", network_file.display());

    build_fathom();
}

fn generate_engine_version() {
    let cargo_version = env!("CARGO_PKG_VERSION");

    let (version, suffix) = match cargo_version.split_once('-') {
        Some((version, suffix)) => (version, generate_version_suffix(suffix)),
        None => (cargo_version, String::new()),
    };

    let version = version.strip_suffix(".0").unwrap();

    println!("cargo:rustc-env=ENGINE_VERSION=v{version}{suffix}");
}

fn generate_version_suffix(suffix: &str) -> String {
    let git_sha = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|v| v.status.success())
        .and_then(|v| String::from_utf8(v.stdout).ok())
        .map(|v| v.trim().to_string());

    if let Some(sha) = git_sha {
        format!("-{suffix}-{sha}")
    } else {
        format!("-{suffix}")
    }
}

fn setup_network() -> PathBuf {
    println!("cargo:rerun-if-env-changed=EVALFILE");

    // OpenBench might provide us with a file (by setting EVALFILE)
    // If so, check that it exists - but just use that.
    if let Ok(eval_file) = env::var("EVALFILE") {
        let path = relative_to_project_root(&eval_file);
        assert!(path.exists(), "EVALFILE was {}, but not found at {}", eval_file, path.display());
        return path;
    }

    let path = relative_to_project_root(Path::new("data").join(NETWORK_FILE));
    if !path.exists() {
        download_network(&path);
    }

    println!("cargo:rerun-if-changed={}", path.display());
    path
}

fn relative_to_project_root(path: impl Into<PathBuf>) -> PathBuf {
    let path = path.into();

    if !path.is_relative() {
        return path;
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Should be able to find engine/..")
        .join(path)
}

fn download_network(download_to: &Path) {
    let download_succeeded = Command::new("curl")
        .args(vec![
            "--fail",
            "--silent",
            "--location",
            "--create-dirs",
            "--output",
            &download_to.display().to_string(),
            &format!("{DOWNLOAD_BASE_URL}/{NETWORK_FILE}"),
        ])
        .status()
        .unwrap();

    assert!(download_succeeded.success(), "Unable to download network");
    println!("cargo::warning=Fetched network: {NETWORK_FILE}");
}

pub fn preprocess_network(file: &PathBuf) -> PathBuf {
    let raw_bytes = fs::read(file).expect("Unable to read file");
    assert_eq!(raw_bytes.len(), size_of::<RawNetwork>());

    let raw_network = unsafe {
        let mut network: Box<RawNetwork> = Box::new_zeroed().assume_init();
        std::ptr::copy_nonoverlapping(
            raw_bytes.as_ptr(),
            std::ptr::from_mut(network.as_mut()).cast::<u8>(),
            raw_bytes.len(),
        );
        network
    };

    let mut network = unsafe {
        let network: Box<Network> = Box::new_zeroed().assume_init();
        network
    };

    preprocessing::preprocess(&raw_network, &mut network);

    let bytes = unsafe {
        std::slice::from_raw_parts(
            std::ptr::from_ref::<Network>(network.as_ref()).cast::<u8>(),
            size_of::<Network>(),
        )
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let output_file = out_dir.join(NETWORK_FILE);
    fs::write(&output_file, bytes).expect("Unable to write file");
    output_file
}

fn build_fathom() {
    println!("cargo:rerun-if-changed=src/engine/tablebases/fathom/src");

    cc::Build::new()
        .include("src/engine/tablebases/fathom/src")
        .file("src/engine/tablebases/fathom/src/tbprobe.c")
        .compile("fathom");
}
