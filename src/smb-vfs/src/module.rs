//! All ABI calls are generated from the target SDK. No custom C shim.
//!
//! # Host boundary contract
//! Samba must call this table on its owning event-loop thread, with live,
//! correctly aligned SDK objects and NUL-terminated strings. Input buffers must
//! be readable and output buffers exclusively writable for the supplied length.
//! Async output buffers remain live until completion or cancellation; cancellation
//! frees the request on that same event loop before releasing the buffer.
//! Handles store only Context boxes installed by connect; directory pointers are
//! opaque Directory boxes returned by fdopendir and freed exactly once by closedir.
//! Null checks defend optional arguments, not dangling pointers or a mismatched ABI.
//! Workers receive owned descriptors/data, never these host pointers.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
use serde_json::Value;
use smb_protocol::{
    client::{Client, error_errno},
    protocol::{OpenResult, Request},
};
use std::{
    collections::BTreeMap,
    ffi::c_void,
    os::fd::{AsRawFd, IntoRawFd},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

mod sdk {
    // Generated C names/bitfield helpers preserve the pinned SDK ABI. Keep these
    // exceptions inside the include module, never on hand-written callbacks.
    #![allow(
        dead_code,
        non_camel_case_types,
        non_snake_case,
        non_upper_case_globals,
        unnecessary_transmutes,
        clippy::undocumented_unsafe_blocks,
        clippy::ptr_offset_with_cast,
        clippy::upper_case_acronyms,
        clippy::transmute_int_to_bool,
        clippy::too_many_arguments,
        clippy::type_complexity
    )]
    include!(concat!(env!("OUT_DIR"), "/sdk.rs"));
}
#[path = "asynchronous.rs"]
mod asynchronous;
mod descriptors;
mod paths;
use descriptors::{close_opened, snapshot};
use paths::{at_name, name, text};

struct Open {
    info: OpenResult,
    fd: i32,
    writable: bool,
    io: Arc<Mutex<()>>,
}
struct Context {
    client: Client,
    files: BTreeMap<usize, Open>,
}
struct Directory {
    entries: Vec<sdk::dirent>,
    position: usize,
}

fn errno(error: i32) {
    // SAFETY: Linux provides a live thread-local errno slot; no reference to it escapes this
    // assignment.
    unsafe {
        *libc::__errno_location() = error;
    }
}
fn guarded(work: impl FnOnce() -> std::io::Result<i32>) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            eprintln!("zxs SMB: {error}");
            errno(error_errno(&error));
            -1
        }
        Err(_) => {
            errno(libc::EIO);
            -1
        }
    }
}
fn bad() -> std::io::Error {
    std::io::Error::from_raw_os_error(libc::EINVAL)
}
/// # Safety
/// For nonzero count, data must be live and readable for count bytes for 'a.
/// The caller must not mutate or free the storage while the returned slice lives.
unsafe fn input_bytes<'a>(data: *const c_void, count: usize) -> std::io::Result<&'a [u8]> {
    if count == 0 {
        return Ok(&[]);
    }
    if data.is_null() || count > isize::MAX as usize {
        return Err(bad());
    }
    // SAFETY: caller guarantees initialized, readable storage for the requested
    // lifetime; non-null and the slice size bound were checked above.
    Ok(unsafe { std::slice::from_raw_parts(data.cast(), count) })
}

/// # Safety
/// A non-null handle must be live. Its non-null data must be our Context box;
/// the caller guarantees exclusive access and that the box outlives 'a.
unsafe fn context<'a>(handle: *mut sdk::vfs_handle_struct) -> std::io::Result<&'a mut Context> {
    // SAFETY: The caller supplies a live SDK handle when non-null; short-circuiting rejects null
    // before dereference.
    if handle.is_null() || unsafe { (*handle).data }.is_null() {
        return Err(bad());
    }
    // SAFETY: connect installed a Context box in data. The caller guarantees exclusive event-loop
    // access for this borrow; neither handle nor data is null.
    Ok(unsafe { &mut *((*handle).data.cast::<Context>()) })
}
fn inode(id: &str) -> u64 {
    let bytes = blake3::hash(id.as_bytes());
    u64::from_le_bytes(bytes.as_bytes()[..8].try_into().unwrap()).max(1)
}

