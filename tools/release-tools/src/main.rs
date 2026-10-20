//! Validate a release tag and every first-party product manifest before building.
use std::{env, fs, io::Write, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn tag_version(tag: &str) -> Result<semver::Version> {
    let raw = tag.strip_prefix('v').ok_or("tag must start with v")?;
    let version = semver::Version::parse(raw)?;
    if version.to_string() != raw {
        return Err("tag must use canonical SemVer".into());
    }
    Ok(version)
}

fn toml_file(path: &Path) -> Result<toml::Value> {
    Ok(fs::read_to_string(path)?.parse()?)
}

fn validate(root: &Path, tag: &str) -> Result<semver::Version> {
    let version = tag_version(tag)?;
    let expected = version.to_string();
    let workspace = toml_file(&root.join("Cargo.toml"))?;
    let workspace_version = workspace["workspace"]["package"]["version"]
        .as_str()
        .ok_or("missing workspace version")?;
    if workspace_version != expected {
        return Err(format!("{tag} differs from workspace version {workspace_version}").into());
    }
    let mut paths = vec![];
    // Upstream-derived foundation and macro components retain upstream version
    // identities. Product components follow the root release version.
    for component in [
        "auth",
        "contract",
        "naming",
        "platform",
        "engine",
        "server",
        "client",
        "admin",
        "web",
        "interface",
        "smb",
        "smb-vfs",
    ] {
        paths.push(root.join("src").join(component).join("Cargo.toml"));
    }
    paths.sort();
    for path in paths {
        let manifest = toml_file(&path)?;
        let value = &manifest["package"]["version"];
        let actual = if value.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
            workspace_version
        } else {
            value.as_str().ok_or("missing package version")?
        };
        if actual != expected {
            return Err(format!(
                "{} has version {actual}, expected {expected}",
                path.display()
            )
            .into());
        }
    }
    Ok(version)
}

fn run() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let arg = env::args().nth(1).unwrap_or_default();
    let tag = if arg.is_empty() {
        let workspace = toml_file(&root.join("Cargo.toml"))?;
        format!(
            "v{}",
            workspace["workspace"]["package"]["version"]
                .as_str()
                .ok_or("missing workspace version")?
        )
    } else {
        arg
    };
    let version = validate(&root, &tag)?;
    println!("Validated {tag}");
    if let Some(output) = env::var_os("GITHUB_OUTPUT") {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(output)?;
        writeln!(
            file,
            "version={version}\ntag={tag}\nprerelease={}",
            !version.pre.is_empty()
        )?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Release validation failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_semver_only() {
        for tag in [
            "v0.1.0",
            "v1.2.3-rc.1",
            "v2.0.0+build.3",
            "v1.0.0-beta.2+abc",
        ] {
            assert!(tag_version(tag).is_ok(), "{tag}");
        }
        for tag in [
            "1.0.0",
            "v1.0",
            "v01.0.0",
            "v1.0.0-01",
            "v1.0.0\n",
            "v1.0.0;echo bad",
            "v1.0.0-",
        ] {
            assert!(tag_version(tag).is_err(), "{tag}");
        }
    }
    #[test]
    fn real_manifests_match_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let workspace = toml_file(&root.join("Cargo.toml")).unwrap();
        let version = workspace["workspace"]["package"]["version"]
            .as_str()
            .unwrap();
        assert!(validate(&root, &format!("v{version}")).is_ok());
        assert!(validate(&root, "v999999.0.0").is_err());
    }
}
