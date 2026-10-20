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
            username,
            repository_id,
        } = request
        else {
            return Err(ApiError::unauthorized());
        };
        if protocol != PROTOCOL {
            return Err(ApiError::invalid("unsupported SMB bridge protocol"));
        }
        let identity = shared
            .identities
            .get(&username)
            .cloned()
            .ok_or_else(ApiError::forbidden)?;
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
        let max_file_bytes =
            parse_u64_decimal(field(&capabilities, "max_upload_bytes")?).map_err(contract)?;
        Ok(Session {
            shared: shared.clone(),
            identity,
            branch,
            handles: BTreeMap::new(),
            next_handle: 1,
            writer_locks: BTreeSet::new(),
            max_file_bytes,
        })
    })();
    let mut session = match setup {
        Ok(session) => session,
        Err(error) => {
            stream.send(&response_error(error, None), None)?;
            return Ok(());
        }
    };
    stream.send(&Response::Ok{result:json!({"protocol":PROTOCOL,"principal":session.identity.principal,"main_branch_id":session.branch})},None)?;
    loop {
        let (request, fd) = match stream.receive::<Request>() {
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
            Request::Flush { handle } | Request::Close { handle } => {
                match session.handles.get(handle) {
                    Some(Handle::Write(writer)) => Some(writer.operation_id.clone()),
                    _ => None,
                }
            }
            _ => None,
        };
        match session.dispatch(request) {
            Ok((result, fd)) => {
                stream.send(&Response::Ok { result }, fd.as_ref().map(AsFd::as_fd))?
            }
            Err(error) => stream.send(&response_error(error, operation_id), None)?,
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

/// Serve only on an existing service-owned private directory. Existing socket
/// names are never unlinked automatically. The trusted Samba daemon must connect
/// using the configured peer uid before impersonating a remote SMB user.
pub async fn serve(
    socket: &Path,
    allowed_uid: u32,
    identities: BTreeMap<String, Identity>,
    backend: Arc<dyn Backend>,
) -> io::Result<()> {
    if identities.is_empty() || identities.values().any(|identity| identity.admin) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "SMB requires explicit non-administrator user mappings",
        ));
    }
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
    let listener = tokio::net::UnixListener::bind(socket)?;
    let metadata = fs::symlink_metadata(socket)?;
    let connections = Arc::new(Mutex::new(BTreeMap::new()));
    let _socket_guard = SocketGuard {
        path: socket.to_owned(),
        identity: (metadata.dev(), metadata.ino()),
        connections: connections.clone(),
    };
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
    let shared = Arc::new(Shared {
        backend,
        identities,
        spool,
        file_writers: Mutex::new(BTreeSet::new()),
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