unsafe extern "C" fn free_context(data: *mut *mut c_void) {
    if data.is_null() {
        return;
    }
    // SAFETY: The host passes a writable data slot to its registered free callback; null was
    // rejected above.
    let pointer = unsafe { *data };
    if !pointer.is_null() {
        // SAFETY: connect created this exact Context with Box::into_raw; free_context consumes it
        // once and clears the host slot.
        let context = unsafe { Box::from_raw(pointer.cast::<Context>()) };
        for opened in context.files.values() {
            // SAFETY: Each Open owns its descriptor, transferred by into_raw_fd on open; consuming
            // Context closes each remaining descriptor once.
            unsafe {
                libc::close(opened.fd);
            }
        }
        // SAFETY: The host data slot remains writable during its free callback; clearing it
        // prevents a second Box reclamation.
        unsafe {
            *data = std::ptr::null_mut();
        }
    }
}
unsafe extern "C" fn connect(
    handle: *mut sdk::vfs_handle_struct,
    _service: *const libc::c_char,
    user: *const libc::c_char,
) -> i32 {
    guarded(|| {
        // SAFETY: A non-null handle is a live host object; short-circuit evaluation prevents
        // dereferencing a null handle.
        if handle.is_null() || unsafe { (*handle).conn }.is_null() {
            return Err(bad());
        }
        // SAFETY: The handle and connection were checked non-null and remain live on the owning
        // host thread.
        let parameters = unsafe { (*(*handle).conn).params };
        if parameters.is_null() {
            return Err(bad());
        }
        // SAFETY: The host parameter object is live for this callback and was checked non-null
        // above.
        let service = unsafe { (*parameters).service };
        // SAFETY: SDK parameter lookup uses static NUL-terminated keys and returns a borrowed NUL-
        // terminated string or null; text validates the latter.
        let socket = unsafe {
            text(sdk::lp_parm_const_string(
                service,
                c"zxs".as_ptr(),
                c"socket".as_ptr(),
                std::ptr::null(),
            ))
        }?;
        // SAFETY: SDK parameter lookup uses static NUL-terminated keys and returns a borrowed NUL-
        // terminated string or null; text validates the latter.
        let repository = unsafe {
            text(sdk::lp_parm_const_string(
                service,
                c"zxs".as_ptr(),
                c"repository".as_ptr(),
                std::ptr::null(),
            ))
        }?;
        // SAFETY: Samba supplies a NUL-terminated user string valid during connect; text rejects
        // null and invalid UTF-8.
        let client = Client::connect(&PathBuf::from(socket), unsafe { text(user) }?, repository)?;
        let boxed = Box::new(Context {
            client,
            files: BTreeMap::new(),
        });
        // SAFETY: The live handle is exclusively accessed on its host event loop; the new Box is
        // transferred to the matching free_context callback.
        unsafe {
            (*handle).data = Box::into_raw(boxed).cast();
            (*handle).free_data = Some(free_context);
        }
        Ok(0)
    })
}
unsafe extern "C" fn disconnect(handle: *mut sdk::vfs_handle_struct) {
    if !handle.is_null() {
        // SAFETY: The non-null live host handle provides its own writable data slot, which
        // free_context clears after taking ownership.
        unsafe {
            free_context(&mut (*handle).data);
        }
    }
}

