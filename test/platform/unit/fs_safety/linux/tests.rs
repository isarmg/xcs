use super::*;
use std::{
    fs,
    os::unix::{fs::symlink, net::UnixListener},
};
#[test]
fn rejects_root_and_intermediate_links_and_all_non_regular_files() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("file"), b"original").unwrap();
    fs::create_dir(temp.path().join("dir")).unwrap();
    symlink(temp.path(), temp.path().join("alias")).unwrap();
    assert!(OpenAt2Root::open(&temp.path().join("alias"), MountPolicy::SameMount).is_err());
    let root = OpenAt2Root::open(temp.path(), MountPolicy::SameMount).unwrap();
    symlink("file", temp.path().join("link")).unwrap();
    fs::hard_link(temp.path().join("file"), temp.path().join("hard")).unwrap();
    rustix::fs::mknodat(
        root.directory(),
        "fifo",
        FileType::Fifo,
        Mode::from_raw_mode(0o600),
        0,
    )
    .unwrap();
    let _socket = UnixListener::bind(temp.path().join("socket")).unwrap();
    for name in [
        "dir",
        "link",
        "hard",
        "file",
        "alias/file",
        "fifo",
        "socket",
    ] {
        assert!(
            root.open_file(&RelativePath::new(name).unwrap(), false)
                .is_err(),
            "{name}"
        );
    }
    assert_eq!(fs::read(temp.path().join("file")).unwrap(), b"original");
}
#[test]
fn root_identity_and_exclusive_lock_survive_directory_rebinding() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("root");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("file"), b"old").unwrap();
    let root = OpenAt2Root::open(&path, MountPolicy::AllowNestedMounts).unwrap();
    let lock = AdvisoryLock::directory(root.directory()).unwrap();
    let second = OpenAt2Root::open(&path, MountPolicy::AllowNestedMounts).unwrap();
    assert!(AdvisoryLock::directory(second.directory()).is_err());
    let moved = temp.path().join("moved");
    fs::rename(&path, &moved).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("file"), b"new").unwrap();
    assert_eq!(
        root.identity(),
        FileIdentity::from_metadata(&fs::metadata(moved).unwrap())
    );
    let mut old = root
        .open_file(&RelativePath::new("file").unwrap(), false)
        .unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut old, &mut bytes).unwrap();
    assert_eq!(bytes, b"old");
    drop(lock);
    drop(root);
    AdvisoryLock::directory(second.directory()).unwrap();
}
