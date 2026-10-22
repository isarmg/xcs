//! zxs HTTP boundary. Storage semantics stay in the synchronous core backend.
pub mod auth;
#[cfg(feature = "core")]
pub mod engine_backend;
mod file_manager;
mod http_auth;
mod http_boundary;
pub use http_boundary::ApiError;
pub mod smb_rpc;
mod spool_budget;
mod telemetry;
pub use spool_budget::SpoolLimits;
mod transfer;
mod transfer_limits;
pub use transfer_limits::TransferLimits;
mod uploads;
mod web_assets;

mod backend_action;
pub use backend_action::BackendAction;

use auth::{AuthStore, Identity};
use http_auth::{account_upsert, accounts, authorize, change_password, login, logout, me};
use http_boundary::{ApiPath, ApiQuery};
use http_routing::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, patch, post, put},
};
use json_codec::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use transfer::{checksums, content, upload, upload_status, workspace_content};

/// A successful content open must hold its core read lease until the body drops.
pub struct Content {
    pub fallback_files: Vec<std::fs::File>,
    pub checksum_manifest: Option<storage_contract::ChecksumManifest>,
    pub file: std::fs::File,
    pub size: u64,
    pub hash: String,
    pub lease: Box<dyn Send + Sync>,
    pub download_name: Option<String>,
}
impl Content {
    /// Raw descriptor consumers require a fully verified replica before handoff.
    /// Range consumers instead verify each emitted chunk from the same buffer.
    pub fn fully_verified(mut self) -> Result<Self, ApiError> {
        let Some(manifest) = self.checksum_manifest.as_ref() else {
            return Ok(self);
        };
        loop {
            if verify_content_file(&mut self.file, manifest, self.size, &self.hash).is_ok() {
                self.checksum_manifest = None;
                self.fallback_files.clear();
                return Ok(self);
            }
            let Some(next) = self.fallback_files.pop() else {
                return Err(ApiError::new(
                    503,
                    "CONTENT_UNAVAILABLE",
                    "no fully verified content replica is available",
                ));
            };
            self.file = next;
        }
    }
}

fn verify_content_file(
    file: &mut std::fs::File,
    manifest: &storage_contract::ChecksumManifest,
    size: u64,
    hash: &str,
) -> std::io::Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    let corrupt = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "content verification failed",
        )
    };
    if !storage_contract::SUPPORTED_CHECKSUM_CHUNK_BYTES.contains(&manifest.chunk_bytes)
        || manifest.logical_size != size
        || file.metadata()?.len() != size
        || manifest.chunks.len() as u64 != size.div_ceil(manifest.chunk_bytes as u64)
    {
        return Err(corrupt());
    }
    file.seek(SeekFrom::Start(0))?;
    let mut whole = blake3::Hasher::new();
    let mut remaining = size;
    let mut buffer = vec![0; manifest.chunk_bytes as usize];
    for expected in &manifest.chunks {
        let length = remaining.min(manifest.chunk_bytes as u64) as usize;
        file.read_exact(&mut buffer[..length])?;
        if blake3::hash(&buffer[..length]).as_bytes() != expected {
            return Err(corrupt());
        }
        whole.update(&buffer[..length]);
        remaining -= length as u64;
    }
    let mut extra = [0u8; 1];
    if file.read(&mut extra)? != 0
        || file.metadata()?.len() != size
        || whole.finalize().to_hex().as_str() != hash
    {
        return Err(corrupt());
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(())
}

/// Capability for the service's private, read-only disk sampler.
///
/// External adapters may implement the observation hook, but cannot construct
/// this request. It is not a user identity and has no public dispatch action.
pub struct InternalDiskObservation(());

