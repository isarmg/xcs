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
        "web/src",
        "web/runtime",
        "web/public",
        "vendor/web",
        "vendor/dependencies/unicode-normalization-0.1.24",
        "crates/zxs-contract",
        "crates/zxs-naming",
        "tools/web-build/src",
        ".cargo",
    ] {
        fs::create_dir_all(product.join(directory)).unwrap();
    }
    for file in [
        "Cargo.toml",
        "rust-toolchain.toml",
        ".cargo/config.toml",
        "web/Cargo.toml",
        "web/Cargo.lock",
        "web/styles.css",
        "tools/web-build/Cargo.toml",
        "tools/web-build/Cargo.lock",
    ] {
        fs::write(product.join(file), "fixture").unwrap();
    }
    let cargo = bin.join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" > \"$ZXS_CONTEXT_RECEIPT\"\nexit 0\n",
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o700)).unwrap();
    let receipt = root.join("receipt");
    let result = Command::new(env!("CARGO_BIN_EXE_zxs-web-build"))
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
    let text = fs::read_to_string(receipt).unwrap();
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
    assert!(!product.join("web/.build-lock").exists());
}
