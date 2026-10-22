#[path = "../../../tools/contract-check/src/document_scan.rs"]
mod document_scan;
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("document-scan-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn check(root: &Path, archive: &Path, expected: Result<Vec<PathBuf>, io::ErrorKind>) {
    let actual = document_scan::scan(root, archive).map_err(|error| error.kind());
    #[cfg(walker_oracle)]
    {
        let mut old = Vec::new();
        let result = (|| {
            for entry in walkdir::WalkDir::new(root)
                .into_iter()
                .filter_entry(|entry| {
                    entry.path() != archive
                        && !matches!(
                            entry.file_name().to_str(),
                            Some("dist" | ".dist-staging" | ".dist-backup" | "target")
                        )
                })
            {
                let entry = entry.map_err(|error| error.io_error().unwrap().kind())?;
                if entry.file_type().is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "md")
                {
                    old.push(entry.into_path());
                }
            }
            old.sort();
            Ok(old)
        })();
        assert_eq!(actual, result, "walker differential at {}", root.display());
    }
    assert_eq!(actual, expected);
}

#[test]
fn filters_extensions_archives_and_generated_basenames_and_sorts() {
    let f = Fixture::new();
    for name in [
        "z.md",
        "a/b.md",
        "a/a.md",
        "a.MD",
        "b.md.txt",
        "md",
        "upstream/old.md",
        "nested/upstream/kept.md",
        "dist/no.md",
        ".dist-staging/no.md",
        ".dist-backup/no.md",
        "target/no.md",
        "nested/target/no.md",
    ] {
        f.file(name);
    }
    for name in ["dist", ".dist-staging", ".dist-backup", "target"] {
        f.file(&format!("files/{name}"));
    }
    let expected = ["a/a.md", "a/b.md", "nested/upstream/kept.md", "z.md"]
        .map(|name| f.0.join(name))
        .to_vec();
    check(&f.0, &f.0.join("upstream"), Ok(expected));
    for name in [
        "dist",
        ".dist-staging",
        ".dist-backup",
        "target",
        "files/dist",
        "files/.dist-staging",
        "files/.dist-backup",
        "files/target",
    ] {
        check(&f.0.join(name), &f.0.join("upstream"), Ok(vec![]));
    }
    check(&f.0.join("upstream"), &f.0.join("upstream"), Ok(vec![]));
}

#[test]
fn files_missing_roots_and_non_directory_paths() {
    let f = Fixture::new();
    f.file("file.md");
    f.file("file.txt");
    check(
        &f.0.join("file.md"),
        &f.0.join("upstream"),
        Ok(vec![f.0.join("file.md")]),
    );
    check(&f.0.join("file.txt"), &f.0.join("upstream"), Ok(vec![]));
    check(
        &f.0.join("missing"),
        &f.0.join("upstream"),
        Err(io::ErrorKind::NotFound),
    );
    check(
        &f.0.join("file.md/child"),
        &f.0.join("upstream"),
        Err(io::ErrorKind::NotADirectory),
    );
}

#[cfg(unix)]
#[test]
fn symlink_roots_descendants_broken_links_and_loops() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    f.file("docs/a.md");
    symlink("docs", f.0.join("alias")).unwrap();
    symlink("a.md", f.0.join("docs/link.md")).unwrap();
    symlink("missing", f.0.join("docs/broken.md")).unwrap();
    symlink(".", f.0.join("docs/loop")).unwrap();
    symlink("docs/a.md", f.0.join("file.md")).unwrap();
    symlink("missing", f.0.join("broken")).unwrap();
    symlink("cycle", f.0.join("cycle")).unwrap();
    check(
        &f.0.join("alias"),
        &f.0.join("upstream"),
        Ok(vec![f.0.join("alias/a.md")]),
    );
    check(
        &f.0.join("docs"),
        &f.0.join("upstream"),
        Ok(vec![f.0.join("docs/a.md")]),
    );
    check(&f.0.join("file.md"), &f.0.join("upstream"), Ok(vec![]));
    check(
        &f.0.join("broken"),
        &f.0.join("upstream"),
        Err(io::ErrorKind::NotFound),
    );
    let error = fs::metadata(f.0.join("cycle")).unwrap_err().kind();
    check(&f.0.join("cycle"), &f.0.join("upstream"), Err(error));
    symlink("missing", f.0.join("target")).unwrap();
    check(
        &f.0.join("target"),
        &f.0.join("upstream"),
        Err(io::ErrorKind::NotFound),
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_names_and_permission_errors() {
    use std::os::unix::{ffi::OsStringExt, fs::PermissionsExt};
    let f = Fixture::new();
    let path = f.0.join(std::ffi::OsString::from_vec(b"\xff.md".to_vec()));
    fs::write(&path, "").unwrap();
    check(&f.0, &f.0.join("upstream"), Ok(vec![path]));
    let directory = f.0.join("private");
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o0)).unwrap();
    // Root can bypass mode bits. Run this test binary as an unprivileged user
    // too, to exercise the error rather than silently claiming coverage.
    if fs::read_dir(&directory).is_err() {
        check(
            &directory,
            &f.0.join("upstream"),
            Err(io::ErrorKind::PermissionDenied),
        );
        check(
            &f.0,
            &f.0.join("upstream"),
            Err(io::ErrorKind::PermissionDenied),
        );
    }
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn deep_directories_do_not_retain_open_handles() {
    let f = Fixture::new();
    let mut directory = f.0.clone();
    for _ in 0..150 {
        directory.push("d");
        fs::create_dir(&directory).unwrap();
    }
    let path = directory.join("deep.md");
    fs::write(&path, "").unwrap();
    check(&f.0, &f.0.join("upstream"), Ok(vec![path]));
}

#[cfg(walker_oracle)]
#[test]
fn maintained_repository_documentation_matches_retired_walker() {
    let root = Path::new(file!())
        .parent()
        .unwrap()
        .join("../../..")
        .canonicalize()
        .unwrap();
    let docs = root.join("docs");
    let archive = docs.join("upstream");
    let expected = document_scan::scan(&docs, &archive).unwrap();
    assert!(!expected.is_empty());
    assert!(expected.iter().all(|path| !path.starts_with(&archive)));
    check(&docs, &archive, Ok(expected));
}
