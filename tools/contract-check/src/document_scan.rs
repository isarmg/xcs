//! Maintained Markdown discovery. Only the root may traverse a directory symlink.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn excluded(path: &Path, archive: &Path) -> bool {
    path == archive
        || matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("dist" | ".dist-staging" | ".dist-backup" | "target")
        )
}

/// Discover regular `.md` files, excluding archived and generated documentation.
/// A symlink root is inspected and, if a directory, traversed, but never emitted
/// as a file. Descendant symlinks (including broken links and loops) are ignored.
/// Directory handles are consumed one at a time; depth cannot exhaust handles
/// or the call stack. Errors are returned, never treated as missing documents.
pub fn scan(root: &Path, archive: &Path) -> io::Result<Vec<PathBuf>> {
    let kind = fs::symlink_metadata(root)?.file_type();
    let directory = if kind.is_symlink() {
        fs::metadata(root)?.is_dir()
    } else {
        kind.is_dir()
    };
    let mut paths = Vec::new();
    if excluded(root, archive) {
        return Ok(paths);
    }
    if kind.is_file() && root.extension().is_some_and(|extension| extension == "md") {
        paths.push(root.to_path_buf());
    }
    let mut directories = Vec::new();
    if directory {
        directories.push(root.to_path_buf());
    }
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let path = entry.path();
            if excluded(&path, archive) {
                continue;
            }
            if kind.is_dir() {
                directories.push(path);
            } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "md")
            {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}
