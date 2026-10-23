use std::env;
use std::ffi::OsString;
use std::process;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rustc-check-cfg=cfg(exhaustive)");
    println!("cargo:rustc-check-cfg=cfg(source_format_debug)");
    println!("cargo:rustc-check-cfg=cfg(source_format_debug_indent)");

    let pkg_version = cargo_env_var("CARGO_PKG_VERSION");
    println!("cargo:VERSION={}", pkg_version.to_str().unwrap());

    let manifest_links = cargo_env_var("CARGO_MANIFEST_LINKS");
    assert_eq!(manifest_links, "rust_source_format");

}

fn cargo_env_var(key: &str) -> OsString {
    env::var_os(key).unwrap_or_else(|| {
        eprintln!("Environment variable ${key} is not set during execution of build script");
        process::exit(1);
    })
}
