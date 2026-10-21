use super::*;
use std::{fs, io::Cursor, os::unix::fs::MetadataExt};

#[test]
fn independent_complete_files_have_matching_content_and_distinct_inodes() {
    let temp = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::create(temp.path().join("payloads")).unwrap();
    let content = vec![42u8; 130_001];
    let expectation = Expectation {
        bytes: content.len() as u64,
        blake3: Some(blake3::hash(&content)),
    };
    let first = write_full(
        &root,
        &EntryName::new("one").unwrap(),
        &mut Cursor::new(&content),
        expectation,
    )
    .unwrap();
    let second = write_full(
        &root,
        &EntryName::new("two").unwrap(),
        &mut Cursor::new(&content),
        expectation,
    )
    .unwrap();
    assert_eq!(first.blake3, second.blake3);
    assert_ne!(first.identity.inode, second.identity.inode);
    for name in ["one", "two"] {
        let path = root.path().join(name);
        assert_eq!(fs::metadata(&path).unwrap().nlink(), 1);
        let mut file = File::open(path).unwrap();
        assert_eq!(
            verify_full(&mut file, expectation).unwrap().blake3,
            first.blake3
        );
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn bad_length_or_digest_leaves_no_published_or_staging_file() {
    let temp = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::create(temp.path().join("payloads")).unwrap();
    let name = EntryName::new("immutable").unwrap();
    for expectation in [
        Expectation {
            bytes: 2,
            blake3: None,
        },
        Expectation {
            bytes: 4,
            blake3: None,
        },
        Expectation {
            bytes: 3,
            blake3: Some(blake3::hash(b"bad")),
        },
    ] {
        assert!(write_full(&root, &name, &mut Cursor::new(b"abc"), expectation).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[test]
fn publishing_never_clobbers_existing_payload_or_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::create(temp.path().join("payloads")).unwrap();
    let name = EntryName::new("immutable").unwrap();
    let expectation = Expectation {
        bytes: 3,
        blake3: None,
    };
    write_full(&root, &name, &mut Cursor::new(b"old"), expectation).unwrap();
    assert!(write_full(&root, &name, &mut Cursor::new(b"new"), expectation).is_err());
    assert_eq!(fs::read(root.path().join("immutable")).unwrap(), b"old");
    std::os::unix::fs::symlink("immutable", root.path().join("link")).unwrap();
    assert!(
        write_full(
            &root,
            &EntryName::new("link").unwrap(),
            &mut Cursor::new(b"new"),
            expectation
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}
