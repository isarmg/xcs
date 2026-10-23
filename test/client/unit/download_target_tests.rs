use super::*;
use std::{
    fs,
    io::Write,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
};

#[test]
fn publication_uses_the_selected_directory_after_parent_rebinding() {
    let root = tempfile::tempdir().unwrap();
    let selected = root.path().join("selected");
    let original = root.path().join("original");
    fs::create_dir(&selected).unwrap();
    let (mut file, stage) = Target::open(&selected.join("file"))
        .unwrap()
        .stage()
        .unwrap();
    file.write_all(b"verified bytes").unwrap();
    file.sync_all().unwrap();
    fs::rename(&selected, &original).unwrap();
    fs::create_dir(&selected).unwrap();
    let replacement = selected.join(&stage.name);
    fs::write(&replacement, b"unverified bytes").unwrap();
    stage.publish().unwrap();
    assert_eq!(fs::read(original.join("file")).unwrap(), b"verified bytes");
    assert_eq!(fs::read(replacement).unwrap(), b"unverified bytes");
    assert!(!selected.join("file").exists());
    assert_eq!(fs::read_dir(original).unwrap().count(), 1);
}

#[test]
fn cancellation_cleans_up_through_the_held_parent_not_its_replacement() {
    let root = tempfile::tempdir().unwrap();
    let selected = root.path().join("selected");
    let original = root.path().join("original");
    fs::create_dir(&selected).unwrap();
    let (file, stage) = Target::open(&selected.join("file"))
        .unwrap()
        .stage()
        .unwrap();
    fs::rename(&selected, &original).unwrap();
    fs::create_dir(&selected).unwrap();
    let replacement = selected.join(&stage.name);
    fs::write(&replacement, b"do not remove").unwrap();
    drop(file);
    drop(stage);
    assert_eq!(fs::read(replacement).unwrap(), b"do not remove");
    assert_eq!(fs::read_dir(original).unwrap().count(), 0);
}

#[test]
fn staging_directory_and_payload_are_private_and_single_linked() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("file");
    let (file, stage) = Target::open(&target).unwrap().stage().unwrap();
    assert_eq!(
        stage.directory.metadata().unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(file.metadata().unwrap().nlink(), 1);
    assert!(!target.exists());
    drop(file);
    drop(stage);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn existing_or_concurrent_symlink_destinations_are_not_followed_or_replaced() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("file");
    symlink("missing", &target).unwrap();
    assert_eq!(
        Target::open(&target).err().unwrap().kind(),
        io::ErrorKind::AlreadyExists
    );
    fs::remove_file(&target).unwrap();
    let (file, stage) = Target::open(&target).unwrap().stage().unwrap();
    symlink("missing", &target).unwrap();
    assert_eq!(
        stage.publish().unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    drop(file);
    assert_eq!(fs::read_link(target).unwrap(), Path::new("missing"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn staging_directory_replacement_is_not_removed_on_cancellation() {
    let root = tempfile::tempdir().unwrap();
    let (file, stage) = Target::open(&root.path().join("file"))
        .unwrap()
        .stage()
        .unwrap();
    let name = root.path().join(&stage.name);
    let renamed = root.path().join("renamed-stage");
    fs::rename(&name, &renamed).unwrap();
    fs::create_dir(&name).unwrap();
    fs::write(name.join("keep"), b"foreign entry").unwrap();
    drop(file);
    drop(stage);
    assert_eq!(fs::read(name.join("keep")).unwrap(), b"foreign entry");
    assert_eq!(fs::read_dir(renamed).unwrap().count(), 0);
}

#[test]
fn directory_spellings_are_not_reinterpreted_as_child_filenames() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("directory");
    fs::create_dir(&directory).unwrap();
    for path in [
        directory.join("."),
        Path::new(&format!("{}/", directory.display())).to_owned(),
    ] {
        assert_eq!(
            Target::open(&path).err().unwrap().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn replacing_payload_name_cannot_publish_unverified_bytes() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("file");
    let (mut file, stage) = Target::open(&target).unwrap().stage().unwrap();
    file.write_all(b"verified bytes").unwrap();
    file.sync_all().unwrap();
    let staging = root.path().join(&stage.name);
    fs::rename(staging.join("payload"), staging.join("original")).unwrap();
    fs::write(staging.join("payload"), b"unverified replacement").unwrap();
    stage.publish().unwrap();
    assert_eq!(fs::read(target).unwrap(), b"verified bytes");
    assert_eq!(
        fs::read(staging.join("payload")).unwrap(),
        b"unverified replacement"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn unlinking_payload_fails_closed_and_preserves_the_replacement() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("file");
    let (mut file, stage) = Target::open(&target).unwrap().stage().unwrap();
    file.write_all(b"verified bytes").unwrap();
    file.sync_all().unwrap();
    let staging = root.path().join(&stage.name);
    fs::remove_file(staging.join("payload")).unwrap();
    fs::write(staging.join("payload"), b"unverified replacement").unwrap();
    assert!(stage.publish().is_err());
    assert!(!target.exists());
    assert_eq!(
        fs::read(staging.join("payload")).unwrap(),
        b"unverified replacement"
    );
}

#[test]
fn cancelling_preserves_a_replacement_payload_entry() {
    let root = tempfile::tempdir().unwrap();
    let (file, stage) = Target::open(&root.path().join("file"))
        .unwrap()
        .stage()
        .unwrap();
    let staging = root.path().join(&stage.name);
    fs::remove_file(staging.join("payload")).unwrap();
    fs::write(staging.join("payload"), b"do not remove").unwrap();
    drop(file);
    drop(stage);
    assert_eq!(fs::read(staging.join("payload")).unwrap(), b"do not remove");
}

#[test]
fn group_inheriting_output_directory_still_supports_private_staging() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o2700)).unwrap();
    let target = root.path().join("file");
    let (file, stage) = Target::open(&target).unwrap().stage().unwrap();
    assert_eq!(
        stage.directory.metadata().unwrap().permissions().mode() & 0o777,
        0o700
    );
    stage.publish().unwrap();
    drop(file);
    assert_eq!(
        fs::metadata(target).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
