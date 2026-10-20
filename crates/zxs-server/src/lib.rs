//! zxs HTTP boundary. Storage semantics stay in the synchronous core backend.
pub mod auth;
#[cfg(feature = "core")]
pub mod engine_backend;
mod file_manager;
mod http_auth;
pub mod smb_rpc;
mod telemetry;
mod transfer;
mod uploads;
mod web_assets;

mod backend_action;
pub use backend_action::BackendAction;

use auth::{AuthStore, Identity};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path as RoutePath, Query, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
};
use http_auth::{account_upsert, accounts, authorize, change_password, login, logout, me};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use transfer::{checksums, content, upload, upload_status, workspace_content};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub request_id: String,
    #[serde(skip)]
    pub status: u16,
}
impl ApiError {
    pub fn new(status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            retryable: (status >= 500 && status != 501) || status == 429,
            request_id: zxs_contract::new_id("req"),
        }
    }
    pub fn invalid(error: impl std::fmt::Display) -> Self {
        Self::new(400, "INVALID_REQUEST", error.to_string())
    }
    pub fn io(error: std::io::Error) -> Self {
        tracing::error!(%error, "local I/O failed");
        Self::new(500, "IO_ERROR", "local storage operation failed")
    }
    pub fn unauthorized() -> Self {
        Self::new(
            401,
            "UNAUTHORIZED",
            "authentication required or session expired",
        )
    }
    pub fn forbidden() -> Self {
        Self::new(403, "FORBIDDEN", "permission denied")
    }
    pub fn conflict(code: &str, message: &str) -> Self {
        Self::new(409, code, message)
    }
    pub fn limit(message: &str) -> Self {
        Self::new(429, "RESOURCE_LIMIT_EXCEEDED", message)
    }
    pub fn unsupported() -> Self {
        Self::new(
            501,
            "UNSUPPORTED_OPERATION",
            "operation is not supported by this build",
        )
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ApiError {}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(json!({"error":self}))).into_response()
    }
}

