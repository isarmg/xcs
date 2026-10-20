//! Standalone Rust builder; all demo inputs and outputs live in this directory.
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
#[path = "../../../../tools/web-build/src/bootstrap.rs"]
#[allow(dead_code)]
mod bootstrap;
#[path = "../entry.rs"]
mod entry;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn copy(source: &Path, dest: &Path) -> Result<()> {
    let meta = source.symlink_metadata()?;
    if meta.file_type().is_symlink() {
        return Err("symbolic links are not demo assets".into());
    }
    if meta.is_dir() {
        fs::create_dir_all(dest)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            copy(&item.path(), &dest.join(item.file_name()))?;
        }
    } else if meta.is_file() {
        fs::copy(source, dest)?;
    } else {
        return Err("special file is not a demo asset".into());
    }
    Ok(())
}
fn inventory(base: &Path, path: &Path, files: &mut BTreeMap<String, String>) -> Result<()> {
    let meta = path.symlink_metadata()?;
    if meta.file_type().is_symlink() {
        return Err("symbolic links are not demo inputs".into());
    }
    if meta.is_dir() {
        for item in fs::read_dir(path)? {
            inventory(base, &item?.path(), files)?;
        }
    } else if meta.is_file() {
        files.insert(
            path.strip_prefix(base)?
                .to_string_lossy()
                .replace('\\', "/"),
            format!("{:x}", Sha256::digest(fs::read(path)?)),
        );
    } else {
        return Err("special file is not a demo input".into());
    }
    Ok(())
}
fn inputs(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for input in [
        "Cargo.toml",
        "rust-toolchain.toml",
        "demos/branch-system/src",
        "demos/branch-system/reference",
        "demos/branch-system/examples.md",
        "demos/branch-system/styles.css",
        "demos/branch-system/Cargo.toml",
        "demos/branch-system/Cargo.lock",
        "web/runtime",
        "web/public/styles",
        "web/public/fonts",
        "web/styles.css",
        "vendor/web",
        "vendor/dependencies/unicode-normalization-0.1.24",
        "crates/zxs-contract",
        "crates/zxs-naming",
        "tools/web-build/src/bootstrap.rs",
    ] {
        inventory(root, &root.join(input), &mut out)?;
    }
    Ok(out)
}
fn main() -> Result<()> {
    let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = demo.parent().unwrap().parent().unwrap();
    let publish = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => None,
        [flag] if flag == "--publish-web" => Some(root.join("web/dist/demo")),
        _ => return Err("usage: zxs-demo-build [--publish-web]".into()),
    };
    let lock = demo.join(".build-lock");
    let _handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)?;
    struct Lock(PathBuf);
    impl Drop for Lock {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let _guard = Lock(lock);
    let before = inputs(root)?;
    let target = root.join("target/web");
    let status = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--offline",
            "--lib",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
        ])
        .arg(demo.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .status()?;
    if !status.success() {
        return Err("demo WASM build failed".into());
    }
    let bindgen = std::env::var_os("ZXS_WEB_BINDGEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("wasm-bindgen"));
    let version = Command::new(&bindgen).arg("--version").output()?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "wasm-bindgen 0.2.129"
    {
        return Err("wasm-bindgen 0.2.129 required".into());
    }
    let stage = demo.join(".dist-staging");
    if stage.exists() {
        if stage.symlink_metadata()?.file_type().is_symlink() {
            return Err("staging must not be a symlink".into());
        }
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(stage.join("pkg"))?;
    if !Command::new(&bindgen)
        .args([
            "--target",
            "web",
            "--no-typescript",
            "--out-name",
            "zxs",
            "--out-dir",
        ])
        .arg(stage.join("pkg"))
        .arg(target.join("wasm32-unknown-unknown/release/zxs_branch_demo.wasm"))
        .status()?
        .success()
    {
        return Err("demo binding generation failed".into());
    }
    for (source, destination) in [
        (root.join("web/public/styles"), stage.join("styles")),
        (root.join("web/public/fonts"), stage.join("fonts")),
        (root.join("web/styles.css"), stage.join("zxs.css")),
        (demo.join("styles.css"), stage.join("demo.css")),
        (demo.join("reference"), stage.join("reference")),
        (demo.join("examples.md"), stage.join("examples.md")),
    ] {
        copy(&source, &destination)?;
    }
    fs::write(stage.join("boot.js"), bootstrap::loader())?;
    fs::write(stage.join("index.html"), entry::html(false, "./"))?;
    fs::write(stage.join("playground.html"), entry::html(true, "./"))?;
    let mut gzip = flate2::GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), flate2::Compression::best());
    gzip.write_all(&fs::read(stage.join("pkg/zxs_bg.wasm"))?)?;
    fs::write(stage.join("pkg/zxs_bg.wasm.gz"), gzip.finish()?)?;
    let after = inputs(root)?;
    if before != after {
        return Err("demo inputs changed during build; retry".into());
    }
    let mut files = BTreeMap::new();
    inventory(&stage, &stage, &mut files)?;
    fs::write(
        stage.join("build-manifest.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"format_version":1,"runtime":"zxs-web","target":"wasm32-unknown-unknown","inputs":after,"files":files}),
        )?,
    )?;
    let dist = demo.join("dist");
    let backup = demo.join(".dist-backup");
    if backup.exists() {
        return Err("inspect interrupted demo publication (.dist-backup)".into());
    }
    let previous = dist.exists();
    if previous {
        if !dist.join("build-manifest.json").is_file()
            || dist.symlink_metadata()?.file_type().is_symlink()
        {
            return Err("unrecognized demo dist".into());
        }
        fs::rename(&dist, &backup)?;
    }
    if let Err(error) = fs::rename(&stage, &dist) {
        if previous {
            fs::rename(&backup, &dist)?;
        }
        return Err(error.into());
    }
    if previous {
        fs::remove_dir_all(backup)?;
    }
    fs::write(demo.join("index.html"), entry::html(false, "./dist/"))?;
    fs::write(demo.join("playground.html"), entry::html(true, "./dist/"))?;
    if let Some(destination) = publish {
        if !destination
            .parent()
            .unwrap()
            .join("build-manifest.json")
            .is_file()
        {
            return Err("build the main Web distribution before publishing its demo".into());
        }
        let stage = destination.with_file_name(".demo-staging");
        let backup = destination.with_file_name(".demo-backup");
        if stage.exists() || backup.exists() {
            return Err("inspect interrupted Web demo publication before retrying".into());
        }
        let previous = destination.exists();
        if previous
            && (destination.symlink_metadata()?.file_type().is_symlink()
                || !destination.join("build-manifest.json").is_file())
        {
            return Err("unrecognized Web demo directory".into());
        }
        copy(&dist, &stage)?;
        if previous {
            fs::rename(&destination, &backup)?;
        }
        if let Err(error) = fs::rename(&stage, &destination) {
            if previous {
                fs::rename(&backup, &destination)?;
            }
            return Err(error.into());
        }
        if previous {
            fs::remove_dir_all(&backup)?;
        }
    }
    println!("Demo built: {} (guided + playground)", dist.display());
    Ok(())
}
