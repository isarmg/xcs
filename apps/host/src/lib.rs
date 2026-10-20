mod provider;
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
async fn choose_files(
    app: tauri::AppHandle,
    bridge: State<'_, Bridge>,
    multiple: bool,
) -> Result<Value, BridgeError> {
    let revision = bridge.work_revision().await?;
    let (send, receive) = tokio::sync::oneshot::channel();
    if multiple {
        app.dialog().file().pick_files(move |files| {
            let _ = send.send(files.unwrap_or_default());
        });
    } else {
        app.dialog().file().pick_file(move |file| {
            let _ = send.send(file.into_iter().collect());
        });
    }
    let files = receive
        .await
        .map_err(|error| local_error("PICKER_CLOSED", error))?;
    if files.len() > 128 {
        return Err(local_error("PICKER_LIMIT", "Select no more than 128 files"));
    }
    let mut sources = Vec::with_capacity(files.len());
    for file in files {
        check_work_revision(&bridge, revision).await?;
        sources.push(import_selection(app.clone(), file).await?);
    }
    let selected = bridge.register_sources(revision, sources).await?;
    serde_json::to_value(selected).map_err(|error| local_error("PICKER_RESPONSE", error))
}

fn selection_name(path: &std::path::Path) -> Result<String, BridgeError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            local_error(
                "INVALID_FILENAME",
                "The selected filename must be valid UTF-8",
            )
        })
}

async fn import_selection(
    app: tauri::AppHandle,
    file: tauri_plugin_fs::FilePath,
) -> Result<zxs_app_bridge::PickedSource, BridgeError> {
    // Only a real filesystem path has an authoritative filename. A content://
    // document ID is not a display name and must never become a remote filename.
    let path = file.clone().into_path().map_err(|_| local_error(
        "PROVIDER_FILENAME_UNAVAILABLE",
        "This file provider does not supply a reliable filename. Use the browser or desktop App to upload this file.",
    ))?;
    let name = selection_name(&path)?;
    if cfg!(desktop) {
        return zxs_app_bridge::PickedSource::from_path(&path, name).await;
    }
    // Provider bytes remain in Rust. The staged import keeps RAII ownership
    // through registration, cancellation, session replacement, and upload.
    let (staged, name) = tauri::async_runtime::spawn_blocking(move || {
        let directory = app
            .path()
            .app_cache_dir()
            .map_err(|error| local_error("CACHE_DIRECTORY", error))?
            .join("imports");
        let staged = provider::with_cleanup(
            || {
                let mut options = tauri_plugin_fs::OpenOptions::new();
                options.read(true);
                let mut source = app
                    .fs()
                    .open(file.clone(), options)
                    .map_err(|error| local_error("PROVIDER_OPEN", error))?;
                zxs_app_bridge::stage_import(&directory, &mut source)
            },
            || {
                // Fs::open starts iOS scoped access before its fallible filesystem
                // open. Install cleanup before calling it, not after `open?`.
                #[cfg(target_os = "ios")]
                if matches!(&file, tauri_plugin_fs::FilePath::Url(url) if url.scheme() == "file") {
                    let _ = app
                        .fs()
                        .stop_accessing_security_scoped_resource(file.clone());
                }
            },
        )?;
        Ok::<_, BridgeError>((staged, name))
    })
    .await
    .map_err(|error| local_error("PICKER_IMPORT", error))??;
    zxs_app_bridge::PickedSource::from_import(staged, name).await
}

#[tauri::command]
fn release_sources(bridge: State<'_, Bridge>, source_ids: Vec<String>) {
    bridge.release_sources(&source_ids);
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
    bridge: State<'_, Bridge>,
    workspace: String,
    item: String,
    path: String,
    source_id: String,
) -> Result<Value, BridgeError> {
    bridge
        .upload_selected(&workspace, &item, &path, &source_id)
        .await
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
    let revision = bridge.work_revision().await?;
    let result =
        match download_file(app.clone(), bridge.clone(), version.clone(), name.clone()).await {
            Ok(value) => value,
            Err(error) if error.code == "DESTINATION_EXISTS" => {
                check_work_revision(&bridge, revision).await?;
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
    check_work_revision(&bridge, revision).await?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|error| local_error("OPEN_FILE", error))?;
    Ok(result)
}

async fn check_work_revision(bridge: &Bridge, expected: u64) -> Result<(), BridgeError> {
    if bridge.work_revision().await? == expected {
        Ok(())
    } else {
        Err(local_error(
            "SESSION_CHANGED",
            "The session changed before opening the file",
        ))
    }
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
            choose_files,
            release_sources,
            open_cached_file
        ])
        .run(tauri::generate_context!())
        .expect("zxs application runtime failed");
}