/// A successful content open must hold its core read lease until the body drops.
pub struct Content {
    pub fallback_files: Vec<std::fs::File>,
    pub checksum_manifest: Option<zxs_contract::ChecksumManifest>,
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
    manifest: &zxs_contract::ChecksumManifest,
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
    if !zxs_contract::SUPPORTED_CHECKSUM_CHUNK_BYTES.contains(&manifest.chunk_bytes)
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
        self.external_origin
            .as_ref()
            .is_some_and(|origin| origin.starts_with("https://"))
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
        if self.max_upload_bytes == 0 {
            return Err(ApiError::invalid("upload limit must be positive"));
        }
        if self.smb.as_ref().is_some_and(|smb| smb.users.is_empty()) {
            return Err(ApiError::invalid(
                "SMB requires explicit authenticated-user mappings",
            ));
        }
        if let Some(origin) = &self.external_origin {
            let parsed = url::Url::parse(origin).map_err(ApiError::invalid)?;
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
        }
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    backend: Arc<dyn Backend>,
    auth: Arc<Mutex<AuthStore>>,
    config: ServerConfig,
    attempts: Arc<Mutex<HashMap<String, http_auth::Attempts>>>,
    pending_logins: Arc<Mutex<std::collections::HashSet<String>>>,
    password_slots: Arc<tokio::sync::Semaphore>,
    upload_slots: Arc<tokio::sync::Semaphore>,
    read_slots: Arc<tokio::sync::Semaphore>,
    upload_claims: Arc<Mutex<std::collections::BTreeSet<String>>>,
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
        attempts: Arc::new(Mutex::new(HashMap::new())),
        pending_logins: Arc::new(Mutex::new(std::collections::HashSet::new())),
        password_slots: Arc::new(tokio::sync::Semaphore::new(4)),
        upload_slots: Arc::new(tokio::sync::Semaphore::new(8)),
        read_slots: Arc::new(tokio::sync::Semaphore::new(8)),
        upload_claims: Arc::new(Mutex::new(Default::default())),
        static_etags: Arc::new(match &config.web_dir {
            Some(root) => web_assets::static_etags(root)?,
            None => BTreeMap::new(),
        }),
    };
    let protected = Router::new()
        .merge(file_manager::routes())
        .route("/v1/system/status", get(telemetry::status))
        .route("/v1/system/history", get(telemetry::history))
        .route("/v1/auth/me", get(me))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/auth/password", post(change_password))
        .route("/v1/admin/accounts", get(accounts).post(account_upsert))
        .route(
            "/v1/repositories",
            get(repositories).post(create_repository),
        )
        .route("/v1/repositories/{repo}", get(repository))
        .route("/v1/repositories/{repo}/grants", put(grants))
        .route(
            "/v1/repositories/{repo}/smb-binding",
            get(smb_binding).put(update_smb_binding),
        )
        .route(
            "/v1/repositories/{repo}/default-branch",
            get(default_branch).put(update_default_branch),
        )
        .route(
            "/v1/repositories/{repo}/directory-scopes",
            post(initialize_directory_scope),
        )
        .route(
            "/v1/repositories/{repo}/directory-branches",
            get(directory_branch_scopes),
        )
        .route(
            "/v1/repositories/{repo}/directory-branches/{id}",
            patch(rename_directory_branch),
        )
        .route(
            "/v1/repositories/{repo}/branches",
            get(branches).post(create_branch),
        )
        .route("/v1/branches/{branch}/tree", get(tree))
        .route("/v1/branches/{branch}", get(branch_info))
        .route("/v1/branches/{branch}/resolve", post(resolve))
        .route("/v1/branches/{branch}/write-sessions", post(write_session))
        .route("/v1/branches/{branch}/archive", post(archive))
        .route("/v1/workspaces", post(workspace))
        .route("/v1/workspaces/{workspace}", get(workspace_info))
        .route(
            "/v1/workspaces/{workspace}/files/{item}/content",
            put(upload).get(workspace_content).head(workspace_content),
        )
        .route(
            "/v1/workspaces/{workspace}/files/{item}/upload-status",
            get(upload_status),
        )
        .route(
            "/v1/workspaces/{workspace}/namespace-operations",
            post(namespace),
        )
        .route("/v1/workspaces/{workspace}/commit", post(commit))
        .route("/v1/workspaces/{workspace}/seal", post(seal))
        .route("/v1/workspaces/{workspace}/abort", post(abort))
        .route("/v1/operations/{operation}", get(operation))
        .route("/v1/versions/{version}/content", get(content).head(content))
        .route("/v1/versions/{version}", get(version))
        .route("/v1/versions/{version}/placement", get(placement))
        .route("/v1/versions/{version}/checksums", get(checksums))
        .route("/v1/files/{file}/history", get(history))
        .route("/v1/commits/{commit}", get(commit_info))
        .route("/v1/history-trees/{history}/graph", get(graph))
        .route("/v1/history-trees/{history}/merge-links", post(merge_link))
        .route("/v1/history-trees/{history}/purge-plans", post(purge_plan))
        .route("/v1/history-trees/{history}/split-plans", post(split_plan))
        .route(
            "/v1/history-trees/{history}/historical-save-plans",
            post(historical_save_plan),
        )
        .route("/v1/plans/{plan}/commit", post(plan_commit))
        .route("/v1/plans/{plan}", get(plan_info))
        .route("/v1/plans/{plan}/cancel", post(plan_cancel))
        .route("/v1/admin/disks", get(disks).post(register_disk))
        .route("/v1/admin/check", get(check))
        .route("/v1/backup-topologies/{topology}", get(topology))
        .route("/v1/policies/{policy}", get(policy).put(update_policy))
        .route(
            "/v1/primary-placement-rules/{rule}",
            get(placement_rule).put(update_placement_rule),
        )
        .route("/v1/events", get(events))
        .route("/v1/compare", post(compare))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize));
    let mut app = Router::new()
        .merge(protected)
        .route(
            "/healthz",
            get(|| async { Json(json!({"status":"alive"})) }),
        )
        .route("/readyz", get(ready))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/auth/login", post(login))
        .fallback(web_assets::not_found);
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
        axum::serve(
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
    let sensitive = request.uri().path().starts_with("/v1/")
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
    tokio::task::spawn_blocking(move || state.backend.call(action, &identity, input))
        .await
        .map_err(ApiError::invalid)?
        .map(Json)
}
async fn json_body(request: Request) -> Result<Value, ApiError> {
    let bytes = axum::body::to_bytes(request.into_body(), 1024 * 1024)
        .await
        .map_err(ApiError::invalid)?;
    if bytes.is_empty() {
        Ok(json!({}))
    } else {
        zxs_contract::strict_json(&bytes).map_err(ApiError::invalid)
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

macro_rules! list_handler { ($name:ident, $action:expr) => { async fn $name(State(state): State<AppState>, axum::Extension(identity): axum::Extension<Identity>, Query(query): Query<BTreeMap<String,String>>) -> Result<Json<Value>, ApiError> { call(state,identity,$action,json!({"query":query})).await } }; }
macro_rules! read_handler { ($name:ident, $action:expr, $id:literal) => { async fn $name(State(state): State<AppState>, axum::Extension(identity): axum::Extension<Identity>, RoutePath(id): RoutePath<String>, Query(query): Query<BTreeMap<String,String>>) -> Result<Json<Value>, ApiError> { let mut input=json!({"query":query}); input[$id]=id.into(); call(state,identity,$action,input).await } }; }
macro_rules! post_handler {
    ($name:ident, $action:expr) => {
        async fn $name(
            State(state): State<AppState>,
            axum::Extension(identity): axum::Extension<Identity>,
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
            axum::Extension(identity): axum::Extension<Identity>,
            RoutePath(id): RoutePath<String>,
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
    axum::Extension(identity): axum::Extension<Identity>,
    RoutePath((repo, id)): RoutePath<(String, String)>,
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
mod tests {
    use super::*;
    use axum::body::{Body, Bytes};
    use std::time::Duration;
    use tower::ServiceExt;
    struct TestBackend {
        path: PathBuf,
    }
    impl Backend for TestBackend {
        fn call(
            &self,
            action: crate::BackendAction,
            _identity: &Identity,
            _input: Value,
        ) -> Result<Value, ApiError> {
            match action {
                crate::BackendAction::Capabilities
                | crate::BackendAction::UploadAuthorize
                | crate::BackendAction::WorkspaceAuthorize
                | crate::BackendAction::CreateRepository => Ok(json!({"accepted":true})),
                _ => Err(ApiError::unsupported()),
            }
        }
        fn upload(
            &self,
            _identity: &Identity,
            workspace: &str,
            item: &str,
            path: &str,
            source: &Path,
        ) -> Result<Value, ApiError> {
            let bytes = std::fs::read(source).map_err(ApiError::io)?;
            Ok(
                json!({"workspace_id":workspace,"item_id":item,"path":path,"size":bytes.len().to_string(),"content_hash":blake3::hash(&bytes).to_hex().to_string()}),
            )
        }
        fn content(&self, _identity: &Identity, _version: &str) -> Result<Content, ApiError> {
            Ok(Content {
                checksum_manifest: None,
                fallback_files: Vec::new(),
                file: std::fs::File::open(&self.path).map_err(ApiError::io)?,
                size: 10,
                hash: blake3::hash(b"0123456789").to_hex().to_string(),
                lease: Box::new(()),
                download_name: None,
            })
        }
    }
    fn application(root: &Path) -> Router {
        application_config(root, ServerConfig::loopback(root))
    }
    fn application_config(root: &Path, config: ServerConfig) -> Router {
        std::fs::write(root.join("content"), b"0123456789").unwrap();
        let auth = AuthStore::initialize(root, "admin", "a-long-test-password").unwrap();
        router(
            Arc::new(TestBackend {
                path: root.join("content"),
            }),
            auth,
            config,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn static_assets_never_shadow_unknown_api_routes_or_their_error_envelope() {
        let root = tempfile::tempdir().unwrap();
        let assets = root.path().join("web");
        std::fs::create_dir_all(assets.join("v1")).unwrap();
        std::fs::write(assets.join("index.html"), b"<html>fixture</html>").unwrap();
        std::fs::write(assets.join("v1/unknown"), b"must never be an API response").unwrap();
        let mut config = ServerConfig::loopback(root.path());
        config.web_dir = Some(assets);
        let app = application_config(root.path(), config);
        for path in ["/v1/unknown", "/v1/missing", "/missing.css"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .header(header::IF_NONE_MATCH, "*")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            if path.starts_with("/v1/") {
                assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            }
            let error: Value = serde_json::from_slice(
                &axum::body::to_bytes(response.into_body(), 8192)
                    .await
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(error["error"]["code"], "NOT_FOUND");
        }
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap(),
            "<html>fixture</html>"
        );
    }
    #[test]
    fn router_prepares_a_fresh_nested_spool_before_monitor_cache_initialization() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("fresh/nested/system");
        let auth = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
        let app = router(
            Arc::new(TestBackend {
                path: root.path().join("content"),
            }),
            auth,
            ServerConfig::loopback(&nested),
        );
        assert!(
            app.is_ok(),
            "fresh nested spool must initialize before telemetry"
        );
        assert!(nested.join("api-spool").is_dir());
        assert!(nested.join("telemetry-cache").is_dir());
    }

    #[test]
    fn an_unwritable_monitor_cache_does_not_prevent_the_business_router_starting() {
        let root = tempfile::tempdir().unwrap();
        let auth = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
        // A preexisting ordinary file deterministically prevents cache writes,
        // including when tests run as root and chmod would not deny access.
        std::fs::write(root.path().join("telemetry-cache"), b"keep this file").unwrap();
        let app = router(
            Arc::new(TestBackend {
                path: root.path().join("content"),
            }),
            auth,
            ServerConfig::loopback(root.path()),
        );
        assert!(
            app.is_ok(),
            "optional monitoring persistence must not stop business routes"
        );
        assert_eq!(
            std::fs::read(root.path().join("telemetry-cache")).unwrap(),
            b"keep this file"
        );
    }

    async fn authenticated(app: &Router) -> (String, String, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"a-long-test-password"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap(),
        )
        .unwrap();
        (
            body["token"].as_str().unwrap().into(),
            body["csrf_token"].as_str().unwrap().into(),
            cookie,
        )
    }

    #[tokio::test]
    async fn cookie_mutations_require_csrf_and_foreign_origins_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let app = application(root.path());
        let (token, csrf, cookie) = authenticated(&app).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/repositories")
                    .header("cookie", &cookie)
                    .header("host", "127.0.0.1:7400")
                    .header("origin", "http://127.0.0.1:7400")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/repositories")
                    .header("cookie", &cookie)
                    .header("x-csrf-token", csrf)
                    .header("host", "127.0.0.1:7400")
                    .header("origin", "http://127.0.0.1:7400")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/auth/me")
                    .header("authorization", format!("Bearer {token}"))
                    .header("origin", "https://foreign.invalid")
                    .header("host", "127.0.0.1:7400")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn authenticated_range_download_returns_exact_immutable_bytes() {
        let root = tempfile::tempdir().unwrap();
        let app = application(root.path());
        let (token, _, _) = authenticated(&app).await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/versions/test/content")
                    .header("authorization", format!("Bearer {token}"))
                    .header("range", "bytes=2-5")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 8192)
                .await
                .unwrap(),
            "2345"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn authentication_lock_wait_does_not_block_the_async_executor() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let root = tempfile::tempdir().unwrap();
        let mut auth = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
        let token = auth.login("admin", "a-long-test-password").unwrap().token;
        let (started, waiting) = tokio::sync::oneshot::channel();
        let started = Mutex::new(Some(started));
        let (release, released) = std::sync::mpsc::channel();
        let released = Mutex::new(released);
        let armed = Arc::new(AtomicBool::new(false));
        let timed_out = Arc::new(AtomicBool::new(false));
        let (hook_armed, hook_timeout) = (armed.clone(), timed_out.clone());
        auth.attach_backup(Arc::new(move |_| {
            if hook_armed.load(Ordering::SeqCst)
                && let Some(started) = started.lock().unwrap().take()
            {
                let _ = started.send(());
                if released
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(2))
                    .is_err()
                {
                    hook_timeout.store(true, Ordering::SeqCst);
                }
            }
            Ok(())
        }))
        .unwrap();
        armed.store(true, Ordering::SeqCst);
        let app = router(
            Arc::new(TestBackend {
                path: root.path().join("unused"),
            }),
            auth,
            ServerConfig::loopback(root.path()),
        )
        .unwrap();
        let login_app = app.clone();
        let login = tokio::spawn(async move {
            login_app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/auth/login")
                        .body(Body::from(
                            r#"{"username":"admin","password":"a-long-test-password"}"#,
                        ))
                        .unwrap(),
                )
                .await
                .unwrap()
        });
        waiting.await.unwrap();
        let release_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            release.send(()).unwrap();
        });
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/auth/me")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        release_task.await.unwrap();
        assert_eq!(login.await.unwrap().status(), StatusCode::OK);
        assert!(
            !timed_out.load(Ordering::SeqCst),
            "authentication starved the timer that releases its blocking backup"
        );
    }

    #[tokio::test]
    async fn interrupted_upload_retains_durable_bytes_and_rejects_different_retry() {
        let root = tempfile::tempdir().unwrap();
        let app = application(root.path());
        let (token, _, _) = authenticated(&app).await;
        let stream = futures_util::stream::iter([
            Ok(Bytes::from_static(b"original")),
            Err(std::io::Error::other("connection interrupted")),
        ]);
        let request = Request::builder()
            .method("PUT")
            .uri("/v1/workspaces/ws/files/item/content?path=file")
            .header("authorization", format!("Bearer {token}"))
            .header("x-zxs-logical-size", "20")
            .header("x-zxs-content-hash", "0".repeat(64))
            .body(Body::from_stream(stream))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let record = uploads::load(&root.path().join("api-spool"), "ws", "item")
            .unwrap()
            .unwrap();
        assert_eq!(record.persisted_bytes, 8);
        assert_eq!(
            std::fs::read(root.path().join("api-spool").join(&record.data_file)).unwrap(),
            b"original"
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/workspaces/ws/files/item/content?path=file")
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-zxs-logical-size", "20")
                    .header("x-zxs-content-hash", "0".repeat(64))
                    .body(Body::from("different retry data"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            uploads::load(&root.path().join("api-spool"), "ws", "item")
                .unwrap()
                .unwrap()
                .data_file,
            record.data_file
        );
        let data_files = std::fs::read_dir(root.path().join("api-spool"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "data")
            })
            .count();
        assert_eq!(
            data_files, 1,
            "conflicting retries must remove their unreferenced staging file"
        );
    }

    #[tokio::test]
    async fn durable_retry_progress_replaces_only_the_same_generation_predecessor() {
        let root = tempfile::tempdir().unwrap();
        let app = application(root.path());
        let (token, _, _) = authenticated(&app).await;
        let spool = root.path().join("api-spool");
        for content in ["original", "original plus", "original plus more"] {
            let stream = futures_util::stream::iter([
                Ok(Bytes::copy_from_slice(content.as_bytes())),
                Err(std::io::Error::other("interrupted")),
            ]);
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri("/v1/workspaces/ws/files/item/content?path=file")
                        .header("authorization", format!("Bearer {token}"))
                        .header("x-zxs-logical-size", "30")
                        .header("x-zxs-content-hash", "0".repeat(64))
                        .body(Body::from_stream(stream))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let current = uploads::load(&spool, "ws", "item").unwrap().unwrap();
            assert_eq!(
                std::fs::read(spool.join(current.data_file)).unwrap(),
                content.as_bytes()
            );
            assert_eq!(
                std::fs::read_dir(&spool)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|path| path
                        .extension()
                        .is_some_and(|extension| extension == "data"))
                    .count(),
                1
            );
        }
    }

    #[tokio::test]
    async fn unpublished_retry_cleanup_covers_short_long_and_cancelled_bodies() {
        let root = tempfile::tempdir().unwrap();
        let app = application(root.path());
        let (token, _, _) = authenticated(&app).await;
        let bytes = vec![42u8; 2 * 1024 * 1024];
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let request = |body| {
            Request::builder()
                .method("PUT")
                .uri("/v1/workspaces/ws/files/item/content?path=file")
                .header("authorization", format!("Bearer {token}"))
                .header("x-zxs-logical-size", bytes.len().to_string())
                .header("x-zxs-content-hash", &hash)
                .body(body)
                .unwrap()
        };
        assert_eq!(
            app.clone()
                .oneshot(request(Body::from(bytes.clone())))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let spool = root.path().join("api-spool");
        let original = uploads::load(&spool, "ws", "item").unwrap().unwrap();
        // A retry must not supersede old progress until it has checked all of it.
        for body in [
            Body::from(vec![42u8; 1024]),
            Body::from(vec![42u8; bytes.len() + 1]),
            Body::from_stream(futures_util::stream::iter([
                Ok(Bytes::from(vec![42u8; 1024])),
                Err(std::io::Error::other("interrupted")),
            ])),
        ] {
            assert!(
                !app.clone()
                    .oneshot(request(body))
                    .await
                    .unwrap()
                    .status()
                    .is_success()
            );
        }
        let (sent, received) = tokio::sync::oneshot::channel();
        let stream = futures_util::stream::once(async move {
            let _ = sent.send(());
            Ok::<_, std::io::Error>(Bytes::from(vec![42u8; 1024]))
        });
        let stream = futures_util::StreamExt::chain(stream, futures_util::stream::pending());
        let retry_app = app.clone();
        let retry_request = request(Body::from_stream(stream));
        let retry = tokio::spawn(async move { retry_app.oneshot(retry_request).await });
        received.await.unwrap();
        retry.abort();
        let _ = retry.await;
        // Cancellation may leave a running prefix-check job holding the guard.
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let files = std::fs::read_dir(&spool)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|path| {
                        path.extension()
                            .is_some_and(|extension| extension == "data")
                    })
                    .count();
                if files == 1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let current = uploads::load(&spool, "ws", "item").unwrap().unwrap();
        assert_eq!(current.data_file, original.data_file);
        assert_eq!(std::fs::read(spool.join(current.data_file)).unwrap(), bytes);
    }

    #[tokio::test]
    async fn successful_new_generation_preserves_old_bytes_and_retry_cleans_up_only_its_own() {
        let root = tempfile::tempdir().unwrap();
        let app = application(root.path());
        let (token, _, _) = authenticated(&app).await;
        let spool = root.path().join("api-spool");
        let mut records = Vec::new();
        for (generation, bytes) in [(1, "original"), (2, "replacement"), (2, "replacement")] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri("/v1/workspaces/ws/files/item/content?path=file")
                        .header("authorization", format!("Bearer {token}"))
                        .header("x-zxs-logical-size", bytes.len().to_string())
                        .header(
                            "x-zxs-content-hash",
                            blake3::hash(bytes.as_bytes()).to_hex().as_str(),
                        )
                        .header("x-zxs-upload-generation", generation.to_string())
                        .body(Body::from(bytes))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            records.push(uploads::load(&spool, "ws", "item").unwrap().unwrap());
            assert_eq!(
                std::fs::read(spool.join(&records[0].data_file)).unwrap(),
                b"original"
            );
        }
        assert!(!spool.join(&records[1].data_file).exists());
        assert_eq!(
            std::fs::read(spool.join(&records[2].data_file)).unwrap(),
            b"replacement"
        );
        let retained = std::fs::read_dir(&spool)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("retained-")
            })
            .collect::<Vec<_>>();
        assert_eq!(retained.len(), 1);
        let retained: uploads::UploadRecord =
            serde_json::from_slice(&std::fs::read(&retained[0]).unwrap()).unwrap();
        assert_eq!(retained.generation, 1);
        assert_eq!(retained.data_file, records[0].data_file);
    }

    #[tokio::test]
    async fn disconnected_upload_keeps_its_claim_until_publication_and_status_are_durable() {
        struct BlockingUpload {
            delegate: TestBackend,
            started: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
            gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
        }
        impl Backend for BlockingUpload {
            fn call(
                &self,
                action: crate::BackendAction,
                identity: &Identity,
                input: Value,
            ) -> Result<Value, ApiError> {
                self.delegate.call(action, identity, input)
            }
            fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
                self.delegate.content(identity, version)
            }
            fn upload(
                &self,
                identity: &Identity,
                workspace: &str,
                item: &str,
                path: &str,
                source: &Path,
            ) -> Result<Value, ApiError> {
                if let Some(started) = self.started.lock().unwrap().take() {
                    let _ = started.send(());
                }
                let (ready, wake) = &*self.gate;
                let mut ready = ready.lock().unwrap();
                while !*ready {
                    ready = wake.wait(ready).unwrap();
                }
                self.delegate
                    .upload(identity, workspace, item, path, source)
            }
        }
        struct Release(Arc<(Mutex<bool>, std::sync::Condvar)>);
        impl Drop for Release {
            fn drop(&mut self) {
                *self.0.0.lock().unwrap() = true;
                self.0.1.notify_all();
            }
        }
        let root = tempfile::tempdir().unwrap();
        let auth = AuthStore::initialize(root.path(), "admin", "a-long-test-password").unwrap();
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let release = Release(gate.clone());
        let app = router(
            Arc::new(BlockingUpload {
                delegate: TestBackend {
                    path: root.path().join("content"),
                },
                started: Mutex::new(Some(started)),
                gate,
            }),
            auth,
            ServerConfig::loopback(root.path()),
        )
        .unwrap();
        let (token, _, _) = authenticated(&app).await;
        let request = || {
            Request::builder()
                .method("PUT")
                .uri("/v1/workspaces/ws/files/item/content?path=file")
                .header("authorization", format!("Bearer {token}"))
                .header("x-zxs-logical-size", "4")
                .header(
                    "x-zxs-content-hash",
                    blake3::hash(b"body").to_hex().to_string(),
                )
                .body(Body::from("body"))
                .unwrap()
        };
        let pending = tokio::spawn(app.clone().oneshot(request()));
        tokio::time::timeout(Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        let retry = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(retry.status(), StatusCode::CONFLICT);
        let error: Value =
            serde_json::from_slice(&axum::body::to_bytes(retry.into_body(), 8192).await.unwrap())
                .unwrap();
        assert_eq!(error["error"]["code"], "FILE_WRITE_BUSY");
        drop(release);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if uploads::load(&root.path().join("api-spool"), "ws", "item")
                    .unwrap()
                    .unwrap()
                    .state
                    == "complete"
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    }
}
