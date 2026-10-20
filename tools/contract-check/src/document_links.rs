//! Source links stay mandatory. Links to excluded, local validation output are
//! reported separately and may be absent in a source-only checkout.
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Target {
    pub path: Option<PathBuf>,
    pub generated: bool,
}
fn generated(relative: &Path) -> bool {
    relative.starts_with("tests/evidence")
        && !matches!(
            relative.extension().and_then(|s| s.to_str()),
            Some("md" | "rs" | "mjs" | "py" | "sh")
        )
}
fn normalize(path: &Path) -> Result<PathBuf> {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return Err("link escapes filesystem root".into());
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}
pub fn resolve(root: &Path, document: &Path, relative: &str) -> Result<Target> {
    let raw = if relative.is_empty() {
        document.to_owned()
    } else {
        document
            .parent()
            .ok_or("document parent missing")?
            .join(relative)
    };
    let path = normalize(&raw)?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("link outside repository: {}", path.display()))?;
    let generated = generated(relative);
    match path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(root) {
                return Err(format!("link outside repository: {}", canonical.display()).into());
            }
            Ok(Target {
                path: Some(canonical),
                generated,
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound && generated => {
            // An excluded output below a link to another repository must not
            // evade the same boundary enforced for available documents.
            for ancestor in path.ancestors() {
                match fs::symlink_metadata(ancestor) {
                    Ok(_) => {
                        if !ancestor.canonicalize()?.starts_with(root) {
                            return Err("generated link outside repository".into());
                        }
                        break;
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(Target {
                path: None,
                generated: true,
            })
        }
        Err(error) => Err(format!(
            "missing local link {} in {}: {error}",
            path.display(),
            document.display()
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("zxs-link-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn excludes_only_generated_output_in_declared_evidence_directories() {
        for path in [
            "tests/evidence/run/result.json",
            "tests/evidence/lazy-branches/shot.png",
            "tests/evidence/recovery/run/build.log",
        ] {
            assert!(generated(Path::new(path)), "{path}");
        }
        for path in [
            "tests/fixtures/result.json",
            "docs/contracts/vectors.json",
            "docs/images/web-file-manager.png",
            "tests/evidence/run/README.md",
            "tests/evidence/run/probe.mjs",
            "tests/evidence/run/seed.rs",
            "tests/evidence/run/helper.py",
            "tests/evidence/run/check.sh",
            "docs/validation/evidence/README.md",
        ] {
            assert!(!generated(Path::new(path)), "{path}");
        }
    }
    #[test]
    fn missing_outputs_are_optional_but_missing_documents_and_scripts_fail() {
        let f = Fixture::new();
        let doc = f.0.join("README.md");
        let target = resolve(&f.0, &doc, "tests/evidence/run/result.json").unwrap();
        assert!(target.path.is_none() && target.generated);
        for path in [
            "docs/missing.md",
            "tests/fixtures/missing.json",
            "tests/evidence/run/README.md",
            "tests/evidence/run/probe.mjs",
        ] {
            assert!(resolve(&f.0, &doc, path).is_err(), "{path}");
        }
        fs::create_dir_all(f.0.join("tests/evidence/run")).unwrap();
        fs::write(f.0.join("tests/evidence/run/result.json"), b"{}").unwrap();
        let target = resolve(&f.0, &doc, "tests/evidence/run/result.json").unwrap();
        assert!(target.path.is_some() && target.generated);
    }
    #[test]
    fn normalize_before_applying_generated_output_exception() {
        let f = Fixture::new();
        let doc = f.0.join("README.md");
        assert!(resolve(&f.0, &doc, "tests/evidence/../../docs/missing.json").is_err());
        assert!(resolve(&f.0, &doc, "../outside.json").is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_turn_an_external_path_into_optional_local_output() {
        let f = Fixture::new();
        let outside = Fixture::new();
        fs::create_dir(f.0.join("tests")).unwrap();
        std::os::unix::fs::symlink(&outside.0, f.0.join("tests/evidence")).unwrap();
        assert!(resolve(&f.0, &f.0.join("README.md"), "tests/evidence/missing.json").is_err());
    }
}