unsafe extern "C" fn openat(
    handle: *mut sdk::vfs_handle_struct,
    directory: *const sdk::files_struct,
    file: *const sdk::smb_filename,
    fsp: *mut sdk::files_struct,
    how: *const sdk::vfs_open_how,
) -> i32 {
    guarded(|| {
        if fsp.is_null() || how.is_null() {
            return Err(bad());
        }
        // SAFETY: The host supplies a live configuration object; its pointer was checked non-null
        // before reading flags.
        let flags = unsafe { (*how).flags };
        let writable = flags & libc::O_ACCMODE != libc::O_RDONLY;
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let path = unsafe { at_name(handle, directory, file) }?;
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let context = unsafe { context(handle) }?;
        let (info, fd) = context.client.open(Request::Open {
            path,
            writable,
            create: flags & libc::O_CREAT != 0,
            truncate: flags & libc::O_TRUNC != 0,
            exclusive: flags & libc::O_EXCL != 0,
            write_through: flags & (libc::O_SYNC | libc::O_DSYNC) != 0,
        })?;
        let fd = fd.into_raw_fd();
        context.files.insert(
            fsp as usize,
            Open {
                info,
                fd,
                writable,
                io: Arc::new(Mutex::new(())),
            },
        );
        Ok(fd)
    })
}
unsafe extern "C" fn pread(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
    data: *mut c_void,
    count: usize,
    offset: sdk::off_t,
) -> isize {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> std::io::Result<isize> {
            // SAFETY: The owning host callback supplies a live handle; Context access stays on this
            // event-loop thread and does not escape the callback.
            let context = unsafe { context(handle) }?;
            let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
            if offset < 0 || count > isize::MAX as usize || (count > 0 && data.is_null()) {
                return Err(bad());
            }
            let _guard = opened
                .io
                .lock()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
            context.client.call(Request::AuthorizeRead {
                handle: opened.info.handle,
            })?;
            // SAFETY: The host guarantees data is writable for count bytes for this synchronous
            // call; the descriptor remains owned and the I/O lock is held.
            let result = unsafe { libc::pread(opened.fd, data, count, offset) };
            if result < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(result)
        },
    ));
    match result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            errno(error_errno(&error));
            -1
        }
        Err(_) => {
            errno(libc::EIO);
            -1
        }
    }
}
unsafe extern "C" fn pwrite(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
    data: *const c_void,
    count: usize,
    offset: sdk::off_t,
) -> isize {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> std::io::Result<isize> {
            // SAFETY: The owning host callback supplies a live handle; Context access stays on this
            // event-loop thread and does not escape the callback.
            let context = unsafe { context(handle) }?;
            let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
            let _guard = opened
                .io
                .lock()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
            write_data(
                &context.client,
                &opened.info,
                opened.fd,
                opened.writable,
                // SAFETY: the host provides count readable bytes for this callback;
                // the helper checks null/length and the slice is not retained.
                unsafe { input_bytes(data, count) }?,
                offset,
            )
        },
    ));
    match result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            eprintln!("zxs SMB write: {error}");
            errno(error_errno(&error));
            -1
        }
        Err(_) => {
            errno(libc::EIO);
            -1
        }
    }
}
fn write_data(
    client: &Client,
    info: &OpenResult,
    fd: i32,
    writable: bool,
    data: &[u8],
    offset: sdk::off_t,
) -> std::io::Result<isize> {
    let count = data.len();
    if !writable || count > isize::MAX as usize {
        return Err(std::io::Error::from_raw_os_error(libc::EACCES));
    }
    let offset = if offset == -1 {
        // SAFETY: libc::stat is a C POD output structure whose integer and pointer fields admit
        // zero initialization.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: The descriptor is owned and the output points to a live, exclusively borrowed
        // stat structure.
        if unsafe { libc::fstat(fd, &mut stat) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        stat.st_size
    } else {
        offset
    };
    if offset < 0
        || (offset as u64)
            .checked_add(count as u64)
            .is_none_or(|end| end > info.max_file_bytes)
    {
        return Err(std::io::Error::from_raw_os_error(libc::EFBIG));
    }
    // SAFETY: The borrowed slice supplies count initialized bytes and remains live throughout
    // pwrite; the caller serializes descriptor I/O.
    let result = unsafe { libc::pwrite(fd, data.as_ptr().cast(), count, offset) };
    if result < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if result > 0 {
        client.call(Request::Dirty {
            handle: info.handle,
        })?;
    }
    Ok(result)
}
unsafe extern "C" fn truncate(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
    length: sdk::off_t,
) -> i32 {
    guarded(|| {
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let context = unsafe { context(handle) }?;
        let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
        if !opened.writable || length < 0 || length as u64 > opened.info.max_file_bytes {
            return Err(bad());
        }
        let _guard = opened
            .io
            .lock()
            .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
        // SAFETY: The descriptor remains owned while the I/O lock is held; length has been
        // validated.
        if unsafe { libc::ftruncate(opened.fd, length) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        context.client.call(Request::Dirty {
            handle: opened.info.handle,
        })?;
        Ok(0)
    })
}
unsafe extern "C" fn pread_send(
    handle: *mut sdk::vfs_handle_struct,
    mem_ctx: *mut sdk::TALLOC_CTX,
    event: *mut sdk::tevent_context,
    fsp: *mut sdk::files_struct,
    data: *mut c_void,
    count: usize,
    offset: sdk::off_t,
) -> *mut sdk::tevent_req {
    let prepared = (|| -> std::io::Result<_> {
        if count > asynchronous::MAX_IO || offset < 0 || (count > 0 && data.is_null()) {
            return Err(bad());
        }
        // SAFETY: The host handle is live on its owning event loop; snapshot duplicates the FD and
        // returns no host pointers to workers.
        let opened = unsafe { snapshot(handle, fsp) }?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count)
            .map_err(|_| std::io::Error::from_raw_os_error(libc::ENOMEM))?;
        bytes.resize(count, 0);
        Ok((opened, bytes))
    })();
    // SAFETY: Host mem_ctx/event remain live on this thread. The send contract retains any output
    // only until completion/cancellation; the worker owns its FD and bytes.
    unsafe {
        asynchronous::send(mem_ctx, event, data, count, move || {
            let (opened, mut bytes) = prepared?;
            let _guard = opened
                .io
                .lock()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
            opened.client.call(Request::AuthorizeRead {
                handle: opened.info.handle,
            })?;
            let result = libc::pread(
                opened.fd.as_raw_fd(),
                bytes.as_mut_ptr().cast(),
                count,
                offset,
            );
            if result < 0 {
                return Err(std::io::Error::last_os_error());
            }
            bytes.truncate(result as usize);
            Ok(asynchronous::Completion { result, bytes })
        })
    }
}
unsafe extern "C" fn pwrite_send(
    handle: *mut sdk::vfs_handle_struct,
    mem_ctx: *mut sdk::TALLOC_CTX,
    event: *mut sdk::tevent_context,
    fsp: *mut sdk::files_struct,
    data: *const c_void,
    count: usize,
    offset: sdk::off_t,
) -> *mut sdk::tevent_req {
    let prepared = (|| -> std::io::Result<_> {
        if count > asynchronous::MAX_IO || (count > 0 && data.is_null()) {
            return Err(bad());
        }
        // SAFETY: The host handle is live on its owning event loop; snapshot duplicates the FD and
        // returns no host pointers to workers.
        let opened = unsafe { snapshot(handle, fsp) }?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count)
            .map_err(|_| std::io::Error::from_raw_os_error(libc::ENOMEM))?;
        // SAFETY: host input is readable for count bytes during this callback.
        bytes.extend_from_slice(unsafe { input_bytes(data, count) }?);
        Ok((opened, bytes))
    })();
    // SAFETY: Host mem_ctx/event remain live on this thread. The send contract retains any output
    // only until completion/cancellation; the worker owns its FD and bytes.
    unsafe {
        asynchronous::send(mem_ctx, event, std::ptr::null_mut(), 0, move || {
            let (opened, bytes) = prepared?;
            let _guard = opened
                .io
                .lock()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
            write_data(
                &opened.client,
                &opened.info,
                opened.fd.as_raw_fd(),
                opened.writable,
                &bytes,
                offset,
            )
            .map(asynchronous::Completion::done)
        })
    }
}
unsafe extern "C" fn io_recv(request: *mut sdk::tevent_req, aio: *mut sdk::vfs_aio_state) -> isize {
    // SAFETY: Samba passes the matching completed request from our send callback and writable aio
    // storage on the same event loop.
    unsafe { asynchronous::recv(request, aio) }
}
unsafe extern "C" fn deny_recvfile(
    _handle: *mut sdk::vfs_handle_struct,
    _from: i32,
    _to: *mut sdk::files_struct,
    _offset: sdk::off_t,
    _count: usize,
) -> isize {
    // Raw socket-to-fd copies cannot publish Dirty through the workspace.
    errno(libc::ENOTSUP);
    -1
}
unsafe extern "C" fn deny_sendfile(
    _handle: *mut sdk::vfs_handle_struct,
    _to: i32,
    _from: *mut sdk::files_struct,
    _header: *const sdk::DATA_BLOB,
    _offset: sdk::off_t,
    _count: usize,
) -> isize {
    // Require the read callback so current ACLs are checked for every read.
    errno(libc::ENOTSUP);
    -1
}
unsafe extern "C" fn close(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
) -> i32 {
    guarded(|| {
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let context = unsafe { context(handle) }?;
        let opened = context.files.remove(&(fsp as usize)).ok_or_else(bad)?;
        close_opened(opened, |handle| {
            context.client.call(Request::Close { handle }).map(|_| ())
        })
    })
}