pub trait Backend: Send + Sync {
    fn call(
        &self,
        action: crate::BackendAction,
        identity: &Identity,
        input: Value,
    ) -> Result<Value, ApiError>;
    fn upload(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, ApiError>;
    /// Return a fully verified raw descriptor (used by SMB and local consumers).
    fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError>;
    /// HTTP may authenticate only requested chunks, while retaining the read lease.
    fn range_content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.content(identity, version)
    }
    fn upload_generation(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
        _generation: u64,
    ) -> Result<Value, ApiError> {
        self.upload(identity, workspace, item, path, source)
    }
    fn workspace_content(
        &self,
        _identity: &Identity,
        _workspace: &str,
        _item: &str,
    ) -> Result<Content, ApiError> {
        Err(ApiError::unsupported())
    }
    fn checksums(&self, _identity: &Identity, _version: &str) -> Result<Vec<u8>, ApiError> {
        Err(ApiError::unsupported())
    }
    /// Internal monitoring only. Protocol requests must use authenticated
    /// actions instead; support is opt-in and never derived from an identity.
    fn observe_disks(&self, _: &InternalDiskObservation) -> Result<Value, ApiError> {
        Err(ApiError::unsupported())
    }
}

/// Re-check current account status even for trusted local protocol adapters.
struct AuthorizedBackend {
    backend: Arc<dyn Backend>,
    auth: Arc<Mutex<AuthStore>>,
}
impl AuthorizedBackend {
    fn current(&self, identity: &Identity) -> Result<Identity, ApiError> {
        lock(&self.auth)?.identity_for_principal(&identity.principal)
    }
}
impl Backend for AuthorizedBackend {
    fn call(
        &self,
        action: crate::BackendAction,
        identity: &Identity,
        input: Value,
    ) -> Result<Value, ApiError> {
        if matches!(
            action,
            crate::BackendAction::Capabilities | crate::BackendAction::Ready
        ) {
            self.backend.call(action, identity, input)
        } else {
            self.backend.call(action, &self.current(identity)?, input)
        }
    }
    fn upload(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, ApiError> {
        self.backend
            .upload(&self.current(identity)?, workspace, item, path, source)
    }
    fn upload_generation(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
        generation: u64,
    ) -> Result<Value, ApiError> {
        self.backend.upload_generation(
            &self.current(identity)?,
            workspace,
            item,
            path,
            source,
            generation,
        )
    }
    fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.backend.content(&self.current(identity)?, version)
    }
    fn range_content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.backend
            .range_content(&self.current(identity)?, version)
    }
    fn workspace_content(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
    ) -> Result<Content, ApiError> {
        self.backend
            .workspace_content(&self.current(identity)?, workspace, item)
    }
    fn checksums(&self, identity: &Identity, version: &str) -> Result<Vec<u8>, ApiError> {
        self.backend.checksums(&self.current(identity)?, version)
    }
}

