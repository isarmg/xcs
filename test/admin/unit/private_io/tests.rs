use super::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    sync::Arc,
};

#[test]
fn owned_draft_reads_are_bounded_and_reject_links_without_requiring_secret_mode() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("draft.json");
    fs::write(&path, b"draft").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(read_owned_bytes(&path, 5).unwrap(), b"draft");
    assert!(read_owned_bytes(&path, 4).is_err());
    symlink(&path, root.path().join("alias")).unwrap();
    assert!(read_owned_bytes(&root.path().join("alias"), 5).is_err());
    fs::hard_link(&path, root.path().join("hard")).unwrap();
    assert!(read_owned_bytes(&path, 5).is_err());
    assert!(read_owned_bytes(root.path(), 5).is_err());
}

#[test]
fn private_reads_validate_the_opened_file_and_bound_its_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("credential");
    write_new(&path, b"fixture-private-value").unwrap();
    assert_eq!(read_private(&path, 21).unwrap(), "fixture-private-value");
    assert!(read_private(&path, 20).is_err());
    for mode in [0o644, 0o640, 0o700, 0o2600] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        assert!(read_private(&path, 1024).is_err(), "{mode:o}");
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(read_private(&path, 1024).is_ok());
    symlink(&path, directory.path().join("alias")).unwrap();
    assert!(read_private(&directory.path().join("alias"), 1024).is_err());
    fs::hard_link(&path, directory.path().join("hard")).unwrap();
    assert!(read_private(&path, 1024).is_err());
    assert!(read_private(directory.path(), 1024).is_err());
}

#[test]
fn held_parent_is_not_reopened_after_directory_rebinding() {
    let temporary = tempfile::tempdir().unwrap();
    let parent = temporary.path().join("parent");
    fs::create_dir(&parent).unwrap();
    write_new(&parent.join("credential"), b"original").unwrap();
    let held = AnchoredPath::open(&parent.join("credential")).unwrap();
    let moved = temporary.path().join("moved");
    fs::rename(&parent, &moved).unwrap();
    fs::create_dir(&parent).unwrap();
    write_new(&parent.join("credential"), b"replacement").unwrap();
    assert_eq!(held.read_private(1024).unwrap(), "original");
}

#[test]
fn atomic_create_is_private_no_clobber_and_cleans_failed_staging() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("published");
    write_new(&path, b"complete").unwrap();
    let metadata = fs::metadata(&path).unwrap();
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.nlink(), 1);
    assert!(write_new(&path, b"overwrite").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"complete");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    let alias = directory.path().join("alias");
    symlink(&path, &alias).unwrap();
    assert!(write_new(&alias, b"follow link").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"complete");
    let occupied = directory.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    assert!(write_new(&occupied, b"must not publish").is_err());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
}

#[test]
fn concurrent_private_publication_has_exactly_one_complete_winner() {
    let directory = tempfile::tempdir().unwrap();
    let path = Arc::new(directory.path().join("published"));
    let threads = [b"first".as_slice(), b"second".as_slice()]
        .into_iter()
        .map(|bytes| {
            let path = path.clone();
            std::thread::spawn(move || write_new(&path, bytes).is_ok())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|won| *won)
            .count(),
        1
    );
    let bytes = fs::read(&*path).unwrap();
    assert!(bytes == b"first" || bytes == b"second");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn published_staging_cleanup_preserves_a_reused_temporary_name() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = OpenAt2Root::open(temporary.path(), MountPolicy::AllowNestedMounts).unwrap();
    let name = "temporary";
    fs::write(temporary.path().join(name), b"published").unwrap();
    let mut cleanup = StagingCleanup {
        directory: directory.directory(),
        name: name.to_owned(),
        armed: true,
    };
    renameat(directory.directory(), name, directory.directory(), "final").unwrap();
    cleanup.armed = false;
    fs::write(temporary.path().join(name), b"another creator").unwrap();
    drop(cleanup);
    assert_eq!(
        fs::read(temporary.path().join(name)).unwrap(),
        b"another creator"
    );
    assert_eq!(
        fs::read(temporary.path().join("final")).unwrap(),
        b"published"
    );
}
#[test]
fn replacement_updates_only_owned_regular_files_without_following_links() {
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("draft");
    write_new(&original, b"before").unwrap();
    replace_private(&original, b"after").unwrap();
    assert_eq!(fs::read(&original).unwrap(), b"after");
    let alias = directory.path().join("alias");
    symlink(&original, &alias).unwrap();
    assert!(replace_private(&alias, b"must not follow").is_err());
    assert_eq!(fs::read(&original).unwrap(), b"after");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}
#[test]
fn administrator_cannot_accidentally_read_another_owners_secret() {
    if rustix::process::geteuid().as_raw() != 0 {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreign");
    write_new(&path, b"fixture-value").unwrap();
    let file = File::open(&path).unwrap();
    rustix::fs::fchown(&file, Some(rustix::process::Uid::from_raw(65534)), None).unwrap();
    assert!(read_private(&path, 1024).is_err());
}
