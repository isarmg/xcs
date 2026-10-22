//! Structural guard for the management-only browser application. Behavioral
//! metadata and session regression tests live alongside their modules.
use std::path::Path;

fn sources(directory: &Path, output: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push((
                path.display().to_string(),
                std::fs::read_to_string(path).unwrap(),
            ));
        }
    }
}

#[test]
fn application_has_no_file_content_transport_or_picker_entry_points() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut application = Vec::new();
    sources(&root.join("src"), &mut application);
    for (path, source) in &application {
        for forbidden in [
            "application/octet-stream",
            "/content\"",
            "/content?",
            "X-ZXS-Content-Hash",
            "X-ZXS-Upload-Generation",
            "UploadSource",
            "web_sys::File",
            "type=\"file\"",
            "array_buffer(",
            "create_object_url",
            "set_download(",
            "preview_version",
            "pick_upload",
            "upload_files",
            "mod transfers",
            "mod transfer;",
            "upload_picker",
            "preview_generation",
        ] {
            assert!(!source.contains(forbidden), "{path} contains {forbidden}");
        }
    }
    for removed in [
        "src/transfers",
        "src/files/actions/transfer.rs",
        "src/api/upload_source.rs",
    ] {
        // An empty checkout directory is harmless; no module or file may return.
        let path = root.join(removed);
        assert!(!path.is_file(), "retired module returned: {removed}");
        if path.is_dir() {
            assert!(std::fs::read_dir(path).unwrap().next().is_none());
        }
    }
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(!manifest.lines().any(|line| line.starts_with("blake3 =")));
    for feature in ["\"File\"", "\"FileList\"", "\"HtmlAnchorElement\""] {
        assert!(!manifest.contains(feature));
    }
}

#[test]
fn management_views_and_bounded_json_transport_remain() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let api = std::fs::read_to_string(root.join("api.rs")).unwrap();
    for required in [
        "application/json",
        "MAX_RESPONSE_BYTES",
        "reader.cancel()",
        "RequestMode::SameOrigin",
        "RequestRedirect::Error",
        "X-CSRF-Token",
    ] {
        assert!(
            api.contains(required),
            "missing transport protection {required}"
        );
    }
    let detail = std::fs::read_to_string(root.join("files/detail.rs")).unwrap();
    for required in [
        "RenameEntry",
        "Kind::Delete",
        "backup::view",
        "FileNote",
        "select_branch",
        "merge::view",
        "tags::view",
        "version_tree::view",
    ] {
        assert!(
            detail.contains(required),
            "missing metadata workflow {required}"
        );
    }
    let view = std::fs::read_to_string(root.join("files/view.rs")).unwrap();
    for required in [
        "disk_roots",
        "go_directory",
        "file_table",
        "batch::view",
        "directory_branch",
    ] {
        assert!(
            view.contains(required),
            "missing browsing workflow {required}"
        );
    }
}
