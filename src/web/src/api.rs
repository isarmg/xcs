use js_sys::{Reflect, Uint8Array};
use json_codec::{Value, json};
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_interface::i18n::t;
use web_sys::{RequestCredentials, RequestInit, RequestMode, RequestRedirect, Response};

mod response;

// Token and CSRF belong to one credential generation. A response from a
// previous login may not clear or replace the current login's credentials.
// Deliberately no Debug: this state contains authentication secrets.
#[derive(Clone, Default)]
struct Credentials {
    token: Option<String>,
    csrf: Option<String>,
    generation: u64,
}
impl Credentials {
    fn replace(&mut self, token: Option<String>, csrf: Option<String>) {
        self.generation += 1;
        self.token = token;
        self.csrf = csrf;
    }
    fn replace_if_current(
        &mut self,
        generation: u64,
        token: Option<String>,
        csrf: Option<String>,
    ) -> bool {
        if self.generation != generation {
            return false;
        }
        self.replace(token, csrf);
        true
    }
    fn revoke(&mut self, generation: u64) -> bool {
        if self.generation != generation {
            return false;
        }
        self.replace(None, None);
        true
    }
}
thread_local! { static CREDENTIALS: RefCell<Credentials> = RefCell::new(Credentials::default()); }

#[derive(Clone, Debug)]
pub struct Error {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub status: u16,
    pub request_id: Option<String>,
}

impl Error {
    /// After a mutation request, only a client-error response proves rejection.
    /// A malformed success response can follow a completed publication.
    pub(crate) fn mutation_outcome_unknown(&self) -> bool {
        storage_contract::client_protocol::mutation_outcome_unknown(self.status)
    }

    fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} · {}{}",
            self.code,
            match self.code.as_str() {
                "DIRECTORY_SCOPE_REQUIRED" => t(
                    "请选择具体文件夹；此硬盘根目录包含多个已有目录。",
                    "Select a specific folder; this disk root contains multiple existing namespaces."
                ),
                "DIRECTORY_BRANCH_INCOMPLETE" => t(
                    "所选目录分支的引用不完整，未发布此次切换。",
                    "The selected folder branch has incomplete references. The switch was not published."
                ),
                _ => self.message.clone(),
            },
            if self.retryable {
                t(
                    "（可重试；提交操作请先查询原操作状态）",
                    " (retryable; check the original save status before retrying)",
                )
            } else {
                String::new()
            }
        )
    }
}

fn error(code: &str, message: impl Into<String>) -> Error {
    Error {
        code: code.into(),
        message: message.into(),
        retryable: false,
        status: 0,
        request_id: None,
    }
}

pub fn invalid_response() -> Error {
    error(
        "INVALID_RESPONSE",
        t(
            "服务端响应缺少必要字段",
            "The server response is missing required fields",
        ),
    )
}

pub fn session_changed() -> Error {
    error(
        "SESSION_CHANGED",
        t(
            "会话已改变，操作已停止。",
            "The session changed, so the operation was stopped.",
        ),
    )
}

pub fn clear_session() {
    download::cancel();
    CREDENTIALS.with(|saved| saved.borrow_mut().replace(None, None));
}

// Stop later stages of in-flight transfers before logout reaches the server.
// Keep the credential until revocation is confirmed so failed logout can retry.
pub(crate) fn invalidate_session_work() {
    download::cancel();
    CREDENTIALS.with(|saved| saved.borrow_mut().generation += 1);
}

fn credential_generation() -> u64 {
    CREDENTIALS.with(|saved| saved.borrow().generation)
}

fn ensure_credential_generation(generation: u64) -> Result<(), Error> {
    if credential_generation() == generation {
        Ok(())
    } else {
        Err(session_changed())
    }
}

/// Admission ticket for multi-stage browser work. Keep the original identity
/// across awaits and check it immediately before publishing or delivering data.
/// Browser JavaScript runs synchronously between this check and the final action.
#[derive(Clone, Copy)]
struct SessionAdmission(u64);
impl SessionAdmission {
    fn capture() -> Self {
        Self(credential_generation())
    }

    fn check(self) -> Result<(), Error> {
        ensure_credential_generation(self.0)
    }
}

pub fn operation_id() -> String {
    fresh_id("op")
}

