use serde_json::Value;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FsExt;
use tauri_plugin_opener::OpenerExt;
use zxs_app_bridge::{Bridge, BridgeError, CommitError};

fn local_error(code: &str, error: impl std::fmt::Display) -> BridgeError {
    BridgeError {
        code: code.into(),
        message: error.to_string(),
        retryable: false,
        status: 0,
    }
}

#[tauri::command]
async fn choose_file(app: tauri::AppHandle) -> Result<Value, BridgeError> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_file(move |file| {
        let _ = send.send(file);
    });
    let Some(file) = receive
        .await
        .map_err(|error| local_error("PICKER_CLOSED", error))?
    else {
        return Ok(Value::Null);
    };
    #[cfg(desktop)]
    if let Ok(path) = file.clone().into_path() {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".into());
        return Ok(serde_json::json!({"path":path,"name":name}));
    }
    // Mobile providers expose content:// or security-scoped URLs. Resolve a
    // provider handle in Rust and stream to private staging; no file bytes cross IPC.
    let (staged, name) = tauri::async_runtime::spawn_blocking(move || {
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|error| local_error("CACHE_DIRECTORY", error))?
            .join("imports");
        let name = match &file {
            tauri_plugin_fs::FilePath::Path(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "upload".into()),
            tauri_plugin_fs::FilePath::Url(url) => url
                .path_segments()
                .and_then(|mut parts| parts.next_back())
                .filter(|name| !name.is_empty())
                .unwrap_or("upload")
                .to_owned(),
        };
        let mut options = tauri_plugin_fs::OpenOptions::new();
        options.read(true);
        let mut source = app
            .fs()
            .open(file.clone(), options)
            .map_err(|error| local_error("PROVIDER_OPEN", error))?;
        #[cfg(target_os = "ios")]
        let _resource = SecurityScopedResource {
            app: app.clone(),
            file: file.clone(),
        };
        let staged = zxs_app_bridge::stage_import(&directory, &mut source)?;
        Ok::<_, BridgeError>((staged, name))
    })
    .await
    .map_err(|error| local_error("PICKER_IMPORT", error))??;
    let path = staged.keep()?;
    Ok(serde_json::json!({"path":path,"name":name}))
}

#[cfg(target_os = "ios")]
struct SecurityScopedResource {
    app: tauri::AppHandle,
    file: tauri_plugin_fs::FilePath,
}
#[cfg(target_os = "ios")]
impl Drop for SecurityScopedResource {
    fn drop(&mut self) {
        let _ = self
            .app
            .fs()
            .stop_accessing_security_scoped_resource(self.file.clone());
    }
}

#[tauri::command]
async fn connect(
    bridge: State<'_, Bridge>,
    endpoint: String,
    username: String,
    password: String,
) -> Result<Value, BridgeError> {
    bridge.connect(&endpoint, &username, &password).await
}
#[tauri::command]
async fn commit_workspace(
    bridge: State<'_, Bridge>,
    workspace: String,
    operation_id: String,
    message: String,
    expected_head: String,
) -> Result<Value, CommitError> {
    bridge
        .commit_workspace(&workspace, &operation_id, &message, &expected_head)
        .await
}
#[tauri::command]
async fn api_request(
    bridge: State<'_, Bridge>,
    method: String,
    path: String,
    body: Option<Value>,
) -> Result<Value, BridgeError> {
    bridge.request(&method, &path, body.as_ref()).await
}
#[tauri::command]
async fn upload_file(
    app: tauri::AppHandle,
    bridge: State<'_, Bridge>,
    workspace: String,
    item: String,
    path: String,
    source: String,
) -> Result<Value, BridgeError> {
    let result = bridge
        .upload(&workspace, &item, &path, std::path::Path::new(&source))
        .await?;
    if let Ok(cache) = app.path().app_cache_dir() {
        if let (Ok(imports), Ok(source)) = (
            cache.join("imports").canonicalize(),
            std::path::Path::new(&source).canonicalize(),
        ) {
            if source.starts_with(imports) {
                let _ = std::fs::remove_file(source);
            }
        }
    }
    Ok(result)
}
#[tauri::command]
async fn download_file(
    app: tauri::AppHandle,
    bridge: State<'_, Bridge>,
    version: String,
    name: String,
) -> Result<Value, BridgeError> {
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| local_error("CACHE_DIRECTORY", error))?;
    let target = zxs_app_bridge::prepare_cache_target(&directory, &version, &name)?;
    bridge.download(&version, &target).await
}

#[tauri::command]
async fn open_cached_file(
    app: tauri::AppHandle,
    bridge: State<'_, Bridge>,
    version: String,
    name: String,
) -> Result<Value, BridgeError> {
    let result =
        match download_file(app.clone(), bridge.clone(), version.clone(), name.clone()).await {
            Ok(value) => value,
            Err(error) if error.code == "DESTINATION_EXISTS" => {
                let cache = app
                    .path()
                    .app_cache_dir()
                    .map_err(|error| local_error("CACHE_DIRECTORY", error))?;
                let path = zxs_app_bridge::cache_target(&cache, &version, &name)?;
                let real = path
                    .canonicalize()
                    .map_err(|error| local_error("CACHE_ENTRY", error))?;
                if !real.starts_with(
                    cache
                        .canonicalize()
                        .map_err(|error| local_error("CACHE_DIRECTORY", error))?,
                ) {
                    return Err(local_error("CACHE_SCOPE", "缓存路径越界"));
                }
                bridge.verify_cached(&version, &real).await?;
                serde_json::json!({"target":real,"verified":true})
            }
            Err(error) => return Err(error),
        };
    let path = result["target"]
        .as_str()
        .ok_or_else(|| local_error("CACHE_ENTRY", "下载结果缺少文件路径"))?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|error| local_error("OPEN_FILE", error))?;
    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Bridge::default())
        .invoke_handler(tauri::generate_handler![
            connect,
            api_request,
            upload_file,
            commit_workspace,
            download_file,
            choose_file,
            open_cached_file
        ])
        .run(tauri::generate_context!())
        .expect("zxs application runtime failed");
}