unsafe extern "C" fn fsync_send(
    handle: *mut sdk::vfs_handle_struct,
    mem_ctx: *mut sdk::TALLOC_CTX,
    event: *mut sdk::tevent_context,
    fsp: *mut sdk::files_struct,
) -> *mut sdk::tevent_req {
    // SAFETY: The host handle is live on its owning event loop; snapshot duplicates the FD and
    // returns no host pointers to workers.
    let prepared = unsafe { snapshot(handle, fsp) };
    // SAFETY: Host mem_ctx/event remain live on this thread. The send contract retains any output
    // only until completion/cancellation; the worker owns its FD and bytes.
    unsafe {
        asynchronous::send(mem_ctx, event, std::ptr::null_mut(), 0, move || {
            let opened = prepared?;
            let _guard = opened
                .io
                .lock()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
            opened.client.call(Request::Flush {
                handle: opened.info.handle,
            })?;
            Ok(asynchronous::Completion::done(0))
        })
    }
}
unsafe extern "C" fn fsync_recv(
    request: *mut sdk::tevent_req,
    aio: *mut sdk::vfs_aio_state,
) -> i32 {
    // SAFETY: Samba passes the matching completed request from our send callback and writable aio
    // storage on the same event loop.
    unsafe { asynchronous::recv(request, aio) as i32 }
}