pub fn entry_id() -> String {
    fresh_id("entry")
}

fn fresh_id(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    if let Some(crypto) = web_sys::window().and_then(|w| w.crypto().ok())
        && crypto.get_random_values_with_u8_array(&mut bytes).is_ok()
    {
        return format!(
            "{prefix}_{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
    }
    // The server rejects empty operation IDs. Do not silently substitute weak IDs.
    String::new()
}

pub fn segment(value: &str) -> String {
    js_sys::encode_uri_component(value)
        .as_string()
        .unwrap_or_default()
}

fn validate_path(path: &str) -> Result<(), Error> {
    storage_contract::validate_api_path(path).map_err(|_| {
        error(
            "INVALID_API_PATH",
            t(
                "仅允许本服务的 /api/ 业务接口",
                "Only this service’s /api/ API is allowed",
            ),
        )
    })
}

fn js_error(value: JsValue) -> Error {
    let message = value.as_string().unwrap_or_else(|| {
        t(
            "网络中断或系统操作失败；保存结果可能需要查询",
            "The connection was interrupted. Check whether the save completed.",
        )
    });
    error("TRANSPORT_ERROR", message)
}

struct Fetched {
    response: Response,
    credential_generation: u64,
    _timeout: gloo_timers::callback::Timeout,
}

async fn fetch(
    method: &str,
    path: &str,
    body: Option<JsValue>,
    content_type: Option<&str>,
) -> Result<Fetched, Error> {
    fetch_with_upload(method, path, body, content_type, None).await
}

async fn fetch_with_upload(
    method: &str,
    path: &str,
    body: Option<JsValue>,
    content_type: Option<&str>,
    upload: Option<&upload_source::UploadMetadata>,
) -> Result<Fetched, Error> {
    validate_path(path)?;
    let controller = web_sys::AbortController::new().map_err(js_error)?;
    let options = RequestInit::new();
    options.set_method(method);
    options.set_mode(RequestMode::SameOrigin);
    options.set_credentials(RequestCredentials::SameOrigin);
    options.set_redirect(RequestRedirect::Error);
    options.set_signal(Some(&controller.signal()));
    if let Some(body) = body.as_ref() {
        options.set_body(body);
    }
    let request = web_sys::Request::new_with_str_and_init(path, &options).map_err(js_error)?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(js_error)?;
    if let Some(content_type) = content_type {
        request
            .headers()
            .set("Content-Type", content_type)
            .map_err(js_error)?;
    }
    if let Some(upload) = upload {
        request
            .headers()
            .set("X-ZXS-Logical-Size", &upload.size.to_string())
            .map_err(js_error)?;
        request
            .headers()
            .set("X-ZXS-Upload-Generation", "1")
            .map_err(js_error)?;
        request
            .headers()
            .set("X-ZXS-Content-Hash", &upload.hash)
            .map_err(js_error)?;
    }
    let credentials = CREDENTIALS.with(|saved| saved.borrow().clone());
    if let Some(token) = credentials.token.as_ref() {
        request
            .headers()
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(js_error)?;
    }
    if !matches!(method, "GET" | "HEAD")
        && let Some(token) = credentials.csrf.as_ref()
    {
        request
            .headers()
            .set("X-CSRF-Token", token)
            .map_err(js_error)?;
    }
    let timeout_ms = upload.map_or(120_000, |upload| upload.total_timeout_ms);
    let timer = gloo_timers::callback::Timeout::new(timeout_ms, move || controller.abort());
    let window = web_sys::window().ok_or_else(|| {
        error(
            "NO_WINDOW",
            t("浏览器窗口不可用", "The browser window is unavailable"),
        )
    })?;
    let result = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(js_error);
    Ok(Fetched {
        response: result?.dyn_into().map_err(js_error)?,
        credential_generation: credentials.generation,
        _timeout: timer,
    })
}

async fn read_body(response: &Response, limit: usize) -> Result<Vec<u8>, Error> {
    read_body_inner(response, limit)
        .await
        .map_err(|error| error.with_status(response.status()))
}

async fn read_body_inner(response: &Response, limit: usize) -> Result<Vec<u8>, Error> {
    let Some(body) = response.body() else {
        return Ok(Vec::new());
    };
    let reader = body
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|value| js_error(value.into()))?;
    let mut bytes = Vec::new();
    loop {
        let part = JsFuture::from(reader.read()).await.map_err(js_error)?;
        if Reflect::get(&part, &"done".into())
            .map_err(js_error)?
            .as_bool()
            .unwrap_or(false)
        {
            break;
        }
        let chunk = Uint8Array::new(&Reflect::get(&part, &"value".into()).map_err(js_error)?);
        if bytes.len().saturating_add(chunk.length() as usize) > limit {
            let _ = JsFuture::from(reader.cancel()).await;
            return Err(error(
                "RESPONSE_TOO_LARGE",
                t(
                    "响应超过界面安全上限，请缩小查询范围",
                    "The response is too large. Narrow the query.",
                ),
            ));
        }
        bytes.extend_from_slice(&chunk.to_vec());
    }
    reader.release_lock();
    Ok(bytes)
}

