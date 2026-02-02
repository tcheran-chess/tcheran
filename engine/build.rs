use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

const NETWORK: &str = "v14-fe4a2725.nnue";
const DOWNLOAD_BASE_URL: &str =
    "https://github.com/tcheran-chess/tcheran-networks/releases/download/networks";

fn main() {
    generate_engine_version();

    let network_file = setup_network();
    println!("cargo:rustc-env=NETWORK={}", network_file.display());

    build_fathom();
}

fn generate_engine_version() {
    let cargo_version = env!("CARGO_PKG_VERSION");

    let (version, suffix) = match cargo_version.split_once('-') {
        Some((version, suffix)) => (version, format!("-{suffix}")),
        None => (cargo_version, String::new()),
    };

    let version = version.strip_suffix(".0").unwrap();

    println!("cargo:rustc-env=ENGINE_VERSION=v{version}{suffix}");
}

fn setup_network() -> PathBuf {
    println!("cargo:rerun-if-env-changed=EVALFILE");

    // OpenBench might provide us with a file (by setting EVALFILE)
    // If so, check that it exists - but just use that.
    if let Ok(eval_file) = env::var("EVALFILE") {
        let path = relative_to_project_root(&eval_file);
        assert!(path.exists(), "EVALFILE was {}, but not found at {}", &eval_file, path.display());
        return path;
    }

    let path = relative_to_project_root(Path::new("data").join(NETWORK));
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
            "--location",
            "--create-dirs",
            "--output",
            &download_to.display().to_string(),
            &format!("{DOWNLOAD_BASE_URL}/{NETWORK}"),
        ])
        .status()
        .unwrap();

    assert!(download_succeeded.success(), "Unable to download network");
}

fn build_fathom() {
    println!("cargo:rerun-if-changed=src/engine/tablebases/fathom/src");

    cc::Build::new()
        .include("src/engine/tablebases/fathom/src")
        .file("src/engine/tablebases/fathom/src/tbprobe.c")
        .compile("fathom");
}
