use js_sys::{Function, Promise, Reflect, Uint8Array};
use serde_json::{Value, json};
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestCredentials, RequestInit, RequestMode, RequestRedirect, Response};
use zxs_web::i18n::t;

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
    pub commit_attempted: Option<bool>,
}

impl Error {
    /// After a mutation request, only a client-error response proves rejection.
    /// A malformed success response can follow a completed publication.
    pub(crate) fn mutation_outcome_unknown(&self) -> bool {
        zxs_contract::client_protocol::mutation_outcome_unknown(self.status)
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
        commit_attempted: None,
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

pub fn native() -> bool {
    web_sys::window()
        .is_some_and(|window| Reflect::has(&window, &"__TAURI__".into()).unwrap_or(false))
}

pub fn clear_session() {
    CREDENTIALS.with(|saved| saved.borrow_mut().replace(None, None));
}

pub fn operation_id() -> String {
    fresh_id("op")
}

pub fn entry_id() -> String {
    fresh_id("entry")
}

fn fresh_id(prefix: &str) -> String {
    let mut bytes = [0u8; 16];
    if let Some(crypto) = web_sys::window().and_then(|w| w.crypto().ok()) {
        if crypto.get_random_values_with_u8_array(&mut bytes).is_ok() {
            return format!(
                "{prefix}_{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            );
        }
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
    zxs_contract::validate_api_path(path).map_err(|_| {
        error(
            "INVALID_API_PATH",
            t(
                "仅允许本服务的 /v1/ 业务接口",
                "Only this service’s /v1/ API is allowed",
            ),
        )
    })
}

pub async fn invoke(command: &str, args: Value) -> Result<Value, Error> {
    let window = web_sys::window().ok_or_else(|| {
        error(
            "NO_WINDOW",
            t("浏览器窗口不可用", "The browser window is unavailable"),
        )
    })?;
    let host_api = Reflect::get(&window, &"__TAURI__".into()).map_err(js_error)?;
    let internal = Reflect::get(&host_api, &"core".into()).map_err(js_error)?;
    let function: Function = Reflect::get(&internal, &"invoke".into())
        .map_err(js_error)?
        .dyn_into()
        .map_err(js_error)?;
    let args = js_sys::JSON::parse(&args.to_string()).map_err(js_error)?;
    let promise: Promise = function
        .call2(&internal, &command.into(), &args)
        .map_err(js_error)?
        .dyn_into()
        .map_err(js_error)?;
    let value = JsFuture::from(promise).await.map_err(js_error)?;
    let text = js_sys::JSON::stringify(&value)
        .map_err(js_error)?
        .as_string()
        .unwrap_or_default();
    serde_json::from_str(&text).map_err(|_| {
        error(
            "INVALID_RESPONSE",
            t(
                "原生客户端响应格式无效",
                "The native application returned an invalid response",
            ),
        )
    })
}

fn js_error(value: JsValue) -> Error {
    if let Ok(code) = Reflect::get(&value, &"code".into()) {
        if let Some(code) = code.as_string() {
            return Error {
                code,
                message: Reflect::get(&value, &"message".into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| t("系统操作失败", "The system operation failed").into()),
                retryable: Reflect::get(&value, &"retryable".into())
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                status: Reflect::get(&value, &"status".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u16,
                commit_attempted: Reflect::get(&value, &"commit_attempted".into())
                    .ok()
                    .and_then(|value| value.as_bool()),
                request_id: Reflect::get(&value, &"request_id".into())
                    .ok()
                    .and_then(|id| id.as_string()),
            };
        }
    }
    let message = value.as_string().unwrap_or_else(|| {
        t(
            "网络中断或系统操作失败；保存结果可能需要查询",
            "The connection was interrupted. Check whether the save completed.",
        )
        .into()
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
    if method == "PUT" && content_type == Some("application/octet-stream") {
        let bytes = Uint8Array::new(body.as_ref().ok_or_else(|| {
            error(
                "EMPTY_UPLOAD",
                t("上传缺少内容", "The upload has no content"),
            )
        })?);
        request
            .headers()
            .set("X-ZXS-Logical-Size", &bytes.length().to_string())
            .map_err(js_error)?;
        request
            .headers()
            .set("X-ZXS-Upload-Generation", "1")
            .map_err(js_error)?;
        request
            .headers()
            .set(
                "X-ZXS-Content-Hash",
                &blake3::hash(&bytes.to_vec()).to_hex(),
            )
            .map_err(js_error)?;
    }
    let credentials = CREDENTIALS.with(|saved| saved.borrow().clone());
    if let Some(token) = credentials.token.as_ref() {
        request
            .headers()
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(js_error)?;
    }
    if !matches!(method, "GET" | "HEAD") {
        if let Some(token) = credentials.csrf.as_ref() {
            request
                .headers()
                .set("X-CSRF-Token", &token)
                .map_err(js_error)?;
        }
    }
    let timer = gloo_timers::callback::Timeout::new(120_000, move || controller.abort());
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
                    "响应超过界面安全上限，请使用 App 或缩小查询范围",
                    "The response is too large. Use the app or narrow the query.",
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
        .is_some_and(|v| v > zxs_contract::client_protocol::MAX_RESPONSE_BYTES as u64)
    {
        return Err(response::from_protocol(
            zxs_contract::client_protocol::response_too_large(status),
        ));
    }
    let bytes = read_body(
        &fetched.response,
        zxs_contract::client_protocol::MAX_RESPONSE_BYTES,
    )
    .await
    .map_err(|error| {
        if error.code == "RESPONSE_TOO_LARGE" {
            response::from_protocol(zxs_contract::client_protocol::response_too_large(status))
        } else {
            error.with_status(status)
        }
    })?;
    response::decode(status, &bytes)
}

pub async fn request(method: &str, path: &str, body: Option<Value>) -> Result<Value, Error> {
    validate_path(path)?;
    if native() {
        return invoke(
            "api_request",
            json!({"method":method,"path":path,"body":body}),
        )
        .await;
    }
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

pub async fn login(endpoint: &str, username: &str, password: &str) -> Result<Value, Error> {
    let generation = CREDENTIALS.with(|saved| saved.borrow().generation);
    let result = if native() {
        invoke(
            "connect",
            json!({"endpoint":endpoint,"username":username,"password":password}),
        )
        .await?
    } else {
        request(
            "POST",
            "/v1/auth/login",
            Some(json!({"username":username,"password":password})),
        )
        .await?
    };
    if !native() {
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
    }
    Ok(result)
}

pub async fn restore() -> Result<Value, Error> {
    let generation = CREDENTIALS.with(|saved| saved.borrow().generation);
    let value = request("GET", "/v1/auth/me", None).await?;
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
    if native() {
        let result = invoke("commit_workspace",json!({"workspace":workspace,"operationId":operation_id,"message":message,"expectedHead":expected_head})).await;
        // Only explicit native evidence can rule out publication. IPC failures
        // without stage information must retain conservative recovery behavior.
        if native_commit_attempted(&result) {
            on_commit();
        }
        return result;
    }
    let body = request(
        "POST",
        &format!("/v1/workspaces/{}/seal", segment(workspace)),
        Some(json!({})),
    )
    .await?;
    let body = zxs_contract::client_protocol::prepare_commit(
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
        &format!("/v1/workspaces/{}/commit", segment(workspace)),
        Some(body),
    )
    .await
}
fn native_commit_attempted(result: &Result<Value, Error>) -> bool {
    zxs_contract::client_protocol::publication_attempted(
        result
            .as_ref()
            .err()
            .and_then(|error| error.commit_attempted),
    )
}

fn error_from_contract(value: zxs_contract::Error) -> Error {
    Error {
        code: value.code,
        message: value.message,
        retryable: value.retryable,
        status: 0,
        request_id: None,
        commit_attempted: None,
    }
}

pub async fn upload(
    workspace: &str,
    item: &str,
    path: &str,
    file: web_sys::File,
) -> Result<Value, Error> {
    // Buffer a bounded browser file. Native Apps stream directly from disk.
    if file.size() > 64.0 * 1024.0 * 1024.0 {
        return Err(error(
            "BROWSER_UPLOAD_LIMIT",
            t(
                "网页单次上传上限 64 MiB；大文件请使用 App 或 CLI",
                "Browser uploads are limited to 64 MiB per file. Use the app or CLI for larger files.",
            ),
        ));
    }
    let data = JsFuture::from(file.array_buffer())
        .await
        .map_err(js_error)?;
    let bytes = Uint8Array::new(&data);
    let endpoint = format!(
        "/v1/workspaces/{}/files/{}/content?path={}",
        segment(workspace),
        segment(item),
        segment(path)
    );
    decode(
        fetch(
            "PUT",
            &endpoint,
            Some(bytes.into()),
            Some("application/octet-stream"),
        )
        .await?,
    )
    .await
}

pub async fn download(path: &str, filename: &str) -> Result<String, Error> {
    if native() {
        let version = path
            .strip_prefix("/v1/versions/")
            .and_then(|s| s.strip_suffix("/content"))
            .ok_or_else(|| {
                error(
                    "INVALID_DOWNLOAD",
                    t(
                        "只能下载固定版本",
                        "A fixed file version is required for downloading",
                    ),
                )
            })?;
        let result = invoke("download_file", json!({"version":version,"name":filename})).await?;
        return Ok(format!(
            "{}: {}",
            t("下载已校验并保存", "The download was verified and saved"),
            result["target"].as_str().unwrap_or("App cache")
        ));
    }
    let fetched = fetch("GET", path, None, None).await?;
    if !fetched.response.ok() {
        return decode(fetched).await.map(|_| String::new());
    }
    let length = fetched
        .response
        .headers()
        .get("Content-Length")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| {
            error(
                "INVALID_RESPONSE",
                t(
                    "下载响应缺少长度",
                    "The download response has no content length",
                ),
            )
        })?;
    if length > 64 * 1024 * 1024 {
        return Err(error(
            "BROWSER_DOWNLOAD_LIMIT",
            t(
                "网页校验下载上限 64 MiB；大文件请使用 App 或 CLI",
                "Verified browser downloads are limited to 64 MiB. Use the app or CLI for larger files.",
            ),
        ));
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
                    "下载响应缺少完整性摘要",
                    "The download response has no integrity hash",
                ),
            )
        })?;
    let bytes = read_body(&fetched.response, 64 * 1024 * 1024).await?;
    if bytes.len() as u64 != length || blake3::hash(&bytes).to_hex().as_str() != hash {
        return Err(error(
            "CHECKSUM_MISMATCH",
            t(
                "下载长度或 BLAKE3 摘要不一致，未交付文件",
                "The downloaded size or BLAKE3 hash does not match. The file was not delivered.",
            ),
        ));
    }
    let parts = js_sys::Array::new();
    parts.push(&Uint8Array::from(bytes.as_slice()).buffer());
    let blob = web_sys::Blob::new_with_buffer_source_sequence(&parts).map_err(js_error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
    let result = (|| {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| error("NO_DOCUMENT", t("页面不可用", "The page is unavailable")))?;
        let anchor = document
            .create_element("a")
            .map_err(js_error)?
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .map_err(|value| js_error(value.into()))?;
        anchor.set_href(&url);
        anchor.set_download(filename);
        anchor.click();
        Ok(t(
            "下载已校验并交给浏览器；此操作不会创建服务器版本",
            "The download was verified and sent to the browser.",
        ))
    })();
    gloo_timers::callback::Timeout::new(30_000, move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    })
    .forget();
    result
}

pub async fn open_version(version: &str, name: &str) -> Result<Option<String>, Error> {
    if native() {
        invoke("open_cached_file", json!({"version":version,"name":name})).await?;
        return Ok(None);
    }
    let fetched = fetch(
        "GET",
        &format!("/v1/versions/{}/content", segment(version)),
        None,
        None,
    )
    .await?;
    if !fetched.response.ok() {
        return decode(fetched).await.map(|_| None);
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
            t("此文件需要通过 App 的系统应用打开，或显式下载", "Preview supports UTF-8 text. Download this file to open it in another application."),
        )
    })?;
    Ok(Some(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_publication_evidence_is_explicit_and_missing_ipc_evidence_is_unknown() {
        let mut failure = error("TRANSPORT_ERROR", "connection lost");
        assert!(native_commit_attempted(&Err(failure.clone())));
        failure.commit_attempted = Some(false);
        assert!(!native_commit_attempted(&Err(failure.clone())));
        failure.commit_attempted = Some(true);
        assert!(native_commit_attempted(&Err(failure)));
        assert!(native_commit_attempted(&Ok(json!({"state":"SUCCEEDED"}))));
    }

    #[test]
    fn late_login_cannot_restore_cleared_or_replace_newer_credentials() {
        let mut saved = Credentials::default();
        let request_generation = saved.generation;
        saved.replace(None, None);
        assert!(!saved.replace_if_current(request_generation, Some("late".into()), None));
        assert!(saved.token.is_none());
        let current = saved.generation;
        assert!(saved.replace_if_current(current, Some("new".into()), Some("csrf".into())));
        assert!(!saved.replace_if_current(current, Some("late".into()), None));
        assert_eq!(saved.token.as_deref(), Some("new"));
        assert_eq!(saved.csrf.as_deref(), Some("csrf"));
    }
    #[test]
    fn an_old_unauthorized_response_cannot_revoke_a_new_login() {
        let mut saved = Credentials::default();
        saved.replace(Some("old-token".into()), Some("old-csrf".into()));
        let old_request = saved.generation;
        saved.replace(Some("new-token".into()), Some("new-csrf".into()));
        assert!(!saved.revoke(old_request));
        assert_eq!(saved.token.as_deref(), Some("new-token"));
        assert_eq!(saved.csrf.as_deref(), Some("new-csrf"));
        assert!(saved.revoke(saved.generation));
        assert!(saved.token.is_none());
        assert!(saved.csrf.is_none());
        assert!(!saved.revoke(old_request));
    }
    #[test]
    fn restrict_transport_to_own_api() {
        assert!(validate_path("/v1/repositories").is_ok());
        assert!(validate_path("/v1/file-manager/disks/P01/entries?path=report..txt").is_ok());
        assert!(validate_path("/v1/file-manager/disks/P01/entries?path=photos%2F2026").is_ok());
        for path in [
            "https://other/v1/a",
            "//other/v1/a",
            "/api/v1/a",
            "/v1/../secret",
            "/v1/%2e%2e/secret",
            "/v1/.%2e/secret",
            "/v1/a%2fb",
            "/v1/a#secret",
            "/v1/\\evil",
            "/v1/\t../secret",
            "/v1/file\nname",
        ] {
            assert!(validate_path(path).is_err(), "{path}");
        }
    }
}