async fn decode(fetched: Fetched) -> Result<Value, Error> {
    let status = fetched.response.status();
    if status == 401 {
        CREDENTIALS.with(|saved| saved.borrow_mut().revoke(fetched.credential_generation));
    }
    if fetched
        .response
        .headers()
        .get("Content-Length")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|v| v > storage_contract::client_protocol::MAX_RESPONSE_BYTES as u64)
    {
        return Err(response::from_protocol(
            storage_contract::client_protocol::response_too_large(status),
        ));
    }
    let bytes = read_body(
        &fetched.response,
        storage_contract::client_protocol::MAX_RESPONSE_BYTES,
    )
    .await
    .map_err(|error| {
        if error.code == "RESPONSE_TOO_LARGE" {
            response::from_protocol(storage_contract::client_protocol::response_too_large(
                status,
            ))
        } else {
            error.with_status(status)
        }
    })?;
    response::decode(status, &bytes)
}

pub async fn request(method: &str, path: &str, body: Option<Value>) -> Result<Value, Error> {
    validate_path(path)?;
    decode(
        fetch(
            method,
            path,
            body.map(|v| JsValue::from_str(&v.to_string())),
            Some("application/json"),
        )
        .await?,
    )
    .await
}

pub async fn login(username: &str, password: &str) -> Result<Value, Error> {
    let generation = CREDENTIALS.with(|saved| saved.borrow().generation);
    let result = request(
        "POST",
        "/api/auth/login",
        Some(json!({"username":username,"password":password})),
    )
    .await?;
    let token = result["token"]
        .as_str()
        .ok_or_else(|| {
            error(
                "INVALID_SESSION",
                t(
                    "登录响应缺少会话令牌",
                    "The login response has no session token",
                ),
            )
        })?
        .to_owned();
    let accepted = CREDENTIALS.with(|saved| {
        saved.borrow_mut().replace_if_current(
            generation,
            Some(token),
            result["csrf_token"].as_str().map(str::to_owned),
        )
    });
    if !accepted {
        return Err(session_changed());
    }
    Ok(result)
}

pub async fn restore() -> Result<Value, Error> {
    let generation = CREDENTIALS.with(|saved| saved.borrow().generation);
    let value = request("GET", "/api/auth/me", None).await?;
    CREDENTIALS.with(|saved| {
        let mut saved = saved.borrow_mut();
        if saved.generation == generation {
            saved.csrf = value["csrf_token"].as_str().map(str::to_owned);
        }
    });
    Ok(value)
}

pub async fn commit_workspace(
    workspace: &str,
    operation_id: &str,
    message: &str,
    expected_head: &str,
    on_commit: impl FnOnce(),
) -> Result<Value, Error> {
    let admission = SessionAdmission::capture();
    let body = request(
        "POST",
        &format!("/api/workspaces/{}/seal", segment(workspace)),
        Some(json!({})),
    )
    .await?;
    // Sealing can outlive logout or a replacement login. Never publish the old
    // workspace using credentials installed while that request was in flight.
    admission.check()?;
    let body = storage_contract::client_protocol::prepare_commit(
        body,
        workspace,
        operation_id,
        message,
        Some(expected_head),
    )
    .map_err(error_from_contract)?;
    // A failed seal (including a lost seal response) has not submitted this
    // operation ID. Only publication errors require querying that operation.
    on_commit();
    request(
        "POST",
        &format!("/api/workspaces/{}/commit", segment(workspace)),
        Some(body),
    )
    .await
}
fn error_from_contract(value: storage_contract::Error) -> Error {
    Error {
        code: value.code,
        message: value.message,
        retryable: value.retryable,
        status: 0,
        request_id: None,
    }
}

