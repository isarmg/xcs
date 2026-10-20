//! Owned descriptor snapshots and close/save ordering.
use super::{Open, bad, context, sdk};
use smb_protocol::{client::Client, protocol::OpenResult};
use std::{
    os::fd::{FromRawFd, IntoRawFd, OwnedFd},
    sync::{Arc, Mutex},
};

pub(super) struct Snapshot {
    pub(super) client: Client,
    pub(super) info: OpenResult,
    pub(super) fd: OwnedFd,
    pub(super) writable: bool,
    pub(super) io: Arc<Mutex<()>>,
}
/// # Safety
/// handle obeys the module host contract and its Context is exclusively accessed
/// for this call. fsp is only used as an opaque lookup key, never dereferenced.
pub(super) unsafe fn snapshot(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
) -> std::io::Result<Snapshot> {
    // SAFETY: The owning host callback supplies a live handle; Context access stays on this event-
    // loop thread and does not escape the callback.
    let context = unsafe { context(handle) }?;
    let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
    // SAFETY: Open owns a live descriptor; F_DUPFD_CLOEXEC returns a distinct owned descriptor or
    // an error, checked below.
    let descriptor = unsafe { libc::fcntl(opened.fd, libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(Snapshot {
        client: context.client.clone(),
        info: opened.info.clone(),
        // SAFETY: fcntl succeeded above; this new descriptor has not been adopted by any other
        // owner.
        fd: unsafe { OwnedFd::from_raw_fd(descriptor) },
        writable: opened.writable,
        io: opened.io.clone(),
    })
}
pub(super) fn close_opened(
    opened: Open,
    save: impl FnOnce(u64) -> std::io::Result<()>,
) -> std::io::Result<i32> {
    // SAFETY: removing Open from Context transfers its sole local descriptor
    // ownership here. Guard it before any fallible lock/save operation, because
    // it is no longer available to free_context on an early return or unwind.
    let descriptor = unsafe { OwnedFd::from_raw_fd(opened.fd) };
    let _guard = opened
        .io
        .lock()
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
    let saved = save(opened.info.handle);
    // Preserve the callback's existing close-error reporting on the normal path.
    // SAFETY: into_raw_fd transfers the sole local ownership to this close call; no owner will
    // close this descriptor again.
    let closed = unsafe { libc::close(descriptor.into_raw_fd()) };
    saved?;
    if closed < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(0)
}
