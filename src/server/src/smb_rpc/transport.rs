//! Private socket lifecycle, peer admission and framed connection serving.
use super::*;

struct SocketGuard {
    path: PathBuf,
    identity: (u64, u64),
    connections: Arc<Mutex<BTreeMap<u64, std::os::unix::net::UnixStream>>>,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if let Ok(mut connections) = self.connections.lock() {
            for stream in connections.values() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
            connections.clear();
        }
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && (metadata.dev(), metadata.ino()) == self.identity
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}
struct ConnectionGuard {
    id: u64,
    connections: Arc<Mutex<BTreeMap<u64, std::os::unix::net::UnixStream>>>,
}
impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if let Ok(mut connections) = self.connections.lock() {
            connections.remove(&self.id);
        }
    }
}

/// Multiple shares must not bypass the sole account's authenticated session cap.
struct SessionPermit {
    shared: Arc<Shared>,
    principal: String,
}
impl SessionPermit {
    fn acquire(shared: &Arc<Shared>, principal: &str) -> Result<Self, ApiError> {
        let mut sessions = shared
            .sessions_per_account
            .lock()
            .map_err(|_| ApiError::new(503, "SMB_UNAVAILABLE", "SMB session registry poisoned"))?;
        let count = sessions.entry(principal.to_owned()).or_default();
        if *count >= MAX_SESSIONS_PER_ACCOUNT {
            return Err(ApiError::limit("too many SMB sessions for this account"));
        }
        *count += 1;
        Ok(Self {
            shared: shared.clone(),
            principal: principal.to_owned(),
        })
    }
}
impl Drop for SessionPermit {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.shared.sessions_per_account.lock()
            && let Some(count) = sessions.get_mut(&self.principal)
        {
            if *count > 1 {
                *count -= 1;
            } else {
                sessions.remove(&self.principal);
            }
        }
    }
}

pub(super) fn run_connection(mut stream: FrameStream, shared: Arc<Shared>) -> io::Result<()> {
    let (request, fd) = stream.receive::<Request>()?;
    if fd.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "client descriptors forbidden",
        ));
    }
    let setup = (|| {
        let Request::Hello {
            protocol,
            capabilities,
            username,
            repository_id,
        } = request
        else {
            return Err(ApiError::unauthorized());
        };
        if protocol != PROTOCOL {
            return Err(ApiError::invalid("unsupported SMB bridge protocol"));
        }
        if !REQUIRED_CAPABILITIES
            .iter()
            .all(|required| capabilities.iter().any(|value| value == required))
        {
            return Err(ApiError::invalid(
                "SMB bridge requires authenticated single-account sessions and durable write reservations",
            ));
        }
        if username != shared.username {
            return Err(ApiError::forbidden());
        }
        let mut identity = crate::lock(&shared.auth)?.sole_identity()?;
        let permit = SessionPermit::acquire(&shared, &identity.principal)?;
        let repo = shared.backend.call(
            crate::BackendAction::Repository,
            &identity,
            json!({"repository_id":repository_id}),
        )?;
        if repo.get("smb_enabled").and_then(Value::as_bool) != Some(true) {
            return Err(ApiError::new(
                409,
                "SMB_REBIND_REQUIRED",
                "repository requires an explicit SMB binding after history migration",
            ));
        }
        let branch = field(&repo, "main_branch_id")?.to_owned();
        identity.smb_binding = Some(crate::auth::SmbBinding {
            repository_id,
            branch_id: branch.clone(),
            generation: parse_u64_decimal(field(&repo, "smb_binding_generation")?)
                .map_err(contract)?,
        });
        let capabilities =
            shared
                .backend
                .call(crate::BackendAction::Capabilities, &identity, json!({}))?;
        let max_file_bytes = parse_u64_decimal(field(&capabilities, "max_upload_bytes")?)
            .map_err(contract)?
            .min(shared.max_file_bytes);
        Ok((
            Session {
                shared: shared.clone(),
                identity,
                branch,
                handles: BTreeMap::new(),
                next_handle: 1,
                writer_locks: BTreeSet::new(),
                max_file_bytes,
            },
            permit,
        ))
    })();
    let (mut session, _permit) = match setup {
        Ok(session) => session,
        Err(error) => {
            stream.send(&response_error(error, None), None)?;
            return Ok(());
        }
    };
    stream.send(
        &Response::Ok {
            result: json!({
                "protocol":PROTOCOL,"capabilities":REQUIRED_CAPABILITIES,
                "principal":session.identity.principal,"main_branch_id":session.branch
            }),
        },
        None,
    )?;
    loop {
        let (request, fd) = match stream
            .receive_with_mode::<Request>(smb_protocol::protocol::ReadMode::EstablishedSession)
        {
            Ok(request) => request,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(error),
        };
        if fd.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "client descriptors forbidden",
            ));
        }
        let operation_id = match &request {
            Request::Namespace { operation_id, .. } => Some(operation_id.clone()),
            _ => None,
        };
        match session.dispatch(request) {
            Ok((result, fd)) => send_success(&mut stream, result, fd, operation_id)?,
            Err(error) => stream.send(&response_error(error.error, error.operation_id), None)?,
        }
    }
}