/// # Safety
/// A non-null stat points to live, aligned, exclusively writable SDK stat storage.
unsafe fn fill_stat(node: &Value, stat: *mut sdk::SMB_STRUCT_STAT) -> std::io::Result<()> {
    if stat.is_null() {
        return Err(bad());
    }
    let kind = node.get("kind").and_then(Value::as_str).ok_or_else(bad)?;
    let id = node
        .get("entry_id")
        .and_then(Value::as_str)
        .ok_or_else(bad)?;
    let size = node
        .get("size")
        .and_then(Value::as_str)
        .ok_or_else(bad)?
        .parse::<i64>()
        .map_err(|_| bad())?;
    if size < 0 {
        return Err(bad());
    }
    let mode = if kind == "directory" {
        libc::S_IFDIR | 0o700
    } else {
        libc::S_IFREG | 0o600
    };
    // SAFETY: The caller supplies exclusive writable SDK stat storage; null was rejected. Its POD
    // fields admit zero and are filled before return.
    unsafe {
        std::ptr::write_bytes(stat, 0, 1);
        (*stat).st_ex_dev = 1;
        (*stat).st_ex_ino = inode(id) as _;
        (*stat).st_ex_mode = mode as _;
        (*stat).st_ex_nlink = 1;
        (*stat).st_ex_uid = libc::geteuid();
        (*stat).st_ex_gid = libc::getegid();
        (*stat).st_ex_size = size as _;
        (*stat).st_ex_blksize = 4096;
        (*stat).st_ex_blocks = (size as u64).div_ceil(512) as _;
    }
    Ok(())
}
unsafe extern "C" fn stat(
    handle: *mut sdk::vfs_handle_struct,
    file: *mut sdk::smb_filename,
) -> i32 {
    guarded(|| {
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let context = unsafe { context(handle) }?;
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let path = unsafe { name(handle, file) }?;
        let (node, _) = context.client.call(Request::Stat { path: path.clone() })?;
        // The host does folded lookup through get_real_filename_at. Its POSIX
        // stat probe must distinguish spelling for case-only rename.
        if node.get("path").and_then(Value::as_str) != Some(path.as_str()) {
            return Err(std::io::Error::from_raw_os_error(libc::ENOENT));
        }
        // SAFETY: The host supplies exclusive writable stat storage; the filename (if used) was
        // validated by name above.
        unsafe { fill_stat(&node, &mut (*file).st) }?;
        Ok(0)
    })
}
unsafe extern "C" fn fstat(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
    stat: *mut sdk::SMB_STRUCT_STAT,
) -> i32 {
    guarded(|| {
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let context = unsafe { context(handle) }?;
        let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
        context.client.call(Request::AuthorizeRead {
            handle: opened.info.handle,
        })?;
        // SAFETY: libc::stat is a C POD output structure whose integer and pointer fields admit
        // zero initialization.
        let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: The descriptor is owned and the output points to a live, exclusively borrowed
        // stat structure.
        if unsafe { libc::fstat(opened.fd, &mut metadata) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: The host provides exclusive writable stat output for this callback, as required
        // by fill_stat.
        unsafe {
            fill_stat(
                &serde_json::json!({"kind":opened.info.kind,"entry_id":opened.info.entry_id,"size":if opened.info.kind=="directory"{"0".into()}else{metadata.st_size.to_string()}}),
                stat,
            )
        }?;
        Ok(0)
    })
}
unsafe extern "C" fn fstatat(
    handle: *mut sdk::vfs_handle_struct,
    dir: *const sdk::files_struct,
    file: *const sdk::smb_filename,
    stat: *mut sdk::SMB_STRUCT_STAT,
    _flags: i32,
) -> i32 {
    guarded(|| {
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let path = unsafe { at_name(handle, dir, file) }?;
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let context = unsafe { context(handle) }?;
        let (node, _) = context.client.call(Request::Stat { path: path.clone() })?;
        if node.get("path").and_then(Value::as_str) != Some(path.as_str()) {
            return Err(std::io::Error::from_raw_os_error(libc::ENOENT));
        }
        // SAFETY: The host supplies exclusive writable stat storage; the filename (if used) was
        // validated by name above.
        unsafe { fill_stat(&node, stat) }?;
        Ok(0)
    })
}
unsafe extern "C" fn real_filename(
    handle: *mut sdk::vfs_handle_struct,
    directory: *mut sdk::files_struct,
    component: *const libc::c_char,
    mem_ctx: *mut sdk::TALLOC_CTX,
    found: *mut *mut libc::c_char,
) -> sdk::NTSTATUS {
    let result = guarded(|| {
        if found.is_null() {
            return Err(bad());
        }
        // The host can pass its existing filename as the output slot. Preserve
        // that pointer on failure so a missing path can still become a new file.
        // SAFETY: The host component is NUL-terminated and live for this callback; text rejects
        // null and invalid UTF-8.
        let component = unsafe { text(component) }?;
        if component.is_empty()
            || component.contains('/')
            || component.contains('\\')
            || matches!(component, "." | "..")
        {
            return Err(bad());
        }
        let parent = if directory.is_null() {
            String::new()
        } else {
            // SAFETY: The host supplies live filename/directory objects and their string fields for
            // this callback; optional nulls are rejected or handled by the helper.
            unsafe { name(handle, (*directory).fsp_name) }?
        };
        let path = if parent.is_empty() {
            component.to_owned()
        } else {
            format!("{parent}/{component}")
        };
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        let (entry, _) = unsafe { context(handle) }?
            .client
            .call(Request::Stat { path })?;
        let name = entry.get("name").and_then(Value::as_str).ok_or_else(bad)?;
        let name = std::ffi::CString::new(name).map_err(|_| bad())?;
        // SAFETY: mem_ctx is a live host talloc context or null; name is a live CString copied by
        // talloc_strdup.
        let allocated = unsafe { sdk::talloc_strdup(mem_ctx.cast(), name.as_ptr()) };
        if allocated.is_null() {
            return Err(std::io::Error::from_raw_os_error(libc::ENOMEM));
        }
        // SAFETY: found is a checked non-null host output slot; allocated is a successful talloc
        // allocation owned by mem_ctx.
        unsafe {
            *found = allocated;
        }
        Ok(0)
    });
    if result == 0 {
        nt_status(0)
    } else {
        // SAFETY: On Linux __errno_location returns valid thread-local errno storage for this
        // thread.
        nt_status(match unsafe { *libc::__errno_location() } {
            libc::ENOENT => 0xc0000034,
            libc::EACCES | libc::EPERM => 0xc0000022,
            libc::ENOMEM => 0xc0000017,
            _ => 0xc0000001,
        })
    }
}
unsafe extern "C" fn mkdirat(
    handle: *mut sdk::vfs_handle_struct,
    dir: *mut sdk::files_struct,
    file: *const sdk::smb_filename,
    _mode: sdk::mode_t,
) -> i32 {
    guarded(|| {
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let path = unsafe { at_name(handle, dir, file) }?;
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        unsafe { context(handle) }?
            .client
            .call(Request::Namespace {
                kind: "mkdir".into(),
                path,
                destination: None,
                replace: false,
            })?;
        Ok(0)
    })
}
unsafe extern "C" fn unlinkat(
    handle: *mut sdk::vfs_handle_struct,
    dir: *mut sdk::files_struct,
    file: *const sdk::smb_filename,
    _flags: i32,
) -> i32 {
    guarded(|| {
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let path = unsafe { at_name(handle, dir, file) }?;
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        unsafe { context(handle) }?
            .client
            .call(Request::Namespace {
                kind: "delete".into(),
                path,
                destination: None,
                replace: false,
            })?;
        Ok(0)
    })
}
unsafe extern "C" fn renameat(
    handle: *mut sdk::vfs_handle_struct,
    srcdir: *mut sdk::files_struct,
    src: *const sdk::smb_filename,
    dstdir: *mut sdk::files_struct,
    dst: *const sdk::smb_filename,
    how: *const sdk::vfs_rename_how,
) -> i32 {
    guarded(|| {
        if how.is_null() {
            return Err(bad());
        }
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let path = unsafe { at_name(handle, srcdir, src) }?;
        // SAFETY: The host supplies live filename/directory objects and their string fields for
        // this callback; optional nulls are rejected or handled by the helper.
        let destination = Some(unsafe { at_name(handle, dstdir, dst) }?);
        // SAFETY: The host supplies a live configuration object; its pointer was checked non-null
        // before reading flags.
        let replace = unsafe { (*how).flags } & sdk::VFS_RENAME_HOW_NO_REPLACE as i32 == 0;
        // SAFETY: The owning host callback supplies a live handle; Context access stays on this
        // event-loop thread and does not escape the callback.
        unsafe { context(handle) }?
            .client
            .call(Request::Namespace {
                kind: "rename".into(),
                path,
                destination,
                replace,
            })?;
        Ok(0)
    })
}
unsafe extern "C" fn fdopendir(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
    _mask: *const libc::c_char,
    _attributes: u32,
) -> *mut sdk::DIR {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> std::io::Result<*mut sdk::DIR> {
            // SAFETY: The owning host callback supplies a live handle; Context access stays on this
            // event-loop thread and does not escape the callback.
            let context = unsafe { context(handle) }?;
            let path = context
                .files
                .get(&(fsp as usize))
                .ok_or_else(bad)?
                .info
                .path
                .clone();
            let (result, _) = context.client.call(Request::List { path })?;
            let mut entries = Vec::new();
            for node in result
                .get("entries")
                .and_then(Value::as_array)
                .ok_or_else(bad)?
            {
                let name = node.get("name").and_then(Value::as_str).ok_or_else(bad)?;
                let mut entry: sdk::dirent = Default::default();
                if name.len() >= entry.d_name.len() || name.as_bytes().contains(&0) {
                    return Err(bad());
                }
                entry.d_ino = inode(
                    node.get("entry_id")
                        .and_then(Value::as_str)
                        .ok_or_else(bad)?,
                ) as _;
                entry.d_type = if node.get("kind").and_then(Value::as_str) == Some("directory") {
                    libc::DT_DIR
                } else {
                    libc::DT_REG
                };
                entry.d_reclen = std::mem::size_of::<sdk::dirent>() as _;
                for (i, byte) in name.bytes().enumerate() {
                    entry.d_name[i] = byte as _;
                }
                entries.push(entry);
            }
            Ok(Box::into_raw(Box::new(Directory {
                entries,
                position: 0,
            }))
            .cast())
        },
    ));
    match result {
        Ok(Ok(dir)) => dir,
        Ok(Err(error)) => {
            errno(error_errno(&error));
            std::ptr::null_mut()
        }
        Err(_) => {
            errno(libc::EIO);
            std::ptr::null_mut()
        }
    }
}
unsafe extern "C" fn readdir(
    _handle: *mut sdk::vfs_handle_struct,
    _fsp: *mut sdk::files_struct,
    directory: *mut sdk::DIR,
) -> *mut sdk::dirent {
    if directory.is_null() {
        errno(libc::EINVAL);
        return std::ptr::null_mut();
    }
    // SAFETY: This non-null opaque directory is the live Box returned by fdopendir; the host
    // serializes callbacks and closedir consumes it exactly once.
    let directory = unsafe { &mut *directory.cast::<Directory>() };
    match directory.entries.get_mut(directory.position) {
        Some(entry) => {
            directory.position += 1;
            entry as *mut sdk::dirent
        }
        None => {
            errno(0);
            std::ptr::null_mut()
        }
    }
}
unsafe extern "C" fn rewind(_handle: *mut sdk::vfs_handle_struct, directory: *mut sdk::DIR) {
    if !directory.is_null() {
        // SAFETY: The non-null opaque pointer is our live Directory box from fdopendir, exclusively
        // accessed on the host event loop.
        unsafe {
            (*directory.cast::<Directory>()).position = 0;
        }
    }
}
unsafe extern "C" fn closedir(
    _handle: *mut sdk::vfs_handle_struct,
    directory: *mut sdk::DIR,
) -> i32 {
    if directory.is_null() {
        errno(libc::EINVAL);
        return -1;
    }
    // SAFETY: This non-null opaque directory is the live Box returned by fdopendir; the host
    // serializes callbacks and closedir consumes it exactly once.
    drop(unsafe { Box::from_raw(directory.cast::<Directory>()) });
    0
}

