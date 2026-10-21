use super::*;
#[cfg(unix)]
#[test]
fn open_existing_never_creates_or_repairs_a_directory() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("private");
    assert!(PrivateDirectory::open_existing(&path).is_err());
    assert!(!path.exists());
    PrivateDirectory::create(&path).unwrap();
    let held = PrivateDirectory::open_existing(&path).unwrap();
    let moved = temp.path().join("moved");
    fs::rename(&path, &moved).unwrap();
    symlink(&moved, &path).unwrap();
    assert!(PrivateDirectory::open_existing(&path).is_err());
    fs::remove_file(&path).unwrap();
    PrivateDirectory::create(&path).unwrap();
    let name = EntryName::new("entry").unwrap();
    AtomicFile::create(&held, &name, b"anchored").unwrap();
    assert_eq!(fs::read(moved.join("entry")).unwrap(), b"anchored");
    let child = held
        .create_child(&EntryName::new("child").unwrap())
        .unwrap();
    AtomicFile::create(&child, &name, b"child anchored").unwrap();
    assert_eq!(
        fs::read(moved.join("child/entry")).unwrap(),
        b"child anchored"
    );
    assert!(!path.join("entry").exists());
    assert!(!path.join("child").exists());
    fs::set_permissions(&moved, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        PrivateDirectory::open_existing(&moved),
        Err(Error::UnsafePermissions(_))
    ));
    assert_eq!(
        fs::metadata(&moved).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn rejects_escape_and_atomically_publishes() {
    assert!(RelativePath::new("../escape").is_err());
    let temp = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::create(temp.path().join("state")).unwrap();
    let name = RelativePath::new("current").unwrap();
    AtomicFile::replace(&root, &name, b"one").unwrap();
    AtomicFile::replace(&root, &name, b"two").unwrap();
    assert_eq!(fs::read(root.resolve(&name)).unwrap(), b"two");
}
#[cfg(unix)]
#[test]
fn inventory_rejects_symlinks_and_budgets() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a"), b"abc").unwrap();
    assert!(
        bounded_directory_inventory(
            temp.path(),
            InventoryLimits {
                max_entries: 0,
                max_total_bytes: 10
            }
        )
        .is_err()
    );
    symlink("a", temp.path().join("b")).unwrap();
    assert!(matches!(
        bounded_directory_inventory(
            temp.path(),
            InventoryLimits {
                max_entries: 10,
                max_total_bytes: 10
            }
        ),
        Err(Error::UnsafeFileType(_))
    ));
}

#[cfg(unix)]
#[test]
fn inventory_rejects_hardlinks_and_advisory_lock_is_exclusive() {
    let temp = tempfile::tempdir().unwrap();
    let state = PrivateDirectory::create(temp.path().join("state")).unwrap();
    let lock_name = RelativePath::new("instance.lock").unwrap();
    let first = AdvisoryLock::acquire(&state, &lock_name).unwrap();
    assert!(matches!(
        AdvisoryLock::acquire(&state, &lock_name),
        Err(Error::AlreadyLocked(_))
    ));
    drop(first);
    AdvisoryLock::acquire(&state, &lock_name).unwrap();

    let original = state.path().join("original");
    fs::write(&original, b"secret").unwrap();
    fs::hard_link(&original, state.path().join("alias")).unwrap();
    assert!(matches!(
        bounded_directory_inventory(
            state.path(),
            InventoryLimits {
                max_entries: 10,
                max_total_bytes: 1024,
            }
        ),
        Err(Error::MultipleLinks(_))
    ));
}