mod upload_source;
pub use upload_source::UploadSource;
pub(crate) use upload_source::upload_limit;

pub async fn upload(
    workspace: &str,
    item: &str,
    path: &str,
    source: UploadSource,
) -> Result<Value, Error> {
    source.preflight()?;
    let admission = SessionAdmission::capture();
    let limits = upload_source::upload_limits().await;
    upload_source::check_completion(admission, &limits)?;
    let limits = limits?;
    source.preflight_limit(limits.max_upload_bytes)?;
    let prepared = source
        .prepare(admission, limits.transfer.total_timeout_ms)
        .await;
    admission.check()?;
    let metadata = prepared?;
    let endpoint = format!(
        "/api/workspaces/{}/files/{}/content?path={}",
        segment(workspace),
        segment(item),
        segment(path)
    );
    let fetched = fetch_with_upload(
        "PUT",
        &endpoint,
        Some(source.into_file().into()),
        Some("application/octet-stream"),
        Some(&metadata),
    )
    .await;
    admission.check()?;
    let result = decode(fetched?).await;
    upload_source::check_completion(admission, &result)?;
    result
}

mod download;
pub(crate) fn observe_downloads(callback: impl Fn() + 'static) -> impl Drop {
    download::observe(callback)
}
pub(crate) fn cancel_download() {
    download::cancel();
}
pub(crate) fn download_copy() -> Option<(String, bool)> {
    download::retained_copy()
}
pub(crate) fn export_download() -> Result<String, Error> {
    download::export()
}
pub(crate) async fn cleanup_download() -> Result<(), Error> {
    download::cleanup().await
}
pub fn download(
    path: &str,
    filename: &str,
) -> impl std::future::Future<Output = Result<String, Error>> + 'static {
    download::start(path, filename)
}

/// Delayed-body test fixture exercising the same integrity/session completion
/// gate as both production streaming sinks. Production never buffers this way.
#[cfg(test)]
async fn verified_download_bytes(
    body: impl std::future::Future<Output = Result<Vec<u8>, Error>>,
    length: u64,
    hash: &str,
    admission: SessionAdmission,
) -> Result<Vec<u8>, Error> {
    let bytes = body.await?;
    let mut integrity = download::Integrity::new(length);
    integrity.update(&bytes)?;
    integrity.finish_for_session(hash, admission)?;
    Ok(bytes)
}

pub async fn preview_version(version: &str) -> Result<String, Error> {
    let fetched = fetch(
        "GET",
        &format!("/api/versions/{}/content", segment(version)),
        None,
        None,
    )
    .await?;
    if !fetched.response.ok() {
        return decode(fetched).await.map(|_| String::new());
    }
    let hash = fetched
        .response
        .headers()
        .get("ETag")
        .ok()
        .flatten()
        .map(|v| v.trim_matches('"').to_owned())
        .ok_or_else(|| {
            error(
                "INVALID_RESPONSE",
                t(
                    "预览响应缺少摘要",
                    "The preview response has no integrity hash",
                ),
            )
        })?;
    let bytes = read_body(&fetched.response, 2 * 1024 * 1024).await?;
    if blake3::hash(&bytes).to_hex().as_str() != hash {
        return Err(error(
            "CHECKSUM_MISMATCH",
            t(
                "预览完整性校验失败",
                "Preview integrity verification failed",
            ),
        ));
    }
    let text = String::from_utf8(bytes).map_err(|_| {
        error(
            "BINARY_PREVIEW",
            t("预览仅支持 UTF-8 文本；请下载此文件后打开。", "Preview supports UTF-8 text. Download this file to open it in another application."),
        )
    })?;
    SessionAdmission(fetched.credential_generation).check()?;
    Ok(text)
}

#[cfg(test)]
#[path = "../../../test/web/unit/api/tests.rs"]
mod tests;
