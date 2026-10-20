//! All ABI calls are generated from the target SDK. No custom C shim.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::{CStr, c_void},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};
use zxs_smb::{
    client::{Client, error_errno},
    protocol::{OpenResult, Request},
};

mod sdk {
    #![allow(
        dead_code,
        non_camel_case_types,
        non_snake_case,
        non_upper_case_globals,
        unnecessary_transmutes
    )]
    include!(concat!(env!("OUT_DIR"), "/sdk.rs"));
}
#[path = "asynchronous.rs"]
mod asynchronous;

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
struct Snapshot {
    client: Client,
    info: OpenResult,
    fd: OwnedFd,
    writable: bool,
    io: Arc<Mutex<()>>,
}
unsafe fn snapshot(
    handle: *mut sdk::vfs_handle_struct,
    fsp: *mut sdk::files_struct,
) -> std::io::Result<Snapshot> {
    let context = unsafe { context(handle) }?;
    let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
    let descriptor = unsafe { libc::fcntl(opened.fd, libc::F_DUPFD_CLOEXEC, 0) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(Snapshot {
        client: context.client.clone(),
        info: opened.info.clone(),
        fd: unsafe { OwnedFd::from_raw_fd(descriptor) },
        writable: opened.writable,
        io: opened.io.clone(),
    })
}

fn errno(error: i32) {
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
unsafe fn text<'a>(pointer: *const libc::c_char) -> std::io::Result<&'a str> {
    if pointer.is_null() {
        return Err(bad());
    }
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map_err(|_| bad())
}
unsafe fn context<'a>(handle: *mut sdk::vfs_handle_struct) -> std::io::Result<&'a mut Context> {
    if handle.is_null() || unsafe { (*handle).data }.is_null() {
        return Err(bad());
    }
    Ok(unsafe { &mut *((*handle).data.cast::<Context>()) })
}
unsafe fn name(
    handle: *mut sdk::vfs_handle_struct,
    pointer: *const sdk::smb_filename,
) -> std::io::Result<String> {
    if pointer.is_null() {
        return Err(bad());
    }
    if !unsafe { (*pointer).stream_name }.is_null()
        && !unsafe { text((*pointer).stream_name) }?.is_empty()
    {
        return Err(std::io::Error::from_raw_os_error(libc::ENOTSUP));
    }
    let name = unsafe { text((*pointer).base_name) }?;
    if name.contains('\\') {
        return Err(bad());
    }
    let name = if name.starts_with('/') {
        if handle.is_null() || unsafe { (*handle).conn }.is_null() {
            return Err(bad());
        }
        let root = unsafe { text((*(*handle).conn).connectpath) }?;
        Path::new(name)
            .strip_prefix(root)
            .map_err(|_| bad())?
            .to_str()
            .ok_or_else(bad)?
    } else {
        name
    };
    let mut components = Vec::new();
    for component in name.split('/') {
        match component {
            "" | "." => {}
            ".." => return Err(bad()),
            component => components.push(component),
        }
    }
    Ok(components.join("/"))
}
unsafe fn at_name(
    handle: *mut sdk::vfs_handle_struct,
    directory: *const sdk::files_struct,
    file: *const sdk::smb_filename,
) -> std::io::Result<String> {
    let child = unsafe { name(handle, file) }?;
    if unsafe { text((*file).base_name) }?.starts_with('/') {
        return Ok(child);
    }
    let parent = if directory.is_null() {
        String::new()
    } else {
        unsafe { name(handle, (*directory).fsp_name) }?
    };
    Ok(if parent.is_empty() {
        child
    } else if child.is_empty() {
        parent
    } else {
        format!("{parent}/{child}")
    })
}
fn inode(id: &str) -> u64 {
    let bytes = blake3::hash(id.as_bytes());
    u64::from_le_bytes(bytes.as_bytes()[..8].try_into().unwrap()).max(1)
}

unsafe extern "C" fn free_context(data: *mut *mut c_void) {
    if data.is_null() {
        return;
    }
    let pointer = unsafe { *data };
    if !pointer.is_null() {
        let context = unsafe { Box::from_raw(pointer.cast::<Context>()) };
        for opened in context.files.values() {
            unsafe {
                libc::close(opened.fd);
            }
        }
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
        if handle.is_null() || unsafe { (*handle).conn }.is_null() {
            return Err(bad());
        }
        let parameters = unsafe { (*(*handle).conn).params };
        if parameters.is_null() {
            return Err(bad());
        }
        let service = unsafe { (*parameters).service };
        let socket = unsafe {
            text(sdk::lp_parm_const_string(
                service,
                c"zxs".as_ptr(),
                c"socket".as_ptr(),
                std::ptr::null(),
            ))
        }?;
        let repository = unsafe {
            text(sdk::lp_parm_const_string(
                service,
                c"zxs".as_ptr(),
                c"repository".as_ptr(),
                std::ptr::null(),
            ))
        }?;
        let client = Client::connect(&PathBuf::from(socket), unsafe { text(user) }?, repository)?;
        let boxed = Box::new(Context {
            client,
            files: BTreeMap::new(),
        });
        unsafe {
            (*handle).data = Box::into_raw(boxed).cast();
            (*handle).free_data = Some(free_context);
        }
        Ok(0)
    })
}
unsafe extern "C" fn disconnect(handle: *mut sdk::vfs_handle_struct) {
    if !handle.is_null() {
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
        let flags = unsafe { (*how).flags };
        let writable = flags & libc::O_ACCMODE != libc::O_RDONLY;
        let path = unsafe { at_name(handle, directory, file) }?;
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
            let context = unsafe { context(handle) }?;
            let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
            if offset < 0 || count > isize::MAX as usize {
                return Err(bad());
            }
            let _guard = opened
                .io
                .lock()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
            context.client.call(Request::AuthorizeRead {
                handle: opened.info.handle,
            })?;
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
                data,
                count,
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
    data: *const c_void,
    count: usize,
    offset: sdk::off_t,
) -> std::io::Result<isize> {
    if !writable || count > isize::MAX as usize {
        return Err(std::io::Error::from_raw_os_error(libc::EACCES));
    }
    let offset = if offset == -1 {
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
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
    let result = unsafe { libc::pwrite(fd, data, count, offset) };
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
        let context = unsafe { context(handle) }?;
        let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
        if !opened.writable || length < 0 || length as u64 > opened.info.max_file_bytes {
            return Err(bad());
        }
        let _guard = opened
            .io
            .lock()
            .map_err(|_| std::io::Error::from_raw_os_error(libc::EIO))?;
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
        let opened = unsafe { snapshot(handle, fsp) }?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count)
            .map_err(|_| std::io::Error::from_raw_os_error(libc::ENOMEM))?;
        bytes.resize(count, 0);
        Ok((opened, bytes))
    })();
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
        let opened = unsafe { snapshot(handle, fsp) }?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(count)
            .map_err(|_| std::io::Error::from_raw_os_error(libc::ENOMEM))?;
        if count > 0 {
            bytes
                .extend_from_slice(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), count) });
        }
        Ok((opened, bytes))
    })();
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
                bytes.as_ptr().cast(),
                bytes.len(),
                offset,
            )
            .map(asynchronous::Completion::done)
        })
    }
}
unsafe extern "C" fn io_recv(request: *mut sdk::tevent_req, aio: *mut sdk::vfs_aio_state) -> isize {
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
        let context = unsafe { context(handle) }?;
        let opened = context.files.remove(&(fsp as usize)).ok_or_else(bad)?;
        close_opened(opened, |handle| {
            context.client.call(Request::Close { handle }).map(|_| ())
        })
    })
}

