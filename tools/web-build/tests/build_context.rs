//! Probe the actual child process without compiling WASM or changing real assets.
#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cargo_uses_product_configuration_but_relative_output_stays_caller_relative() {
    let root = std::env::temp_dir().join(format!(
        "zxs-build-context-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _cleanup = Fixture(root.clone());
    let product = root.join("product");
    let caller = root.join("caller");
    let bin = root.join("bin");
    fs::create_dir_all(&caller).unwrap();
    fs::create_dir_all(&bin).unwrap();
    for directory in [
        "src/web/src",
        "src/interface",
        "src/web/public",
        "src/foundation",
        "src/macros",
        "src/contract",
        "src/naming",
        "tools/web-build/src",
        "tools/web-bindings/src",
        "compliance",
        ".cargo",
        "docs",
        "tools/rust-sources",
        ".github/scripts",
    ] {
        fs::create_dir_all(product.join(directory)).unwrap();
    }
    for file in [
        "Cargo.toml",
        "rust-toolchain.toml",
        ".cargo/config.toml",
        "compliance/rust-sources.json",
        "docs/THIRD_PARTY_RUST_SOURCES.md",
        "tools/rust-sources/manage.py",
        ".github/scripts/rust-build-tool.sh",
        "src/web/Cargo.toml",
        "Cargo.lock",
        "src/web/styles.css",
        "tools/web-build/Cargo.toml",
        "tools/web-bindings/Cargo.toml",
    ] {
        fs::write(product.join(file), "fixture").unwrap();
    }
    let cargo = bin.join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" > \"$ZXS_CONTEXT_RECEIPT\"\nprintf 'profile:%s/%s/%s\\n' \"$CARGO_PROFILE_RELEASE_OPT_LEVEL\" \"$CARGO_PROFILE_RELEASE_LTO\" \"$CARGO_PROFILE_RELEASE_STRIP\" >> \"$ZXS_CONTEXT_RECEIPT\"\nif [ \"$1\" = build ]; then exit 17; fi\nexit 0\n",
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let receipt = root.join("receipt");
    let result = Command::new(env!("CARGO_BIN_EXE_web-build"))
        .current_dir(&caller)
        .args(["check", "--product-root"])
        .arg(&product)
        .env("PATH", &bin)
        .env("ZXS_CONTEXT_RECEIPT", &receipt)
        .env("ZXS_WEB_TARGET_DIR", "custom-target")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = fs::read_to_string(&receipt).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines[0], product.to_str().unwrap());
    let index = lines
        .iter()
        .position(|line| *line == "--target-dir")
        .unwrap();
    assert_eq!(
        lines[index + 1],
        caller.join("custom-target").to_str().unwrap()
    );
    let package = lines.iter().position(|line| *line == "--package").unwrap();
    assert_eq!(lines[package + 1], "web-app");
    assert!(!lines.contains(&"--manifest-path"));
    assert!(!product.join("build/.web-build-lock").exists());

    let test = Command::new(env!("CARGO_BIN_EXE_web-build"))
        .current_dir(&caller)
        .args(["test", "--product-root"])
        .arg(&product)
        .env("PATH", &bin)
        .env("ZXS_CONTEXT_RECEIPT", &receipt)
        .output()
        .unwrap();
    assert!(test.status.success());
    let text = fs::read_to_string(&receipt).unwrap();
    let lines: Vec<_> = text.lines().collect();
    for package in ["web-app", "web-interface", "view-macros"] {
        assert!(lines.windows(2).any(|args| args == ["--package", package]));
    }
    assert!(
        !lines.contains(&"--lib"),
        "run interface integration tests too"
    );
    assert!(
        !lines.contains(&"--tests"),
        "include consumer Rustdoc tests"
    );
    assert!(!lines.contains(&"--target"), "tests run on the host");

    // An inherited native profile must not add megabytes of function names to
    // browser delivery. Stop at Cargo so this process probe needs no real WASM.
    let python = bin.join("python3");
    fs::write(&python, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&python, fs::Permissions::from_mode(0o700)).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_web-build"))
        .current_dir(&caller)
        .args(["build", "--product-root"])
        .arg(&product)
        .env("PATH", &bin)
        .env("ZXS_CONTEXT_RECEIPT", &receipt)
        .env("CARGO_PROFILE_RELEASE_OPT_LEVEL", "3")
        .env("CARGO_PROFILE_RELEASE_LTO", "thin")
        .env("CARGO_PROFILE_RELEASE_STRIP", "debuginfo")
        .output()
        .unwrap();
    assert!(!build.status.success());
    assert!(String::from_utf8_lossy(&build.stderr).contains("browser Cargo build failed"));
    assert!(
        fs::read_to_string(&receipt)
            .unwrap()
            .lines()
            .any(|line| line == "profile:s/fat/symbols")
    );
    assert!(!product.join("build/.web-build-lock").exists());
}