#[derive(Clone)]
pub struct SmbConfig {
    pub socket: PathBuf,
    pub allowed_uid: u32,
    pub users: BTreeMap<String, String>,
}
#[derive(Clone)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    /// Required when TLS terminates upstream. Origin comparison never uses forwarded headers.
    pub external_origin: Option<String>,
    /// Exact loopback proxy peers allowed to supply a single, overwritten X-Forwarded-For IP.
    pub trusted_proxy_ips: Vec<std::net::IpAddr>,
    pub spool_dir: PathBuf,
    pub web_dir: Option<PathBuf>,
    pub max_upload_bytes: u64,
    /// HTTP spool only; independent of engine workspace and SMB storage.
    pub http_spool_limits: SpoolLimits,
    pub http_transfer_limits: TransferLimits,
    pub smb: Option<SmbConfig>,
}
impl ServerConfig {
    pub fn loopback(root: &Path) -> Self {
        Self {
            listen: "127.0.0.1:7400".parse().unwrap(),
            external_origin: None,
            trusted_proxy_ips: Vec::new(),
            spool_dir: root.join("api-spool"),
            web_dir: None,
            max_upload_bytes: 1024 * 1024 * 1024,
            http_spool_limits: SpoolLimits::default(),
            http_transfer_limits: TransferLimits::default(),
            smb: None,
        }
    }
    fn prepare_spool(&self) -> Result<(), ApiError> {
        std::fs::create_dir_all(&self.spool_dir).map_err(ApiError::io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.spool_dir, std::fs::Permissions::from_mode(0o700))
                .map_err(ApiError::io)?;
        }
        Ok(())
    }
    fn secure_cookie(&self) -> bool {
        self.normalized_external_origin()
            .is_ok_and(|origin| origin.is_some())
    }
    /// Match browser Origin serialization, including host case, default ports
    /// and IPv6 compression. Validation and cookie policy share this boundary.
    fn normalized_external_origin(&self) -> Result<Option<String>, ApiError> {
        self.external_origin
            .as_ref()
            .map(|origin| {
                let parsed =
                    url_encoding::address::Url::parse(origin).map_err(ApiError::invalid)?;
                if parsed.scheme() != "https"
                    || parsed.host_str().is_none()
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                    || parsed.path() != "/"
                    || parsed.query().is_some()
                    || parsed.fragment().is_some()
                    || origin.ends_with('/')
                {
                    return Err(ApiError::invalid(
                        "external origin must be an HTTPS origin without trailing slash",
                    ));
                }
                Ok(parsed.origin().ascii_serialization())
            })
            .transpose()
    }
    fn validate(&self) -> Result<(), ApiError> {
        if !self.listen.ip().is_loopback() {
            return Err(ApiError::invalid(
                "API must listen on loopback; publish through a TLS terminator",
            ));
        }
        if !self.trusted_proxy_ips.is_empty()
            && (self.external_origin.is_none()
                || self.trusted_proxy_ips.iter().any(|ip| !ip.is_loopback()))
        {
            return Err(ApiError::invalid(
                "trusted proxies must be exact loopback IPs and require an HTTPS external origin",
            ));
        }
        self.http_spool_limits.validate()?;
        self.http_transfer_limits.validate()?;
        if self.max_upload_bytes == 0 {
            return Err(ApiError::invalid("upload limit must be positive"));
        }
        if self.smb.as_ref().is_some_and(|smb| smb.users.is_empty()) {
            return Err(ApiError::invalid(
                "SMB requires explicit authenticated-user mappings",
            ));
        }
        self.normalized_external_origin()?;
        Ok(())
    }
}

// A conservative, per-router bound for general HTTP backend jobs. This is
// independent of content/upload admission and is not a process-wide I/O budget.
const HTTP_BACKEND_SLOTS: usize = 8;

#[derive(Clone)]
struct AppState {
    backend: Arc<dyn Backend>,
    auth: Arc<Mutex<AuthStore>>,
    config: ServerConfig,
    attempts: Arc<Mutex<HashMap<String, http_auth::Attempts>>>,
    pending_logins: Arc<Mutex<std::collections::HashSet<String>>>,
    password_slots: Arc<tokio::sync::Semaphore>,
    backend_slots: Arc<tokio::sync::Semaphore>,
    upload_slots: Arc<transfer_limits::Admission>,
    read_slots: Arc<transfer_limits::Admission>,
    upload_claims: Arc<Mutex<std::collections::BTreeSet<String>>>,
    spool_budget: Arc<spool_budget::SpoolBudget>,
    static_etags: Arc<BTreeMap<String, String>>,
    telemetry: Arc<telemetry::Telemetry>,
}
fn lock<T>(value: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, ApiError> {
    value
        .lock()
        .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "service state requires restart"))
}