// Refuse lower-layer mutations that could bypass the workspace or change an
// immutable content descriptor. These features are outside the SMB subset.
macro_rules! deny_int {
    ($name:ident($($argument:ident:$ty:ty),* $(,)?))=>{
        unsafe extern "C" fn $name($($argument:$ty),*)->i32{let _=($($argument,)*);errno(libc::ENOTSUP);-1}
    };
}
deny_int!(deny_chmod(h: *mut sdk::vfs_handle_struct, f: *mut sdk::files_struct, m: sdk::mode_t));
deny_int!(
    deny_chown(
        h: *mut sdk::vfs_handle_struct,
        f: *mut sdk::files_struct,
        u: sdk::uid_t,
        g: sdk::gid_t,
    )
);
deny_int!(
    deny_lchown(
        h: *mut sdk::vfs_handle_struct,
        f: *const sdk::smb_filename,
        u: sdk::uid_t,
        g: sdk::gid_t,
    )
);
deny_int!(
    deny_times(
        h: *mut sdk::vfs_handle_struct,
        f: *mut sdk::files_struct,
        t: *mut sdk::smb_file_time,
    )
);
deny_int!(
    deny_allocate(
        h: *mut sdk::vfs_handle_struct,
        f: *mut sdk::files_struct,
        m: u32,
        o: sdk::off_t,
        n: sdk::off_t,
    )
);
deny_int!(deny_flags(h: *mut sdk::vfs_handle_struct, f: *mut sdk::files_struct, flags: u32));
deny_int!(
    deny_symlink(
        h: *mut sdk::vfs_handle_struct,
        t: *const sdk::smb_filename,
        d: *mut sdk::files_struct,
        n: *const sdk::smb_filename,
    )
);
deny_int!(
    deny_link(
        h: *mut sdk::vfs_handle_struct,
        s: *mut sdk::files_struct,
        a: *const sdk::smb_filename,
        d: *mut sdk::files_struct,
        b: *const sdk::smb_filename,
        flags: i32,
    )
);
deny_int!(
    deny_mknod(
        h: *mut sdk::vfs_handle_struct,
        d: *mut sdk::files_struct,
        n: *const sdk::smb_filename,
        m: sdk::mode_t,
        device: sdk::dev_t,
    )
);
deny_int!(
    deny_setxattr(
        h: *mut sdk::vfs_handle_struct,
        f: *mut sdk::files_struct,
        n: *const libc::c_char,
        v: *const c_void,
        len: usize,
        flags: i32,
    )
);
deny_int!(
    deny_removexattr(
        h: *mut sdk::vfs_handle_struct,
        f: *mut sdk::files_struct,
        n: *const libc::c_char,
    )
);
deny_int!(
    deny_acl(
        h: *mut sdk::vfs_handle_struct,
        f: *mut sdk::files_struct,
        t: sdk::SMB_ACL_TYPE_T,
        a: sdk::SMB_ACL_T,
    )
);
deny_int!(deny_delete_acl(h: *mut sdk::vfs_handle_struct, f: *mut sdk::files_struct));
fn nt_status(value: u32) -> sdk::NTSTATUS {
    sdk::NTSTATUS { v: value }
}
unsafe extern "C" fn deny_nt_acl(
    _h: *mut sdk::vfs_handle_struct,
    _f: *mut sdk::files_struct,
    _security: u32,
    _sd: *const sdk::security_descriptor,
) -> sdk::NTSTATUS {
    nt_status(0xc00000bb)
}
unsafe extern "C" fn deny_dos(
    _h: *mut sdk::vfs_handle_struct,
    _f: *mut sdk::files_struct,
    _mode: u32,
) -> sdk::NTSTATUS {
    nt_status(0xc00000bb)
}
unsafe extern "C" fn deny_compression(
    _h: *mut sdk::vfs_handle_struct,
    _c: *mut sdk::TALLOC_CTX,
    _f: *mut sdk::files_struct,
    _format: u16,
) -> sdk::NTSTATUS {
    nt_status(0xc00000bb)
}
unsafe extern "C" fn deny_fsctl(
    _h: *mut sdk::vfs_handle_struct,
    _f: *mut sdk::files_struct,
    _c: *mut sdk::TALLOC_CTX,
    _function: u32,
    _flags: u16,
    _input: *const u8,
    _input_len: u32,
    output: *mut *mut u8,
    _max: u32,
    len: *mut u32,
) -> sdk::NTSTATUS {
    if !output.is_null() {
        // SAFETY: The non-null output slot is writable SDK callback storage; no allocation is
        // transferred on this unsupported operation.
        unsafe {
            *output = std::ptr::null_mut();
        }
    }
    if !len.is_null() {
        // SAFETY: The non-null length slot is writable SDK callback storage for this invocation.
        unsafe {
            *len = 0;
        }
    }
    nt_status(0xc00000bb)
}
unsafe extern "C" fn deny_offload_write(
    _h: *mut sdk::vfs_handle_struct,
    _c: *mut sdk::TALLOC_CTX,
    _event: *mut sdk::tevent_context,
    _fsctl: u32,
    _token: *mut sdk::DATA_BLOB,
    _offset: sdk::off_t,
    _f: *mut sdk::files_struct,
    _dest: sdk::off_t,
    _count: sdk::off_t,
) -> *mut sdk::tevent_req {
    errno(libc::ENOTSUP);
    std::ptr::null_mut()
}

