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

/// Account for the mapped principal, not the Samba username or repository:
/// aliases and multiple shares must not bypass the authenticated session cap.
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
        if !capabilities
            .iter()
            .any(|capability| capability == WRITE_RESERVATION_CAPABILITY)
        {
            return Err(ApiError::invalid(
                "SMB bridge requires durable write reservations",
            ));
        }
        let identity = shared
            .identities
            .get(&username)
            .cloned()
            .ok_or_else(ApiError::forbidden)?;
        let permit = SessionPermit::acquire(&shared, &identity.principal)?;
        let repo = shared.backend.call(
            crate::BackendAction::Repository,
            &identity,
            json!({"repository_id":repository_id}),
        )?;
        if repo.get("smb_enabled").and_then(Value::as_bool) == Some(false) {
            return Err(ApiError::new(
                409,
                "SMB_REBIND_REQUIRED",
                "repository requires an explicit SMB binding after history migration",
            ));
        }
        let branch = field(&repo, "main_branch_id")?.to_owned();
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
                "protocol":PROTOCOL,"capabilities":[WRITE_RESERVATION_CAPABILITY],
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
        match session.dispatch(request) {
            Ok((result, fd)) => {
                stream.send(&Response::Ok { result }, fd.as_ref().map(AsFd::as_fd))?
            }
            Err(error) => stream.send(&response_error(error.error, error.operation_id), None)?,
        }
    }
}

fn response_error(error: ApiError, operation_id: Option<String>) -> Response {
    Response::Error {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        operation_id,
    }
}

fn private_directory(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "absolute socket parent required",
        ));
    }
    let mut current = PathBuf::from("/");
    for part in path.components() {
        if !matches!(part, Component::Normal(_) | Component::RootDir) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "noncanonical socket directory path",
            ));
        }
        if let Component::Normal(part) = part {
            current.push(part);
            let metadata = fs::symlink_metadata(&current)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "socket path contains an alias",
                ));
            }
        }
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.uid() != smb_protocol::protocol::effective_uid()
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "socket parent must be service-owned 0700",
        ));
    }
    Ok(())
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
    let spool = parent.join("smb-spool");
    match fs::create_dir(&spool) {
        Ok(()) => fs::set_permissions(&spool, fs::Permissions::from_mode(0o700))?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    };
    private_directory(&spool)?;
    let sentinel = spool.join("empty-directory");
    match fs::create_dir(&sentinel) {
        Ok(()) => fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o700))?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    };
    private_directory(&sentinel)?;
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
    identities: BTreeMap<String, Identity>,
    backend: Arc<dyn Backend>,
    limits: crate::SpoolLimits,
    max_file_bytes: u64,
) -> io::Result<()> {
    if identities.is_empty() || identities.values().any(|identity| identity.admin) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SMB requires explicit non-administrator user mappings",
        ));
    }
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
        identities,
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