pub fn router(
    backend: Arc<dyn Backend>,
    auth: AuthStore,
    config: ServerConfig,
) -> Result<Router, ApiError> {
    config.validate()?;
    config.prepare_spool()?;
    let telemetry = telemetry::Telemetry::start(
        backend.clone(),
        &config
            .spool_dir
            .parent()
            .ok_or_else(|| ApiError::invalid("spool directory requires a parent"))?
            .join("telemetry-cache"),
    )?;
    let auth = Arc::new(Mutex::new(auth));
    let backend = Arc::new(AuthorizedBackend {
        backend,
        auth: auth.clone(),
    });
    router_shared(backend, auth, config, telemetry)
}
fn router_shared(
    backend: Arc<dyn Backend>,
    auth: Arc<Mutex<AuthStore>>,
    config: ServerConfig,
    telemetry: Arc<telemetry::Telemetry>,
) -> Result<Router, ApiError> {
    config.validate()?;
    let state = AppState {
        backend,
        auth,
        telemetry,
        config: config.clone(),
        spool_budget: spool_budget::SpoolBudget::recover(
            &config.spool_dir,
            config.http_spool_limits.clone(),
        )?,
        attempts: Arc::new(Mutex::new(HashMap::new())),
        pending_logins: Arc::new(Mutex::new(std::collections::HashSet::new())),
        password_slots: Arc::new(tokio::sync::Semaphore::new(4)),
        backend_slots: Arc::new(tokio::sync::Semaphore::new(HTTP_BACKEND_SLOTS)),
        upload_slots: transfer_limits::Admission::new(&config.http_transfer_limits),
        read_slots: transfer_limits::Admission::new(&config.http_transfer_limits),
        upload_claims: Arc::new(Mutex::new(Default::default())),
        static_etags: Arc::new(match &config.web_dir {
            Some(root) => web_assets::static_etags(root)?,
            None => BTreeMap::new(),
        }),
    };
    let protected = Router::new()
        .merge(file_manager::routes())
        .route("/api/system/status", get(telemetry::status))
        .route("/api/system/history", get(telemetry::history))
        .route("/api/auth/me", get(me))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/password", post(change_password))
        .route("/api/admin/accounts", get(accounts).post(account_upsert))
        .route(
            "/api/repositories",
            get(repositories).post(create_repository),
        )
        .route("/api/repositories/{repo}", get(repository))
        .route("/api/repositories/{repo}/grants", put(grants))
        .route(
            "/api/repositories/{repo}/smb-binding",
            get(smb_binding).put(update_smb_binding),
        )
        .route(
            "/api/repositories/{repo}/default-branch",
            get(default_branch).put(update_default_branch),
        )
        .route(
            "/api/repositories/{repo}/directory-scopes",
            post(initialize_directory_scope),
        )
        .route(
            "/api/repositories/{repo}/directory-branches",
            get(directory_branch_scopes),
        )
        .route(
            "/api/repositories/{repo}/directory-branches/{id}",
            patch(rename_directory_branch),
        )
        .route(
            "/api/repositories/{repo}/branches",
            get(branches).post(create_branch),
        )
        .route("/api/branches/{branch}/tree", get(tree))
        .route("/api/branches/{branch}", get(branch_info))
        .route("/api/branches/{branch}/resolve", post(resolve))
        .route("/api/branches/{branch}/write-sessions", post(write_session))
        .route("/api/branches/{branch}/archive", post(archive))
        .route("/api/workspaces", post(workspace))
        .route("/api/workspaces/{workspace}", get(workspace_info))
        .route(
            "/api/workspaces/{workspace}/files/{item}/content",
            put(upload).get(workspace_content).head(workspace_content),
        )
        .route(
            "/api/workspaces/{workspace}/files/{item}/upload-status",
            get(upload_status),
        )
        .route(
            "/api/workspaces/{workspace}/namespace-operations",
            post(namespace),
        )
        .route("/api/workspaces/{workspace}/commit", post(commit))
        .route("/api/workspaces/{workspace}/seal", post(seal))
        .route("/api/workspaces/{workspace}/abort", post(abort))
        .route("/api/operations/{operation}", get(operation))
        .route(
            "/api/versions/{version}/content",
            get(content).head(content),
        )
        .route("/api/versions/{version}", get(version))
        .route("/api/versions/{version}/placement", get(placement))
        .route("/api/versions/{version}/checksums", get(checksums))
        .route("/api/files/{file}/history", get(history))
        .route("/api/commits/{commit}", get(commit_info))
        .route("/api/history-trees/{history}/graph", get(graph))
        .route("/api/history-trees/{history}/merge-links", post(merge_link))
        .route("/api/history-trees/{history}/purge-plans", post(purge_plan))
        .route("/api/history-trees/{history}/split-plans", post(split_plan))
        .route(
            "/api/history-trees/{history}/historical-save-plans",
            post(historical_save_plan),
        )
        .route("/api/plans/{plan}/commit", post(plan_commit))
        .route("/api/plans/{plan}", get(plan_info))
        .route("/api/plans/{plan}/cancel", post(plan_cancel))
        .route("/api/admin/disks", get(disks).post(register_disk))
        .route("/api/admin/check", get(check))
        .route("/api/backup-topologies/{topology}", get(topology))
        .route("/api/policies/{policy}", get(policy).put(update_policy))
        .route(
            "/api/primary-placement-rules/{rule}",
            get(placement_rule).put(update_placement_rule),
        )
        .route("/api/events", get(events))
        .route("/api/compare", post(compare))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize));
    let mut app = Router::new()
        .merge(protected)
        .route(
            "/healthz",
            get(|| async { Json(json!({"status":"alive"})) }),
        )
        .route("/readyz", get(ready))
        .route("/api/capabilities", get(capabilities))
        .route("/api/auth/login", post(login))
        .method_not_allowed_fallback(http_boundary::method_not_allowed)
        .fallback(http_boundary::not_found);
    if let Some(web_dir) = config.web_dir {
        app = app.fallback_service(web_assets::fallback(web_dir));
    }
    Ok(app
        .layer(DefaultBodyLimit::disable())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            web_assets::static_validation,
        ))
        .layer(middleware::from_fn(security_headers))
        .with_state(state))
}