static FUNCTIONS: OnceLock<sdk::vfs_fn_pointers> = OnceLock::new();

/// The host resolves this ABI symbol from the cdylib. Binding layout is checked against
/// its exact target source version during the build; no guessed struct offsets.
///
/// # Safety
/// Call only from the matching Samba SDK host after its module registry is ready.
/// The host must honor the module-level callback ownership and lifetime contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn samba_init_module() -> sdk::NTSTATUS {
    let functions = FUNCTIONS.get_or_init(|| sdk::vfs_fn_pointers {
        connect_fn: Some(connect),
        disconnect_fn: Some(disconnect),
        openat_fn: Some(openat),
        close_fn: Some(close),
        pread_fn: Some(pread),
        pread_send_fn: Some(pread_send),
        pread_recv_fn: Some(io_recv),
        pwrite_fn: Some(pwrite),
        pwrite_send_fn: Some(pwrite_send),
        pwrite_recv_fn: Some(io_recv),
        recvfile_fn: Some(deny_recvfile),
        sendfile_fn: Some(deny_sendfile),
        ftruncate_fn: Some(truncate),
        fsync_send_fn: Some(fsync_send),
        fsync_recv_fn: Some(fsync_recv),
        stat_fn: Some(stat),
        lstat_fn: Some(stat),
        fstat_fn: Some(fstat),
        fstatat_fn: Some(fstatat),
        get_real_filename_at_fn: Some(real_filename),
        mkdirat_fn: Some(mkdirat),
        unlinkat_fn: Some(unlinkat),
        renameat_fn: Some(renameat),
        fdopendir_fn: Some(fdopendir),
        readdir_fn: Some(readdir),
        rewind_dir_fn: Some(rewind),
        closedir_fn: Some(closedir),
        fchmod_fn: Some(deny_chmod),
        fchown_fn: Some(deny_chown),
        lchown_fn: Some(deny_lchown),
        fntimes_fn: Some(deny_times),
        fallocate_fn: Some(deny_allocate),
        fchflags_fn: Some(deny_flags),
        symlinkat_fn: Some(deny_symlink),
        linkat_fn: Some(deny_link),
        mknodat_fn: Some(deny_mknod),
        fsetxattr_fn: Some(deny_setxattr),
        fremovexattr_fn: Some(deny_removexattr),
        sys_acl_set_fd_fn: Some(deny_acl),
        sys_acl_delete_def_fd_fn: Some(deny_delete_acl),
        fset_nt_acl_fn: Some(deny_nt_acl),
        fset_dos_attributes_fn: Some(deny_dos),
        set_compression_fn: Some(deny_compression),
        fsctl_fn: Some(deny_fsctl),
        offload_write_send_fn: Some(deny_offload_write),
        ..Default::default()
    });
    // SAFETY: The SDK version matches build.rs; the name and OnceLock callback table have static
    // lifetime and match the generated function signatures.
    unsafe { sdk::smb_register_vfs(51, c"zxs".as_ptr(), functions) }
}

#[cfg(test)]
mod tests;