fn close_opened(
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
    let closed = unsafe { libc::close(descriptor.into_raw_fd()) };
    saved?;
    if closed < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(0)
}
unsafe extern "C" fn fsync_send(
    handle: *mut sdk::vfs_handle_struct,
    mem_ctx: *mut sdk::TALLOC_CTX,
    event: *mut sdk::tevent_context,
    fsp: *mut sdk::files_struct,
) -> *mut sdk::tevent_req {
    let prepared = unsafe { snapshot(handle, fsp) };
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
    unsafe { asynchronous::recv(request, aio) as i32 }
}

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
        let context = unsafe { context(handle) }?;
        let path = unsafe { name(handle, file) }?;
        let (node, _) = context.client.call(Request::Stat { path: path.clone() })?;
        // The host does folded lookup through get_real_filename_at. Its POSIX
        // stat probe must distinguish spelling for case-only rename.
        if node.get("path").and_then(Value::as_str) != Some(path.as_str()) {
            return Err(std::io::Error::from_raw_os_error(libc::ENOENT));
        }
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
        let context = unsafe { context(handle) }?;
        let opened = context.files.get(&(fsp as usize)).ok_or_else(bad)?;
        context.client.call(Request::AuthorizeRead {
            handle: opened.info.handle,
        })?;
        let mut metadata: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(opened.fd, &mut metadata) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
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
        let path = unsafe { at_name(handle, dir, file) }?;
        let context = unsafe { context(handle) }?;
        let (node, _) = context.client.call(Request::Stat { path: path.clone() })?;
        if node.get("path").and_then(Value::as_str) != Some(path.as_str()) {
            return Err(std::io::Error::from_raw_os_error(libc::ENOENT));
        }
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
            unsafe { name(handle, (*directory).fsp_name) }?
        };
        let path = if parent.is_empty() {
            component.to_owned()
        } else {
            format!("{parent}/{component}")
        };
        let (entry, _) = unsafe { context(handle) }?
            .client
            .call(Request::Stat { path })?;
        let name = entry.get("name").and_then(Value::as_str).ok_or_else(bad)?;
        let name = std::ffi::CString::new(name).map_err(|_| bad())?;
        let allocated = unsafe { sdk::talloc_strdup(mem_ctx.cast(), name.as_ptr()) };
        if allocated.is_null() {
            return Err(std::io::Error::from_raw_os_error(libc::ENOMEM));
        }
        unsafe {
            *found = allocated;
        }
        Ok(0)
    });
    if result == 0 {
        nt_status(0)
    } else {
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
        let path = unsafe { at_name(handle, dir, file) }?;
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
        let path = unsafe { at_name(handle, dir, file) }?;
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
        let path = unsafe { at_name(handle, srcdir, src) }?;
        let destination = Some(unsafe { at_name(handle, dstdir, dst) }?);
        let replace = unsafe { (*how).flags } & sdk::VFS_RENAME_HOW_NO_REPLACE as i32 == 0;
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
                let mut entry: sdk::dirent = unsafe { std::mem::zeroed() };
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
    unsafe { std::mem::transmute(value) }
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
        unsafe {
            *output = std::ptr::null_mut();
        }
    }
    if !len.is_null() {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn samba_init_module() -> sdk::NTSTATUS {
    let functions = FUNCTIONS.get_or_init(|| {
        let mut functions: sdk::vfs_fn_pointers = unsafe { std::mem::zeroed() };
        functions.connect_fn = Some(connect);
        functions.disconnect_fn = Some(disconnect);
        functions.openat_fn = Some(openat);
        functions.close_fn = Some(close);
        functions.pread_fn = Some(pread);
        functions.pread_send_fn = Some(pread_send);
        functions.pread_recv_fn = Some(io_recv);
        functions.pwrite_fn = Some(pwrite);
        functions.pwrite_send_fn = Some(pwrite_send);
        functions.pwrite_recv_fn = Some(io_recv);
        functions.recvfile_fn = Some(deny_recvfile);
        functions.sendfile_fn = Some(deny_sendfile);
        functions.ftruncate_fn = Some(truncate);
        functions.fsync_send_fn = Some(fsync_send);
        functions.fsync_recv_fn = Some(fsync_recv);
        functions.stat_fn = Some(stat);
        functions.lstat_fn = Some(stat);
        functions.fstat_fn = Some(fstat);
        functions.fstatat_fn = Some(fstatat);
        functions.get_real_filename_at_fn = Some(real_filename);
        functions.mkdirat_fn = Some(mkdirat);
        functions.unlinkat_fn = Some(unlinkat);
        functions.renameat_fn = Some(renameat);
        functions.fdopendir_fn = Some(fdopendir);
        functions.readdir_fn = Some(readdir);
        functions.rewind_dir_fn = Some(rewind);
        functions.closedir_fn = Some(closedir);
        functions.fchmod_fn = Some(deny_chmod);
        functions.fchown_fn = Some(deny_chown);
        functions.lchown_fn = Some(deny_lchown);
        functions.fntimes_fn = Some(deny_times);
        functions.fallocate_fn = Some(deny_allocate);
        functions.fchflags_fn = Some(deny_flags);
        functions.symlinkat_fn = Some(deny_symlink);
        functions.linkat_fn = Some(deny_link);
        functions.mknodat_fn = Some(deny_mknod);
        functions.fsetxattr_fn = Some(deny_setxattr);
        functions.fremovexattr_fn = Some(deny_removexattr);
        functions.sys_acl_set_fd_fn = Some(deny_acl);
        functions.sys_acl_delete_def_fd_fn = Some(deny_delete_acl);
        functions.fset_nt_acl_fn = Some(deny_nt_acl);
        functions.fset_dos_attributes_fn = Some(deny_dos);
        functions.set_compression_fn = Some(deny_compression);
        functions.fsctl_fn = Some(deny_fsctl);
        functions.offload_write_send_fn = Some(deny_offload_write);
        functions
    });
    unsafe { sdk::smb_register_vfs(51, c"zxs".as_ptr(), functions) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_open_descriptor_is_closed_on_poisoned_lock_and_remote_save_failure() {
        use std::{io::Read, os::unix::net::UnixStream};
        for poisoned in [true, false] {
            let (descriptor, mut peer) = UnixStream::pair().unwrap();
            peer.set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .unwrap();
            let lock = Arc::new(Mutex::new(()));
            if poisoned {
                let _ = std::panic::catch_unwind(|| {
                    let _guard = lock.lock().unwrap();
                    panic!("simulate a failed I/O worker");
                });
            }
            let opened = Open {
                info: OpenResult {
                    entry_id: "entry".into(),
                    kind: "file".into(),
                    path: "file.txt".into(),
                    handle: 7,
                    size: 0,
                    max_file_bytes: 1,
                    version_id: None,
                    workspace_id: None,
                },
                fd: descriptor.into_raw_fd(),
                writable: true,
                io: lock,
            };
            let error = close_opened(opened, |handle| {
                assert!(!poisoned, "a poisoned I/O lock cannot save content");
                assert_eq!(handle, 7);
                Err(std::io::ErrorKind::PermissionDenied.into())
            })
            .unwrap_err();
            if poisoned {
                assert_eq!(error.raw_os_error(), Some(libc::EIO));
            } else {
                assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            }
            assert_eq!(
                peer.read(&mut [0; 1]).unwrap(),
                0,
                "removed descriptor leaked"
            );
        }
    }
    #[test]
    fn failed_name_lookup_preserves_the_callers_filename() {
        let original = c"new.txt".as_ptr().cast_mut();
        let mut found = original;
        let status = unsafe {
            real_filename(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                c"..".as_ptr(),
                std::ptr::null_mut(),
                &mut found,
            )
        };
        assert_ne!(status.v, 0);
        assert_eq!(found, original);
    }
    #[test]
    fn absolute_names_are_confined_to_the_exact_share_root() {
        let root = c"/safe/share";
        let mut connection: sdk::connection_struct = unsafe { std::mem::zeroed() };
        connection.connectpath = root.as_ptr().cast_mut();
        let mut handle: sdk::vfs_handle_struct = unsafe { std::mem::zeroed() };
        handle.conn = &mut connection;
        let mut file: sdk::smb_filename = unsafe { std::mem::zeroed() };
        for (input, expected) in [
            ("/safe/share", Some("")),
            ("/safe/share/./文件.txt", Some("文件.txt")),
            ("./documents/file.txt", Some("documents/file.txt")),
            ("/safe/share-other/file", None),
            ("/safe/share/../secret", None),
            ("/outside/file", None),
            ("documents/../../secret", None),
        ] {
            let input = std::ffi::CString::new(input).unwrap();
            file.base_name = input.as_ptr().cast_mut();
            let result = unsafe { name(&mut handle, &file) };
            match expected {
                Some(expected) => assert_eq!(result.unwrap(), expected),
                None => assert!(result.is_err()),
            }
        }
    }
}