// A local response-size failure has not written any frame bytes. Report it as
// a business error instead of tearing down unrelated handles on this stream.
fn send_success(
    stream: &mut FrameStream,
    result: Value,
    descriptor: Option<OwnedFd>,
    requested_operation: Option<String>,
) -> io::Result<()> {
    let operation_id = result
        .get("operation_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or(requested_operation);
    let response = Response::Ok { result };
    let payload = json_codec::to_vec(&response).map_err(io::Error::other)?;
    if payload.len() > smb_protocol::protocol::MAX_FRAME_BYTES {
        drop(payload);
        return stream.send(&Response::Error {
            code: "RESPONSE_TOO_LARGE".into(),
            message: "result exceeds RPC frame limit; the operation may have completed, query its original identity rather than replaying".into(),
            retryable: false,
            operation_id,
        }, None);
    }
    stream.send_serialized(&payload, descriptor.as_ref().map(AsFd::as_fd))
}

fn response_error(error: ApiError, operation_id: Option<String>) -> Response {
    Response::Error {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        operation_id,
    }
}

fn private_path_error(error: platform_io::fs_safety::Error) -> io::Error {
    use platform_io::fs_safety::Error;
    match error {
        Error::Io(error)
            if error.kind() == io::ErrorKind::NotADirectory
                || error.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error()) =>
        {
            io::Error::new(io::ErrorKind::PermissionDenied, error)
        }
        Error::Io(error) => error,
        error @ (Error::AbsolutePathRequired(_) | Error::UnsafeRelativePath(_)) => {
            io::Error::new(io::ErrorKind::InvalidInput, error)
        }
        error => io::Error::new(io::ErrorKind::PermissionDenied, error),
    }
}
fn private_directory(path: &Path) -> io::Result<()> {
    platform_io::fs_safety::PrivateDirectory::open_trusted(path)
        .map(|_| ())
        .map_err(private_path_error)
}

struct PreparedSocket {
    listener: std::os::unix::net::UnixListener,
    guard: SocketGuard,
    connections: Arc<Mutex<BTreeMap<u64, std::os::unix::net::UnixStream>>>,
    spool: PathBuf,
    budget: Arc<budget::Budget>,
}

// Keep all path traversal, creation, bind/chmod and budget recovery on a
// blocking worker. Only Tokio reactor registration belongs on the async side.
fn prepare_socket(socket: &Path, limits: crate::SpoolLimits) -> io::Result<PreparedSocket> {
    let parent = socket
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket parent missing"))?;
    private_directory(parent)?;
    use platform_io::fs_safety::{EntryName, PrivateDirectory};
    let parent_handle = PrivateDirectory::open_trusted(parent).map_err(private_path_error)?;
    let spool_handle = parent_handle
        .create_child(&EntryName::new("smb-spool").map_err(private_path_error)?)
        .map_err(private_path_error)?;
    let spool = spool_handle.path().to_path_buf();
    spool_handle
        .create_child(&EntryName::new("empty-directory").map_err(private_path_error)?)
        .map_err(private_path_error)?;
    let listener = std::os::unix::net::UnixListener::bind(socket)?;
    let metadata = fs::symlink_metadata(socket)?;
    let connections = Arc::new(Mutex::new(BTreeMap::new()));
    let guard = SocketGuard {
        path: socket.to_owned(),
        identity: (metadata.dev(), metadata.ino()),
        connections: connections.clone(),
    };
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    let budget = budget::Budget::recover(&spool, limits).map_err(io::Error::other)?;
    Ok(PreparedSocket {
        listener,
        guard,
        connections,
        spool,
        budget,
    })
}

async fn startup_io<T: Send + 'static>(
    work: impl FnOnce() -> io::Result<T> + Send + 'static,
) -> io::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(io::Error::other)?
}

/// Serve only on an existing service-owned private directory. Existing socket
/// names are never unlinked automatically. The trusted Samba daemon must connect
/// using the configured peer uid before impersonating a remote SMB user.
pub async fn serve(
    socket: &Path,
    allowed_uid: u32,
    username: String,
    auth: Arc<Mutex<AuthStore>>,
    backend: Arc<dyn Backend>,
    limits: crate::SpoolLimits,
    max_file_bytes: u64,
) -> io::Result<()> {
    smb_protocol::protocol::validate_username(&username)?;
    let socket = socket.to_owned();
    let PreparedSocket {
        listener,
        guard: _socket_guard,
        connections,
        spool,
        budget,
    } = startup_io(move || prepare_socket(&socket, limits)).await?;
    // The descriptor is already nonblocking and mode 0600 before registration;
    // no connection is accepted until preparation and budget recovery succeed.
    let listener = tokio::net::UnixListener::from_std(listener)?;
    let shared = Arc::new(Shared {
        budget,
        max_file_bytes,
        sessions_per_account: Mutex::new(BTreeMap::new()),
        backend,
        username,
        auth,
        spool,
        file_writers: Mutex::new(BTreeSet::new()),
        namespace_operations: Mutex::new(BTreeSet::new()),
    });
    let admission = Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
    let mut next_connection = 1u64;
    loop {
        let (stream, _) = listener.accept().await?;
        if stream.peer_cred()?.uid() != allowed_uid {
            drop(stream);
            continue;
        }
        let Ok(permit) = admission.clone().try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let stream = stream.into_std()?;
        stream.set_nonblocking(false)?;
        let id = next_connection;
        next_connection = next_connection
            .checked_add(1)
            .ok_or_else(|| io::Error::other("SMB connection identifiers exhausted"))?;
        connections
            .lock()
            .map_err(|_| io::Error::other("SMB connection registry poisoned"))?
            .insert(id, stream.try_clone()?);
        let connection_guard = ConnectionGuard {
            id,
            connections: connections.clone(),
        };
        let shared = shared.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _connection_guard = connection_guard;
            match FrameStream::new(stream).and_then(|stream| run_connection(stream, shared)) {
                Ok(()) => {}
                Err(error) => tracing::warn!(%error,"SMB local RPC session ended"),
            }
        });
    }
}

#[cfg(test)]
#[path = "../../../../test/server/unit/smb_rpc/startup_tests.rs"]
mod startup_tests;