pub async fn serve(
    backend: Arc<dyn Backend>,
    auth: AuthStore,
    config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    config.validate()?;
    config.prepare_spool()?;
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let telemetry = telemetry::Telemetry::start(
        backend.clone(),
        &config
            .spool_dir
            .parent()
            .ok_or_else(|| ApiError::invalid("spool directory requires a parent"))?
            .join("telemetry-cache"),
    )?;
    let auth = Arc::new(Mutex::new(auth));
    let backend: Arc<dyn Backend> = Arc::new(AuthorizedBackend {
        backend,
        auth: auth.clone(),
    });
    let smb = config.smb.clone();
    let mut identities = BTreeMap::new();
    if let Some(smb) = &smb {
        for (alias, principal) in &smb.users {
            identities.insert(
                alias.clone(),
                lock(&auth)?.identity_for_principal(principal)?,
            );
        }
        let parent = smb
            .socket
            .parent()
            .ok_or("SMB socket needs a parent directory")?;
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
            }
        }
    }
    let app = router_shared(backend.clone(), auth, config, telemetry)?;
    let http = async move {
        http_routing::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown())
        .await
    };
    if let Some(smb) = smb {
        tokio::select! {result=http=>result?,result=smb_rpc::serve(&smb.socket,smb.allowed_uid,identities,backend)=>result?}
    } else {
        http.await?;
    }
    Ok(())
}
async fn shutdown() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut term) => {
                tokio::select! {_=tokio::signal::ctrl_c()=>{},_=term.recv()=>{}}
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn security_headers(request: Request, next: Next) -> Response {
    let sensitive = http_boundary::is_api_path(request.uri().path())
        || matches!(request.uri().path(), "/healthz" | "/readyz");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if sensitive { "no-store" } else { "no-cache" }),
    );
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; font-src 'self'; img-src 'self' data: blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    response
}

