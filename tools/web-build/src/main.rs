//! Rust-only production builder. JavaScript consists of generated WASM bindings
//! and a generated generic module loader; product behavior lives in Rust.
use anyhow::{Context, Result, bail};
mod bootstrap;
mod icon;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "build".into());
    let root = match args.next().as_deref() {
        Some("--product-root") => {
            PathBuf::from(args.next().context("missing product root")?).canonicalize()?
        }
        Some(value) => bail!("unknown option {value}"),
        None => std::env::current_dir()?.canonicalize()?,
    };
    if args.next().is_some() {
        bail!("unexpected argument");
    }
    if command == "icons" {
        return icon::generate(&root);
    }
    let web = root.join("web");
    if !web.join("Cargo.toml").is_file() {
        bail!("product does not contain web/Cargo.toml");
    }
    let lock_path = web.join(".build-lock");
    let _lock = BuildLock { path:lock_path.clone(), _file:fs::OpenOptions::new().write(true).create_new(true).open(&lock_path).context("another Web build is running (remove a stale web/.build-lock only after confirming its process exited)")? };
    let inputs_before = source_inventory(&root)?;
    let target = std::env::var_os("XCS_WEB_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/web"));
    let mut cargo = Command::new("cargo");
    cargo.arg(match command.as_str() {
        "build" => "build",
        "check" => "check",
        "test" => "test",
        _ => bail!("usage: xcs-web-build build|check|test [--product-root DIR]"),
    });
    cargo
        .args(["--locked", "--offline", "--manifest-path"])
        .arg(web.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target);
    if command != "test" {
        cargo.args(["--target", "wasm32-unknown-unknown"]);
    } else {
        cargo.arg("--workspace");
    }
    if command == "build" {
        cargo.arg("--release");
    }
    if !cargo.status().context("cannot run cargo")?.success() {
        bail!("browser Cargo {command} failed");
    }
    if command != "build" {
        return Ok(());
    }
    let bindgen = std::env::var_os("XCS_WEB_BINDGEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("wasm-bindgen"));
    let version = Command::new(&bindgen)
        .arg("--version")
        .output()
        .context("wasm-bindgen CLI 0.2.129 is required (XCS_WEB_BINDGEN may point to it)")?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "wasm-bindgen 0.2.129"
    {
        bail!("wasm-bindgen CLI must be exactly 0.2.129");
    }
    let staging = web.join(".dist-staging");
    if staging.exists() {
        reject_link(&staging)?;
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(staging.join("pkg"))?;
    let status = Command::new(bindgen)
        .args(["--target", "web", "--no-typescript", "--out-name", "xcs"])
        .arg("--out-dir")
        .arg(staging.join("pkg"))
        .arg(target.join("wasm32-unknown-unknown/release/xcs_web_app.wasm"))
        .status()?;
    if !status.success() {
        bail!("wasm-bindgen failed");
    }
    fs::copy(web.join("styles.css"), staging.join("xcs.css"))?;
    copy_tree(&web.join("public"), &staging)?;
    fs::write(staging.join("boot.js"), bootstrap::loader())?;
    fs::write(staging.join("index.html"), bootstrap::index())?;
    let wasm = staging.join("pkg/xcs_bg.wasm");
    let mut gzip = flate2::GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), flate2::Compression::best());
    gzip.write_all(&fs::read(&wasm)?)?;
    fs::write(wasm.with_extension("wasm.gz"), gzip.finish()?)?;
    let inputs_after = source_inventory(&root)?;
    if inputs_before != inputs_after {
        bail!("Web sources changed during compilation; retry with a stable source tree");
    }
    let mut inventory = BTreeMap::new();
    inventory_tree(&staging, &staging, &mut inventory)?;
    fs::write(
        staging.join("build-manifest.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"format_version":1,"runtime":"xcs-web","files":inventory,"inputs":inputs_after,"target":"wasm32-unknown-unknown","wasm_bindgen":"0.2.129"}),
        )?,
    )?;
    let dist = web.join("dist");
    let backup = web.join(".dist-backup");
    if backup.exists() {
        bail!("web/.dist-backup needs inspection after an interrupted publication");
    }
    let had_previous = dist.exists();
    if had_previous {
        reject_link(&dist)?;
        if !dist.join("build-manifest.json").is_file() {
            bail!("refusing to replace unrecognized web/dist");
        }
        fs::rename(&dist, &backup)?;
    }
    if let Err(error) = fs::rename(&staging, &dist) {
        if had_previous {
            fs::rename(&backup, &dist).context("failed to restore previous Web distribution")?;
        }
        return Err(error.into());
    }
    if had_previous {
        fs::remove_dir_all(&backup)?;
    }
    println!(
        "{}",
        serde_json::json!({"event":"built","dist":dist,"runtime":"xcs-web"})
    );
    Ok(())
}
struct BuildLock {
    path: PathBuf,
    _file: fs::File,
}
impl Drop for BuildLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
fn source_inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut inventory = BTreeMap::new();
    for path in [
        "web/src",
        "web/runtime",
        "web/public",
        "vendor/web",
        "vendor/dependencies/unicode-normalization-0.1.24",
        "crates/xcs-contract",
        "crates/xcs-naming",
        "tools/web-build/src",
    ] {
        inventory_tree(root, &root.join(path), &mut inventory)?;
    }
    for path in [
        "Cargo.toml",
        "web/Cargo.toml",
        "web/Cargo.lock",
        "web/styles.css",
        "tools/web-build/Cargo.toml",
        "tools/web-build/Cargo.lock",
    ] {
        let input = root.join(path);
        inventory.insert(
            path.into(),
            format!("{:x}", Sha256::digest(read_input(&input)?)),
        );
    }
    Ok(inventory)
}
fn input_type(path: &Path) -> Result<fs::FileType> {
    let kind = fs::symlink_metadata(path)?.file_type();
    if kind.is_symlink() {
        bail!("symbolic links are not build inputs: {}", path.display());
    }
    if !kind.is_file() && !kind.is_dir() {
        bail!("special files are not build inputs: {}", path.display());
    }
    Ok(kind)
}
fn reject_link(path: &Path) -> Result<()> {
    input_type(path).map(|_| ())
}
fn read_input(path: &Path) -> Result<Vec<u8>> {
    if !input_type(path)?.is_file() {
        bail!("expected regular build input: {}", path.display());
    }
    Ok(fs::read(path)?)
}
fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    if !input_type(source)?.is_dir() {
        bail!("expected build input directory: {}", source.display());
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let kind = input_type(&path)?;
        let output = destination.join(entry.file_name());
        if kind.is_dir() {
            fs::create_dir_all(&output)?;
            copy_tree(&path, &output)?;
        } else {
            fs::copy(&path, &output)?;
        }
    }
    Ok(())
}
fn inventory_tree(
    root: &Path,
    directory: &Path,
    output: &mut BTreeMap<String, String>,
) -> Result<()> {
    if !input_type(directory)?.is_dir() {
        bail!("expected inventory directory: {}", directory.display());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if input_type(&path)?.is_dir() {
            inventory_tree(root, &path, output)?;
        } else {
            output.insert(
                path.strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
                format!("{:x}", Sha256::digest(read_input(&path)?)),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "xcs-build-inputs-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn inventory_and_copy_share_regular_file_bytes_and_nested_paths() {
        let fixture = Fixture::new();
        let source = fixture.0.join("source");
        let output = fixture.0.join("output");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::create_dir(&output).unwrap();
        fs::write(source.join("nested/file.css"), b"body {}").unwrap();
        copy_tree(&source, &output).unwrap();
        let mut inventory = BTreeMap::new();
        inventory_tree(&source, &source, &mut inventory).unwrap();
        assert_eq!(
            inventory["nested/file.css"],
            format!("{:x}", Sha256::digest(b"body {}"))
        );
        assert_eq!(
            fs::read(output.join("nested/file.css")).unwrap(),
            b"body {}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn inventory_rejects_linked_roots_children_and_nonregular_inputs() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let root = fixture.0.join("source");
        fs::create_dir(&root).unwrap();
        symlink(&root, fixture.0.join("linked-root")).unwrap();
        assert!(
            inventory_tree(
                &fixture.0,
                &fixture.0.join("linked-root"),
                &mut BTreeMap::new()
            )
            .is_err()
        );
        fs::write(root.join("real.css"), b"body {}").unwrap();
        symlink(root.join("real.css"), root.join("linked.css")).unwrap();
        assert!(inventory_tree(&root, &root, &mut BTreeMap::new()).is_err());
        assert!(read_input(&root.join("linked.css")).is_err());
        fs::remove_file(root.join("linked.css")).unwrap();
        let fifo = root.join("blocked.css");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            read_input(&fifo)
                .unwrap_err()
                .to_string()
                .contains("special files")
        );
        assert!(inventory_tree(&root, &root, &mut BTreeMap::new()).is_err());
        let output = fixture.0.join("output");
        fs::create_dir(&output).unwrap();
        assert!(copy_tree(&root, &output).is_err());
    }
}
