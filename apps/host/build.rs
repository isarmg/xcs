fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "connect",
            "api_request",
            "upload_file",
            "download_file",
            "choose_file",
            "open_cached_file",
            "commit_workspace",
        ]),
    ))
    .expect("zxs application permissions could not be generated");
}