async fn call(
    state: AppState,
    identity: Identity,
    action: crate::BackendAction,
    input: Value,
) -> Result<Json<Value>, ApiError> {
    let permit = state
        .backend_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::limit("HTTP backend capacity reached"))?;
    tokio::task::spawn_blocking(move || {
        // The job, including time queued in Tokio, owns admission. Dropping the
        // HTTP future must not release capacity while blocking work continues.
        let _permit = permit;
        state.backend.call(action, &identity, input)
    })
    .await
    .map_err(ApiError::join)?
    .map(Json)
}
async fn json_body(request: Request) -> Result<Value, ApiError> {
    let bytes = http_boundary::read_body(request, 1024 * 1024).await?;
    if bytes.is_empty() {
        Ok(json!({}))
    } else {
        storage_contract::strict_json(&bytes).map_err(ApiError::invalid)
    }
}
fn with_id(mut body: Value, key: &str, id: String) -> Result<Value, ApiError> {
    let object = body
        .as_object_mut()
        .ok_or_else(|| ApiError::invalid("expected JSON object"))?;
    if object
        .get(key)
        .is_some_and(|value| value.as_str() != Some(&id))
    {
        return Err(ApiError::invalid("path and body identity disagree"));
    }
    object.insert(key.into(), id.into());
    Ok(body)
}

