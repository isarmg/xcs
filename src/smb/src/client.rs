use crate::protocol::{FrameStream, OpenResult, PROTOCOL, Request, Response};
use serde_json::Value;
use std::{
    io,
    net::Shutdown,
    os::{
        fd::{IntoRawFd, OwnedFd},
        unix::net::UnixStream,
    },
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Client(Arc<Mutex<Option<FrameStream>>>);

#[derive(Debug)]
pub struct RemoteError {
    pub code: String,
    pub message: String,
    pub operation_id: Option<String>,
    pub retryable: bool,
    errno: i32,
}
impl std::fmt::Display for RemoteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)?;
        if let Some(operation) = &self.operation_id {
            write!(formatter, "; operation_id={operation}")?;
        }
        Ok(())
    }
}
impl std::error::Error for RemoteError {}
fn remote_errno(code: &str) -> i32 {
    match code {
        "NOT_FOUND"
        | "ENTRY_NOT_FOUND"
        | "VERSION_NOT_FOUND"
        | "BRANCH_NOT_FOUND"
        | "REPOSITORY_NOT_FOUND" => libc::ENOENT,
        "UNAUTHENTICATED" | "UNAUTHORIZED" | "PERMISSION_DENIED" | "FORBIDDEN" => libc::EACCES,
        "BRANCH_NOT_ACTIVE" | "FILE_BRANCH_READ_ONLY" => libc::EROFS,
        "FILE_WRITE_BUSY" | "BRANCH_WRITE_BUSY" | "WORKSPACE_BUSY" | "CONTENT_PINNED" => {
            libc::EBUSY
        }
        "NAME_CONFLICT" | "NAME_COLLISION" | "OBJECT_COLLISION" => libc::EEXIST,
        "NO_SPACE" | "QUOTA_EXCEEDED" => libc::ENOSPC,
        "FILE_TOO_LARGE" => libc::EFBIG,
        "RESOURCE_LIMIT_EXCEEDED" | "RESOURCE_LIMIT" => libc::EAGAIN,
        "NAME_TOO_LONG" | "PATH_TOO_DEEP" => libc::ENAMETOOLONG,
        "DIRECTORY_NOT_EMPTY" => libc::ENOTEMPTY,
        "IS_DIRECTORY" => libc::EISDIR,
        "PARENT_NOT_DIRECTORY" | "NOT_DIRECTORY" => libc::ENOTDIR,
        "UNSUPPORTED"
        | "OPERATION_UNSUPPORTED"
        | "UNSUPPORTED_OPERATION"
        | "FORMAT_UNSUPPORTED" => libc::ENOTSUP,
        "INVALID_REQUEST" | "INVALID_NAME" | "INVALID_PATH" | "INVALID_MOVE" | "NAME_INVALID" => {
            libc::EINVAL
        }
        "SMB_REBIND_REQUIRED" | "TREE_MOVED" | "BRANCH_ARCHIVED" => libc::ESTALE,
        "FILE_VERSION_CONFLICT"
        | "FILE_CONFLICT"
        | "FILE_BRANCH_CONFLICT"
        | "DIRECTORY_BRANCH_CONFLICT"
        | "DIRECTORY_ALIAS_CONFLICT"
        | "BATCH_MAPPING_CONFLICT"
        | "BATCH_SCOPE_CONFLICT"
        | "NAMESPACE_CONFLICT"
        | "HEAD_CONFLICT"
        | "WORKSPACE_REBASE_REQUIRED" => libc::EAGAIN,
        _ => libc::EIO,
    }
}
/// Preserve remote diagnostics while presenting the corresponding filesystem errno.
pub fn error_errno(error: &io::Error) -> i32 {
    error
        .raw_os_error()
        .or_else(|| {
            error
                .get_ref()
                .and_then(|inner| inner.downcast_ref::<RemoteError>())
                .map(|remote| remote.errno)
        })
        .unwrap_or(libc::EIO)
}

