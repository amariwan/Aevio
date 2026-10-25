use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os == "macos" {
        build_ml_sidecar();
    }
    if os == "macos" || os == "windows" {
        build_cli_sidecar();
    }
    tauri_build::build()
}

fn build_cli_sidecar() {
    println!("cargo:rerun-if-changed=../../cli/Cargo.toml");
    println!("cargo:rerun-if-changed=../../cli/src");
    println!("cargo:rerun-if-changed=../../api/Cargo.toml");
    println!("cargo:rerun-if-changed=../../api/src");
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let target = env::var("TARGET").expect("TARGET");
    let output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("cli-target");
    let status = Command::new(env::var_os("CARGO").expect("CARGO"))
        .args([
            "build",
            "--locked",
            "--release",
            "--package",
            "aevio-cli",
            "--target",
        ])
        .arg(&target)
        .arg("--manifest-path")
        .arg(manifest.join("../../cli/Cargo.toml"))
        .arg("--target-dir")
        .arg(&output)
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .status()
        .expect("could not build the CLI sidecar");
    assert!(status.success(), "CLI sidecar build failed: {status}");
    let suffix = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let built = output.join(&target).join(format!("release/aevio{suffix}"));
    let staged = manifest
        .join("binaries")
        .join(format!("aevio-cli-{target}{suffix}"));
    if !matches!((fs::read(&built), fs::read(&staged)), (Ok(a), Ok(b)) if a == b) {
        fs::create_dir_all(staged.parent().expect("binaries dir")).expect("binaries dir");
        fs::copy(built, &staged).expect("could not stage the CLI sidecar");
    }
}

fn build_ml_sidecar() {
    println!("cargo:rerun-if-changed=swift/Package.swift");
    println!("cargo:rerun-if-changed=swift/Sources");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let package = manifest_dir.join("swift");
    let target = env::var("TARGET").expect("TARGET");
    let arch = if target.starts_with("x86_64") {
        "x86_64"
    } else {
        "arm64"
    };
    let args = [
        "build",
        "-c",
        "release",
        "--arch",
        arch,
        "--product",
        "aevio-ml",
        "--package-path",
    ];

    let status = Command::new("swift")
        .args(args)
        .arg(&package)
        .status()
        .unwrap_or_else(|error| {
            eprintln!("could not run `swift build` for the ML sidecar: {error}");
            stage_placeholder(&manifest_dir, &target);
            std::process::exit(0);
        });
    if !status.success() {
        eprintln!(
            "`swift build` for the ML sidecar failed ({status}); building without \
             on-device AI (needs Xcode 26+)."
        );
        stage_placeholder(&manifest_dir, &target);
        return;
    }

    let output = Command::new("swift")
        .args(args)
        .arg(&package)
        .arg("--show-bin-path")
        .output()
        .expect("could not ask swift for the sidecar bin path");
    let bin_dir = String::from_utf8(output.stdout).expect("utf-8 bin path");
    let built = Path::new(bin_dir.trim()).join("aevio-ml");

    let staged = manifest_dir
        .join("binaries")
        .join(format!("aevio-ml-{target}"));
    let unchanged = matches!(
        (fs::read(&built), fs::read(&staged)),
        (Ok(fresh), Ok(current)) if fresh == current
    );
    if !unchanged {
        fs::create_dir_all(staged.parent().expect("binaries dir"))
            .expect("could not create binaries dir");
        fs::copy(&built, &staged).unwrap_or_else(|error| {
            panic!(
                "could not stage the ML sidecar at {}: {error}",
                staged.display()
            )
        });
    }
}

fn stage_placeholder(manifest_dir: &Path, target: &str) {
    let staged = manifest_dir
        .join("binaries")
        .join(format!("aevio-ml-{target}"));
    if let Some(parent) = staged.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if !staged.exists() {
        let _ = fs::write(&staged, []);
    }
}