macro_rules! list_handler {
    ($name:ident, $action:expr) => {
        async fn $name(
            State(state): State<AppState>,
            http_routing::Extension(identity): http_routing::Extension<Identity>,
            ApiQuery(query): ApiQuery<BTreeMap<String, String>>,
        ) -> Result<Json<Value>, ApiError> {
            call(state, identity, $action, json!({"query": query})).await
        }
    };
}
macro_rules! read_handler {
    ($name:ident, $action:expr, $key:literal) => {
        async fn $name(
            State(state): State<AppState>,
            http_routing::Extension(identity): http_routing::Extension<Identity>,
            ApiPath(id): ApiPath<String>,
            ApiQuery(query): ApiQuery<BTreeMap<String, String>>,
        ) -> Result<Json<Value>, ApiError> {
            let mut input = json!({"query": query});
            input[$key] = id.into();
            call(state, identity, $action, input).await
        }
    };
}
macro_rules! post_handler {
    ($name:ident, $action:expr) => {
        async fn $name(
            State(state): State<AppState>,
            http_routing::Extension(identity): http_routing::Extension<Identity>,
            request: Request,
        ) -> Result<Json<Value>, ApiError> {
            call(state, identity, $action, json_body(request).await?).await
        }
    };
}
macro_rules! id_post_handler {
    ($name:ident, $action:expr, $id:literal) => {
        async fn $name(
            State(state): State<AppState>,
            http_routing::Extension(identity): http_routing::Extension<Identity>,
            ApiPath(id): ApiPath<String>,
            request: Request,
        ) -> Result<Json<Value>, ApiError> {
            call(
                state,
                identity,
                $action,
                with_id(json_body(request).await?, $id, id)?,
            )
            .await
        }
    };
}
list_handler!(repositories, crate::BackendAction::Repositories);
list_handler!(disks, crate::BackendAction::Disks);
list_handler!(events, crate::BackendAction::Events);
list_handler!(check, crate::BackendAction::Check);
read_handler!(
    repository,
    crate::BackendAction::Repository,
    "repository_id"
);
read_handler!(
    smb_binding,
    crate::BackendAction::SmbBinding,
    "repository_id"
);
read_handler!(
    default_branch,
    crate::BackendAction::DefaultBranch,
    "repository_id"
);
read_handler!(branches, crate::BackendAction::Branches, "repository_id");
read_handler!(branch_info, crate::BackendAction::Branch, "branch_id");
read_handler!(tree, crate::BackendAction::Tree, "branch_id");
read_handler!(operation, crate::BackendAction::Operation, "operation_id");
read_handler!(version, crate::BackendAction::Version, "version_id");
read_handler!(placement, crate::BackendAction::Placement, "version_id");
read_handler!(history, crate::BackendAction::History, "file_id");
read_handler!(commit_info, crate::BackendAction::Commit, "commit_id");
read_handler!(graph, crate::BackendAction::Graph, "history_tree_id");
read_handler!(topology, crate::BackendAction::Topology, "topology_id");
read_handler!(
    workspace_info,
    crate::BackendAction::Workspace,
    "workspace_id"
);
read_handler!(plan_info, crate::BackendAction::Plan, "plan_id");
read_handler!(policy, crate::BackendAction::Policy, "policy_id");
read_handler!(
    placement_rule,
    crate::BackendAction::PlacementRule,
    "rule_id"
);
read_handler!(
    directory_branch_scopes,
    crate::BackendAction::DirectoryBranchScopes,
    "repository_id"
);
post_handler!(create_repository, crate::BackendAction::CreateRepository);
post_handler!(workspace, crate::BackendAction::CreateWorkspace);
post_handler!(register_disk, crate::BackendAction::RegisterDisk);
post_handler!(compare, crate::BackendAction::Compare);
id_post_handler!(
    create_branch,
    crate::BackendAction::CreateBranch,
    "repository_id"
);
id_post_handler!(resolve, crate::BackendAction::Resolve, "branch_id");
id_post_handler!(grants, crate::BackendAction::SetGrants, "repository_id");
id_post_handler!(
    update_smb_binding,
    crate::BackendAction::UpdateSmbBinding,
    "repository_id"
);
id_post_handler!(
    initialize_directory_scope,
    crate::BackendAction::InitializeDirectoryScope,
    "repository_id"
);
id_post_handler!(
    update_default_branch,
    crate::BackendAction::UpdateDefaultBranch,
    "repository_id"
);
async fn rename_directory_branch(
    State(state): State<AppState>,
    http_routing::Extension(identity): http_routing::Extension<Identity>,
    ApiPath((repo, id)): ApiPath<(String, String)>,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let input = with_id(json_body(request).await?, "repository_id", repo)?;
    let input = with_id(input, "directory_branch_id", id)?;
    call(
        state,
        identity,
        crate::BackendAction::RenameDirectoryBranch,
        input,
    )
    .await
}
id_post_handler!(
    write_session,
    crate::BackendAction::WriteSession,
    "branch_id"
);
id_post_handler!(archive, crate::BackendAction::Archive, "branch_id");
id_post_handler!(namespace, crate::BackendAction::Namespace, "workspace_id");
id_post_handler!(
    commit,
    crate::BackendAction::CommitWorkspace,
    "workspace_id"
);
id_post_handler!(seal, crate::BackendAction::SealWorkspace, "workspace_id");
id_post_handler!(abort, crate::BackendAction::AbortWorkspace, "workspace_id");
id_post_handler!(
    merge_link,
    crate::BackendAction::MergeLink,
    "history_tree_id"
);
id_post_handler!(
    purge_plan,
    crate::BackendAction::PurgePlan,
    "history_tree_id"
);
id_post_handler!(
    split_plan,
    crate::BackendAction::SplitPlan,
    "history_tree_id"
);
id_post_handler!(
    historical_save_plan,
    crate::BackendAction::HistoricalSavePlan,
    "history_tree_id"
);
id_post_handler!(
    update_policy,
    crate::BackendAction::UpdatePolicy,
    "policy_id"
);
id_post_handler!(
    update_placement_rule,
    crate::BackendAction::UpdatePlacementRule,
    "rule_id"
);
id_post_handler!(plan_commit, crate::BackendAction::PlanCommit, "plan_id");
id_post_handler!(plan_cancel, crate::BackendAction::PlanCancel, "plan_id");

async fn ready(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let identity = Identity {
        principal: "health".into(),
        admin: false,
        repositories: Default::default(),
    };
    call(state, identity, crate::BackendAction::Ready, json!({})).await
}
async fn capabilities(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let identity = Identity {
        principal: "anonymous".into(),
        admin: false,
        repositories: Default::default(),
    };
    let limit = state.config.max_upload_bytes;
    let Json(mut value) = call(
        state,
        identity,
        crate::BackendAction::Capabilities,
        json!({}),
    )
    .await?;
    let core_limit = value
        .get("max_upload_bytes")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(limit);
    value["max_upload_bytes"] = limit.min(core_limit).to_string().into();
    Ok(Json(value))
}

#[cfg(test)]
#[path = "../../../test/server/unit/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../test/server/unit/backend_admission.rs"]
mod backend_admission_tests;