/// Finish an already-removed VFS handle and always release its local descriptor.
///
/// Close is terminal even when publication fails. If a worker poisoned the I/O
/// lock, detach instead: neither publishing suspect bytes nor aborting/deleting
/// them is safe. The guard still serializes against outstanding FD users.
pub fn close_descriptor(
    handle: u64,
    descriptor: OwnedFd,
    io_lock: &Mutex<()>,
    finish: impl FnOnce(Request) -> io::Result<()>,
) -> io::Result<()> {
    let (guard, poisoned) = match io_lock.lock() {
        Ok(guard) => (guard, false),
        Err(error) => (error.into_inner(), true),
    };
    let request = if poisoned {
        Request::Detach { handle }
    } else {
        Request::Close { handle }
    };
    let saved = finish(request);
    // SAFETY: into_raw_fd transfers the sole ownership to close. Do not retry a
    // failed close: the numeric descriptor may already have been reused.
    let closed = unsafe { libc::close(descriptor.into_raw_fd()) };
    let close_error = (closed < 0).then(io::Error::last_os_error);
    drop(guard);
    saved?;
    if poisoned {
        return Err(io::Error::from_raw_os_error(libc::EIO));
    }
    match close_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

impl Client {
    pub fn connect(socket: &Path, username: &str, repository_id: &str) -> io::Result<Self> {
        let client = Self(Arc::new(Mutex::new(Some(FrameStream::new(
            UnixStream::connect(socket)?,
        )?))));
        let (_, fd) = client.call(Request::Hello {
            protocol: PROTOCOL.into(),
            username: username.into(),
            repository_id: repository_id.into(),
        })?;
        if fd.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "hello returned a descriptor",
            ));
        }
        Ok(client)
    }
    pub fn call(&self, request: Request) -> io::Result<(Value, Option<OwnedFd>)> {
        let mut connection = self
            .0
            .lock()
            .map_err(|_| io::Error::other("RPC connection poisoned"))?;
        let stream = connection.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "RPC connection is closed")
        })?;
        let exchange = stream
            .send(&request, None)
            .and_then(|()| stream.receive::<Response>());
        let (response, fd) = match exchange {
            Ok(response) => response,
            Err(error) => {
                // A partial request/response or invalid frame loses message
                // alignment. Close the shared connection before unlocking so
                // every clone fails instead of consuming a stale response.
                if let Some(stream) = connection.take() {
                    let _ = stream.0.shutdown(Shutdown::Both);
                }
                return Err(match request {
                    Request::Namespace { operation_id, .. } => io::Error::new(
                        error.kind(),
                        RemoteError {
                            code: "RPC_OUTCOME_UNKNOWN".into(),
                            message: format!(
                                "{error}; query the original operation before retrying"
                            ),
                            operation_id: Some(operation_id),
                            retryable: false,
                            errno: error.raw_os_error().unwrap_or(libc::EIO),
                        },
                    ),
                    _ => error,
                });
            }
        };
        match response {
            Response::Ok { result } => Ok((result, fd)),
            Response::Error {
                code,
                message,
                operation_id,
                retryable,
            } => {
                let errno = remote_errno(&code);
                Err(io::Error::new(
                    io::Error::from_raw_os_error(errno).kind(),
                    RemoteError {
                        code,
                        message,
                        operation_id,
                        retryable,
                        errno,
                    },
                ))
            }
        }
    }
    pub fn open(&self, request: Request) -> io::Result<(OpenResult, OwnedFd)> {
        if !matches!(request, Request::Open { .. }) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected open request",
            ));
        }
        let (result, fd) = self.call(request)?;
        let result = serde_json::from_value(result).map_err(io::Error::other)?;
        Ok((
            result,
            fd.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "open returned no descriptor")
            })?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_transport_failure_retains_identity_and_disconnects_every_clone() {
        for send_failure in [false, true] {
            let request = crate::protocol::namespace_request("mkdir", "new".into(), None, false);
            let Request::Namespace { operation_id, .. } = &request else {
                unreachable!()
            };
            let expected = operation_id.clone();
            let (one, two) = UnixStream::pair().unwrap();
            let (done, disconnected) = std::sync::mpsc::channel();
            let server = std::thread::spawn(move || {
                if !send_failure {
                    let mut stream = FrameStream::new(two).unwrap();
                    let (received, _) = stream.receive::<Request>().unwrap();
                    assert!(matches!(received, Request::Namespace { .. }));
                    // The peer consumed the operation, but its reply is lost.
                } else {
                    drop(two);
                }
                done.send(()).unwrap();
            });
            if send_failure {
                disconnected.recv().unwrap();
            }
            let client = Client(Arc::new(Mutex::new(Some(FrameStream::new(one).unwrap()))));
            let clone = client.clone();
            let error = client.call(request).unwrap_err();
            assert_eq!(
                error.kind(),
                if send_failure {
                    io::ErrorKind::BrokenPipe
                } else {
                    io::ErrorKind::UnexpectedEof
                }
            );
            assert_eq!(
                error_errno(&error),
                if send_failure { libc::EPIPE } else { libc::EIO }
            );
            let remote = error
                .get_ref()
                .and_then(|error| error.downcast_ref::<RemoteError>())
                .expect("transport failure lost the caller's operation identity");
            assert_eq!(remote.operation_id.as_deref(), Some(expected.as_str()));
            assert_eq!(remote.code, "RPC_OUTCOME_UNKNOWN");
            assert!(!remote.retryable);
            assert_eq!(
                clone
                    .call(Request::Stat { path: "new".into() })
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::NotConnected
            );
            server.join().unwrap();
        }
    }

    #[test]
    fn terminal_close_releases_descriptor_on_business_error_poison_or_unwind() {
        use std::io::Read;
        for mode in ["business_error", "poison", "unwind"] {
            let (descriptor, mut peer) = UnixStream::pair().unwrap();
            peer.set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .unwrap();
            let lock = Mutex::new(());
            if mode == "poison" {
                let _ = std::panic::catch_unwind(|| {
                    let _guard = lock.lock().unwrap();
                    panic!("injected I/O worker failure");
                });
            }
            let result = std::panic::catch_unwind(|| {
                close_descriptor(7, descriptor.into(), &lock, |request| {
                    if mode == "poison" {
                        assert!(matches!(request, Request::Detach { handle: 7 }));
                        Ok(())
                    } else {
                        assert!(matches!(request, Request::Close { handle: 7 }));
                        assert_ne!(mode, "unwind", "injected close callback panic");
                        Err(io::ErrorKind::PermissionDenied.into())
                    }
                })
            });
            if mode == "unwind" {
                assert!(result.is_err());
            } else {
                let error = result.unwrap().unwrap_err();
                if mode == "poison" {
                    assert_eq!(error.raw_os_error(), Some(libc::EIO));
                } else {
                    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
                }
            }
            assert_eq!(peer.read(&mut [0; 1]).unwrap(), 0, "descriptor leaked");
        }
    }

    #[test]
    fn remote_error_retains_operation_and_filesystem_errno() {
        for (code, message, errno, retryable, operation) in [
            (
                "BRANCH_NOT_ACTIVE",
                "merged file branch is read-only",
                libc::EROFS,
                false,
                None,
            ),
            (
                "FILE_WRITE_BUSY",
                "another writer owns this file",
                libc::EBUSY,
                true,
                Some("op_pending"),
            ),
            (
                "FILE_BRANCH_CONFLICT",
                "file branch changed after the workspace began",
                libc::EAGAIN,
                false,
                None,
            ),
            (
                "DIRECTORY_BRANCH_CONFLICT",
                "directory branch changed after the workspace began",
                libc::EAGAIN,
                false,
                None,
            ),
        ] {
            let (one, two) = UnixStream::pair().unwrap();
            let server = std::thread::spawn(move || {
                let mut stream = FrameStream::new(two).unwrap();
                let (_request, fd) = stream.receive::<Request>().unwrap();
                assert!(fd.is_none());
                stream
                    .send(
                        &Response::Error {
                            code: code.into(),
                            message: message.into(),
                            retryable,
                            operation_id: operation.map(str::to_owned),
                        },
                        None,
                    )
                    .unwrap();
            });
            let client = Client(Arc::new(Mutex::new(Some(FrameStream::new(one).unwrap()))));
            let error = client
                .call(Request::Stat {
                    path: "report.txt".into(),
                })
                .unwrap_err();
            assert_eq!(error_errno(&error), errno);
            let remote = error
                .get_ref()
                .unwrap()
                .downcast_ref::<RemoteError>()
                .unwrap();
            assert_eq!(remote.operation_id.as_deref(), operation);
            assert_eq!(remote.retryable, retryable);
            assert!(error.to_string().contains(code));
            server.join().unwrap();
        }
    }

    #[test]
    fn invalid_frame_closes_every_clone_before_a_stale_success_can_be_reused() {
        use std::io::Write;
        let (one, mut two) = UnixStream::pair().unwrap();
        // Queue the invalid header and a well-formed response together. Merely
        // returning the parse error would leave that success for the next call.
        let stale = serde_json::to_vec(&Response::Ok {
            result: serde_json::json!({"stale": true}),
        })
        .unwrap();
        let mut frames = Vec::new();
        frames.extend_from_slice(&0_u32.to_be_bytes());
        frames.extend_from_slice(&(stale.len() as u32).to_be_bytes());
        frames.extend_from_slice(&stale);
        two.write_all(&frames).unwrap();
        let client = Client(Arc::new(Mutex::new(Some(FrameStream::new(one).unwrap()))));
        let clone = client.clone();
        assert_eq!(
            client
                .call(Request::Stat {
                    path: "first".into()
                })
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        for handle in [&clone, &client] {
            assert_eq!(
                handle
                    .call(Request::Stat {
                        path: "second".into()
                    })
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::NotConnected
            );
        }
    }

    #[test]
    fn remote_business_error_preserves_the_stream_for_the_next_call() {
        let (one, two) = UnixStream::pair().unwrap();
        let server = std::thread::spawn(move || {
            let mut stream = FrameStream::new(two).unwrap();
            stream.receive::<Request>().unwrap();
            stream
                .send(
                    &Response::Error {
                        code: "ENTRY_NOT_FOUND".into(),
                        message: "missing".into(),
                        operation_id: None,
                        retryable: false,
                    },
                    None,
                )
                .unwrap();
            let (request, _) = stream.receive::<Request>().unwrap();
            assert!(matches!(request, Request::Stat { path } if path == "exists"));
            stream
                .send(
                    &Response::Ok {
                        result: serde_json::json!({"found": true}),
                    },
                    None,
                )
                .unwrap();
        });
        let client = Client(Arc::new(Mutex::new(Some(FrameStream::new(one).unwrap()))));
        let clone = client.clone();
        assert_eq!(
            error_errno(
                &client
                    .call(Request::Stat {
                        path: "missing".into()
                    })
                    .unwrap_err()
            ),
            libc::ENOENT
        );
        assert_eq!(
            clone
                .call(Request::Stat {
                    path: "exists".into()
                })
                .unwrap()
                .0["found"],
            true
        );
        server.join().unwrap();
    }

    #[test]
    fn send_failure_also_disconnects_all_clones() {
        let (one, two) = UnixStream::pair().unwrap();
        drop(two);
        let client = Client(Arc::new(Mutex::new(Some(FrameStream::new(one).unwrap()))));
        let clone = client.clone();
        assert!(
            client
                .call(Request::Stat {
                    path: "first".into()
                })
                .is_err()
        );
        assert_eq!(
            clone
                .call(Request::Stat {
                    path: "second".into()
                })
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotConnected
        );
    }
}
