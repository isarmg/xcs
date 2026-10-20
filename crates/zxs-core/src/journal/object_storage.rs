//! Hash-prefix object layout. Unsupported flat objects fail closed; readers
//! never migrate, delete, reuse, or accept them as current-format evidence.
use super::*;
use std::{collections::BTreeSet, fs};

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn validate(kind: &str, hash: &str) -> Result<()> {
    if kind.is_empty()
        || !kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
        || !valid_hash(hash)
    {
        return Err(Error::new(
            "OBJECT_CORRUPT",
            "invalid immutable object address",
        ));
    }
    Ok(())
}
fn flat_path(d: &DiskConfig, kind: &str, hash: &str) -> PathBuf {
    d.root
        .join("metadata/objects")
        .join(kind)
        .join(format!("{hash}.json"))
}
pub(crate) fn object_path(d: &DiskConfig, kind: &str, hash: &str) -> PathBuf {
    // All I/O entry points validate before using this pure path constructor.
    d.root
        .join("metadata/objects")
        .join(kind)
        .join(hash.get(..2).unwrap_or(hash))
        .join(format!("{hash}.json"))
}
fn directory_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            existing_private_dir(path)?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn kind_directory(d: &DiskConfig, kind: &str) -> Result<Option<PathBuf>> {
    let mut path = d.root.clone();
    if !directory_exists(&path)? {
        return Ok(None);
    }
    for component in ["metadata", "objects", kind] {
        path.push(component);
        if !directory_exists(&path)? {
            return Ok(None);
        }
    }
    Ok(Some(path))
}
fn add_existing(path: PathBuf, out: &mut Vec<PathBuf>) -> Result<()> {
    match fs::symlink_metadata(&path) {
        Ok(_) => {
            open_regular(&path)?;
            out.push(path);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
pub(crate) fn existing_object_paths(
    d: &DiskConfig,
    kind: &str,
    hash: &str,
) -> Result<Vec<PathBuf>> {
    validate(kind, hash)?;
    let mut paths = Vec::new();
    if let Some(directory) = kind_directory(d, kind)? {
        // Reject old-format evidence rather than silently creating a second
        // representation or treating an unsupported object as absent.
        match fs::symlink_metadata(flat_path(d, kind, hash)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
            Ok(_) => {
                return Err(Error::new(
                    "FORMAT_UNSUPPORTED",
                    "flat metadata object layout is unsupported",
                ));
            }
        }
        if directory_exists(&directory.join(&hash[..2]))? {
            add_existing(object_path(d, kind, hash), &mut paths)?;
        }
    }
    Ok(paths)
}
pub(super) fn read_object_bytes(d: &DiskConfig, kind: &str, hash: &str) -> Result<Vec<u8>> {
    let paths = existing_object_paths(d, kind, hash)?;
    let path = paths
        .first()
        .ok_or_else(|| Error::new("OBJECT_CORRUPT", "immutable object is missing"))?;
    read_bytes(path, MAX_METADATA)
}
pub(super) fn write_object_bytes(
    d: &DiskConfig,
    kind: &str,
    hash: &str,
    bytes: &[u8],
) -> Result<()> {
    let paths = existing_object_paths(d, kind, hash)?;
    for path in &paths {
        if read_bytes(path, MAX_METADATA)? != bytes {
            return Err(Error::new(
                "OBJECT_COLLISION",
                "immutable object differs from existing bytes",
            ));
        }
    }
    // New objects are installed only at their current hash-prefix address.
    if paths.is_empty() {
        write_atomic(&object_path(d, kind, hash), bytes, false)?;
    }
    Ok(())
}
/// Inventory both layouts, rejecting unexpected entries rather than claiming
/// absence when unreadable evidence could contain a publication.
pub(crate) fn list_object_hashes(d: &DiskConfig, kind: &str) -> Result<BTreeSet<String>> {
    validate(kind, &"0".repeat(64))?;
    let mut hashes = BTreeSet::new();
    let Some(directory) = kind_directory(d, kind)? else {
        return Ok(hashes);
    };
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::new("OBJECT_CORRUPT", "invalid object name"))?;
        if name.len() == 2
            && name
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            existing_private_dir(&entry.path())?;
            for child in fs::read_dir(entry.path())? {
                let child = child?;
                let hash = inventory_hash(&child.path())?;
                if !hash.starts_with(&name) {
                    return Err(Error::new("OBJECT_CORRUPT", "object is in the wrong shard"));
                }
                hashes.insert(hash);
            }
        } else {
            return Err(Error::new(
                "FORMAT_UNSUPPORTED",
                "expected a metadata object hash-prefix directory",
            ));
        }
    }
    Ok(hashes)
}
fn inventory_hash(path: &Path) -> Result<String> {
    let hash = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".json"))
        .filter(|hash| valid_hash(hash))
        .ok_or_else(|| Error::new("OBJECT_CORRUPT", "invalid object filename"))?;
    open_regular(path)?;
    Ok(hash.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn config(temp: &tempfile::TempDir) -> Config {
        Config::development([
            temp.path().join("a"),
            temp.path().join("b"),
            temp.path().join("c"),
        ])
    }
    #[test]
    fn current_shards_preserve_hashes_and_reject_conflicting_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let config = config(&temp);
        let disk = &config.disks[0];
        let value = json!({"retained": "历史"});
        let hash = store_object(disk, "publication", &value).unwrap();
        let path = object_path(disk, "publication", &hash);
        assert_eq!(fs::read(&path).unwrap(), canonical_json(&value).unwrap());
        assert_eq!(
            load_object::<Value>(disk, "publication", &hash).unwrap(),
            value
        );
        assert_eq!(store_object(disk, "publication", &value).unwrap(), hash);
        assert_eq!(
            list_object_hashes(disk, "publication").unwrap(),
            BTreeSet::from([hash.clone()])
        );
        fs::write(&path, b"broken").unwrap();
        assert!(load_object::<Value>(disk, "publication", &hash).is_err());
        assert!(store_object(disk, "publication", &value).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
        for invalid in ["../outside", "é", &"A".repeat(64)] {
            assert!(load_object::<Value>(disk, "publication", invalid).is_err());
        }
        assert!(load_object::<Value>(disk, "../outside", &hash).is_err());
    }

    #[test]
    fn flat_objects_are_rejected_without_migration_or_deletion() {
        for sharded_copy in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let config = config(&temp);
            let disk = &config.disks[0];
            let value = json!({"retained": "old format"});
            let hash = object_hash("publication", &value).unwrap();
            if sharded_copy {
                store_object(disk, "publication", &value).unwrap();
            }
            let flat = flat_path(disk, "publication", &hash);
            write_json(&flat, &value, false).unwrap();
            let bytes = fs::read(&flat).unwrap();
            assert_eq!(
                load_object::<Value>(disk, "publication", &hash)
                    .unwrap_err()
                    .code(),
                "FORMAT_UNSUPPORTED"
            );
            assert_eq!(
                store_object(disk, "publication", &value)
                    .unwrap_err()
                    .code(),
                "FORMAT_UNSUPPORTED"
            );
            assert_eq!(
                list_object_hashes(disk, "publication").unwrap_err().code(),
                "FORMAT_UNSUPPORTED"
            );
            assert_eq!(fs::read(flat).unwrap(), bytes);
            assert_eq!(
                object_path(disk, "publication", &hash).exists(),
                sharded_copy
            );
        }
    }
    #[test]
    fn inventory_rejects_wrong_shards_and_symlink_fallbacks() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let config = config(&temp);
        let disk = &config.disks[0];
        let value = json!({"v": 1});
        let hash = store_object(disk, "publication", &value).unwrap();
        let path = object_path(disk, "publication", &hash);
        let flat = flat_path(disk, "publication", &hash);
        symlink(&path, &flat).unwrap();
        assert!(load_object::<Value>(disk, "publication", &hash).is_err());
        assert!(store_object(disk, "publication", &value).is_err());
        assert!(list_object_hashes(disk, "publication").is_err());
        fs::remove_file(&flat).unwrap();
        write_json(&flat, &value, false).unwrap();
        fs::remove_file(&path).unwrap();
        fs::remove_dir(path.parent().unwrap()).unwrap();
        symlink(temp.path().join("missing"), path.parent().unwrap()).unwrap();
        assert!(load_object::<Value>(disk, "publication", &hash).is_err());
        assert!(store_object(disk, "publication", &value).is_err());
        assert!(list_object_hashes(disk, "publication").is_err());
        fs::remove_file(path.parent().unwrap()).unwrap();
        let wrong = if hash.starts_with("00") { "01" } else { "00" };
        let wrong_path = flat
            .parent()
            .unwrap()
            .join(wrong)
            .join(format!("{hash}.json"));
        write_json(&wrong_path, &value, false).unwrap();
        assert!(list_object_hashes(disk, "publication").is_err());
    }
}
