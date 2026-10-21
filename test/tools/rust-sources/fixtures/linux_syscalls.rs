//! Linux amd64 ABI and syscall smoke tests for both maintained rustix backends.
#![deny(warnings)]

use std::mem::{align_of, offset_of, size_of};

#[test]
fn lp64_public_layout_matches_libc() {
    assert_eq!(size_of::<usize>(), 8);
    assert_eq!(size_of::<libc::c_long>(), 8);
    assert_eq!(
        size_of::<rustix::time::Timespec>(),
        size_of::<libc::timespec>()
    );
    assert_eq!(
        align_of::<rustix::time::Timespec>(),
        align_of::<libc::timespec>()
    );
    assert_eq!(
        offset_of!(rustix::time::Timespec, tv_nsec),
        offset_of!(libc::timespec, tv_nsec)
    );
    assert_eq!(size_of::<rustix::fs::Stat>(), size_of::<libc::stat>());
    assert_eq!(align_of::<rustix::fs::Stat>(), align_of::<libc::stat>());
    assert_eq!(
        offset_of!(rustix::fs::Stat, st_size),
        offset_of!(libc::stat, st_size)
    );
    assert_eq!(
        offset_of!(rustix::fs::Stat, st_mtime),
        offset_of!(libc::stat, st_mtime)
    );
    assert_eq!(size_of::<rustix::fs::StatFs>(), size_of::<libc::statfs>());
    assert_eq!(align_of::<rustix::fs::StatFs>(), align_of::<libc::statfs>());
    assert_eq!(
        offset_of!(rustix::fs::StatFs, f_bsize),
        offset_of!(libc::statfs, f_bsize)
    );
}

#[test]
fn process_and_clock_observation_is_consistent() {
    assert_eq!(
        rustix::process::getpid().as_raw_nonzero().get() as u32,
        std::process::id()
    );
    let before = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    let after = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    assert!((0..1_000_000_000).contains(&before.tv_nsec));
    assert!(after >= before);
    assert!(rustix::param::page_size().is_power_of_two());
}

#[test]
fn pipe_round_trip_and_eof_preserve_descriptor_semantics() {
    let (reader, writer) = rustix::pipe::pipe().unwrap();
    let payload = b"linux-amd64 syscall boundary";
    assert_eq!(rustix::io::write(&writer, payload).unwrap(), payload.len());
    drop(writer);
    let mut output = [0u8; 128];
    let count = rustix::io::read(&reader, &mut output).unwrap();
    assert_eq!(&output[..count], payload);
    assert_eq!(rustix::io::read(&reader, &mut output).unwrap(), 0);
}

#[test]
fn filesystem_stat_and_error_mapping_match_std() {
    let current = std::env::current_dir().unwrap();
    let fd = rustix::fs::openat(
        rustix::fs::CWD,
        &current,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
        rustix::fs::Mode::empty(),
    )
    .unwrap();
    let stat = rustix::fs::fstat(&fd).unwrap();
    assert!(rustix::fs::FileType::from_raw_mode(stat.st_mode).is_dir());
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(&current).unwrap();
    assert_eq!(stat.st_ino, metadata.ino());
    assert_eq!(stat.st_dev, metadata.dev());
    let error = rustix::fs::openat(
        &fd,
        "",
        rustix::fs::OFlags::RDONLY,
        rustix::fs::Mode::empty(),
    )
    .unwrap_err();
    assert_eq!(error, rustix::io::Errno::NOENT);
    assert_eq!(error.raw_os_error(), libc::ENOENT);
}
