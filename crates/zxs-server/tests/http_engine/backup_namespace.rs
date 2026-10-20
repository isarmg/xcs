//! Real HTTP backup browsing uses user prefixes and retained per-slot bodies.
use super::*;

fn listing_path(slot: &str, path: &str) -> String {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("path", path)
        .finish();
    format!("/api/file-manager/disks/{slot}/entries?{query}")
}
async fn listing(app: &Router, token: &str, slot: &str, path: &str) -> Value {
    success(app, token, "GET", &listing_path(slot, path), None).await
}
async fn configure(app: &Router, token: &str, repo: &str, path: &str, b: &str, c: &str) -> Value {
    let route = format!("/api/repositories/{repo}/backup-configuration");
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("path", path)
        .finish();
    let old = success(app, token, "GET", &format!("{route}?{query}"), None).await;
    success(app,token,"PUT",&route,Some(json!({"path":path,"target_copies":3,"backup_prefixes":{"B01":b,"C01":c},
        "expected_scope_id":old["scope"]["id"],"expected_generation":old["generation"],"operation_id":new_id("op")}))).await
}
fn read_only_listing(value: &Value) {
    assert_eq!(value["backup"], true);
    assert_eq!(value["writable"], false);
    assert!(value["context"].is_null());
    assert_eq!(value["directory_branches"], json!([]));
    assert_eq!(value["directory_branches_supported"], false);
    assert_eq!(value["default_branch_scope"], false);
    for entry in value["entries"].as_array().unwrap() {
        assert_eq!(entry["read_only"], true);
        assert_eq!(entry["backup"], true);
    }
}

#[tokio::test]
async fn user_backup_prefixes_keep_fixed_paths_and_slot_versions_after_primary_rename_and_delete() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &admin, "docs/nested/report.txt", b"first stored body").await;
    manager_upload(
        &app,
        &admin,
        "docs/nested/report.txt",
        b"second stored body",
    )
    .await;
    let primary = listing(&app, &admin, "P01", "docs/nested").await;
    let file = primary["entries"][0]["file_id"].as_str().unwrap();
    let version = primary["entries"][0]["version_id"].as_str().unwrap();
    let repo = primary["context"]["repository_id"].as_str().unwrap();
    let configured = configure(&app, &admin, repo, "docs", "WorkBackup", "冷备份/docs").await;
    let configuration = success(
        &app,
        &admin,
        "GET",
        &format!("/api/repositories/{repo}/backup-configuration?path=docs"),
        None,
    )
    .await;
    assert_eq!(
        configuration["backup_routes"],
        json!([["P01", "B01", "C01"]])
    );
    assert_eq!(configuration["backup_slot_ids"], json!(["B01", "C01"]));
    capture_wire("backup-prefix-configuration.json", &configuration);
    for (slot, prefix, leaf) in [
        ("B01", "WorkBackup", "WorkBackup/nested"),
        ("C01", "冷备份", "冷备份/docs/nested"),
    ] {
        let top = listing(&app, &admin, slot, "").await;
        read_only_listing(&top);
        assert_eq!(top["entries"].as_array().unwrap().len(), 1);
        assert_eq!(top["entries"][0]["name"], prefix);
        assert_ne!(
            top["entries"][0]["name"], "P01",
            "a configured prefix is not hidden behind a primary-slot directory"
        );
        let child = listing(&app, &admin, slot, leaf).await;
        read_only_listing(&child);
        assert_eq!(child["entries"][0]["file_id"], file);
        assert_eq!(child["entries"][0]["version_id"], version);
        assert_eq!(
            child["entries"][0]["repository_path"],
            "docs/nested/report.txt"
        );
    }
    // The legacy policy update affects later writes only. Existing AVAILABLE
    // v1/v2 remain officially placed on B/C, while the new body is P-only.
    let policy = configured["result"]["policy"]["policy_id"]
        .as_str()
        .unwrap();
    success(
        &app,
        &admin,
        "PUT",
        &format!("/api/policies/{policy}"),
        Some(
            json!({"repository_id":repo,"path_prefix":"docs","target_copies":1,
        "expected_generation":"1","operation_id":new_id("op")}),
        ),
    )
    .await;
    manager_upload(
        &app,
        &admin,
        "docs/nested/report.txt",
        b"primary body without B or C placement",
    )
    .await;
    let current = listing(&app, &admin, "P01", "docs/nested").await;
    let current_version = current["entries"][0]["version_id"].as_str().unwrap();
    assert_ne!(current_version, version);
    for (path, destination) in [
        ("docs", "archive"),
        ("archive/nested/report.txt", "archive/nested/renamed.txt"),
    ] {
        success(&app,&admin,"POST","/api/file-manager/disks/P01/namespace",Some(json!({"kind":"rename","path":path,"destination":destination,"operation_id":new_id("op")}))).await;
    }
    let renamed = listing(&app, &admin, "P01", "archive/nested").await;
    assert_eq!(renamed["entries"][0]["version_id"], current_version);
    assert!(
        renamed["entries"][0]["node_id"]
            .as_str()
            .unwrap()
            .starts_with("fnode_")
    );
    let backup = listing(&app, &admin, "B01", "WorkBackup/nested").await;
    read_only_listing(&backup);
    let entry = &backup["entries"][0];
    assert_eq!(entry["path"], "WorkBackup/nested/report.txt");
    assert_eq!(entry["name"], "report.txt");
    assert_eq!(entry["repository_id"], repo);
    assert_eq!(entry["repository_path"], "archive/nested/renamed.txt");
    assert_eq!(entry["historical_only"], false);
    assert_eq!(entry["version_id"], version);
    assert_eq!(entry["node_id"], version);
    let history=success(&app,&admin,"GET",&format!("/api/file-manager/files/{file}/history?repository_id={repo}&path=archive/nested/renamed.txt"),None).await;
    assert_eq!(history["version_id"], current_version);
    assert!(
        history["versions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|body| body["version_id"] == version)
    );
    assert_eq!(
        version_bytes(&app, &admin, version).await,
        b"second stored body"
    );
    assert_eq!(
        version_bytes(&app, &admin, current_version).await,
        b"primary body without B or C placement"
    );
    let cold = listing(&app, &admin, "C01", "冷备份/docs/nested").await;
    assert_eq!(cold["entries"][0]["path"], "冷备份/docs/nested/report.txt");
    assert_eq!(cold["entries"][0]["version_id"], version);
    for (route, body) in [
        (
            "write-sessions",
            json!({"path":"WorkBackup/nested/report.txt"}),
        ),
        (
            "directory-branches",
            json!({"path":"WorkBackup/nested","name":"forbidden","operation_id":new_id("op")}),
        ),
        (
            "namespace",
            json!({"kind":"delete","path":"WorkBackup/nested/report.txt","operation_id":new_id("op")}),
        ),
    ] {
        assert_eq!(
            send(
                &app,
                Some(&admin),
                "POST",
                &format!("/api/file-manager/disks/B01/{route}"),
                Some(body)
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    success(&app,&admin,"POST","/api/file-manager/disks/P01/namespace",Some(json!({"kind":"delete","path":"archive/nested/renamed.txt","operation_id":new_id("op")}))).await;
    let retained = listing(&app, &admin, "B01", "WorkBackup/nested").await;
    read_only_listing(&retained);
    assert_eq!(retained["entries"][0]["historical_only"], true);
    assert!(retained["entries"][0]["repository_path"].is_null());
    assert_eq!(retained["entries"][0]["version_id"], version);
    assert_eq!(retained["entries"][0]["node_id"], version);
    let historical = success(
        &app,
        &admin,
        "GET",
        &format!("/api/file-manager/files/{file}/history"),
        None,
    )
    .await;
    assert!(historical["path"].is_null());
    assert_eq!(historical["versions"].as_array().unwrap().len(), 3);
    assert_eq!(
        version_bytes(&app, &admin, version).await,
        b"second stored body"
    );
    capture_wire("backup-prefix-files.json", &backup);
    capture_wire("backup-prefix-retained.json", &retained);
    drop(app);
    let engine = reopen_after_router_drop(&root.path().join("system")).await;
    assert_eq!(
        engine.state().file_backup_paths[file]["B01"],
        "WorkBackup/nested/report.txt"
    );
    assert_eq!(engine.file_history(file).unwrap().len(), 3);
}

#[tokio::test]
async fn backup_path_collisions_are_rejected_after_acl_filtering_without_overwriting_bodies() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let admin = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &admin, "docs/report.txt", b"publicly authorized body").await;
    let primary = listing(&app, &admin, "P01", "docs").await;
    let repo = primary["context"]["repository_id"].as_str().unwrap();
    let public_version = primary["entries"][0]["version_id"].as_str().unwrap();
    configure(&app, &admin, repo, "docs", "WorkBackup/nested", "冷备份").await;
    let private=success(&app,&admin,"POST","/api/repositories",Some(json!({"name":"Private","primary_slot_id":"P01","target_copies":3,"operation_id":new_id("op")}))).await;
    let private_repo = private["repository_id"].as_str().unwrap();
    manager_upload(
        &app,
        &admin,
        "Private/report.txt",
        b"private body must not leak",
    )
    .await;
    let private_entries = listing(&app, &admin, "P01", "Private").await;
    let private_version = private_entries["entries"][0]["version_id"]
        .as_str()
        .unwrap();
    configure(
        &app,
        &admin,
        private_repo,
        "report.txt",
        "workbackup/nested",
        "PrivateOnly",
    )
    .await;
    let (status, conflict) = send(
        &app,
        Some(&admin),
        "GET",
        &listing_path("B01", "WorkBackup/nested"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict["error"]["code"], "BACKUP_PATH_CONFLICT");
    assert_eq!(
        version_bytes(&app, &admin, public_version).await,
        b"publicly authorized body"
    );
    assert_eq!(
        version_bytes(&app, &admin, private_version).await,
        b"private body must not leak"
    );
    success(&app,&admin,"POST","/api/admin/accounts",Some(json!({"username":"backup-viewer","password":"backup-viewer-test-password","repositories":[repo]}))).await;
    success(&app,&admin,"PUT",&format!("/api/repositories/{repo}/grants"),Some(json!({"grants":{"read":["backup-viewer"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "backup-viewer", "backup-viewer-test-password").await;
    let authorized = listing(&app, &reader, "B01", "WorkBackup/nested").await;
    read_only_listing(&authorized);
    assert_eq!(authorized["entries"].as_array().unwrap().len(), 1);
    assert_eq!(authorized["entries"][0]["version_id"], public_version);
    assert_eq!(
        version_bytes(&app, &reader, public_version).await,
        b"publicly authorized body"
    );
    let cold = listing(&app, &reader, "C01", "").await;
    assert!(
        cold["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["name"] != "PrivateOnly")
    );
    assert_eq!(
        send(
            &app,
            Some(&reader),
            "GET",
            &listing_path("C01", "PrivateOnly"),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, None, "GET", &listing_path("B01", ""), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/versions/{private_version}/content"))
                .header("authorization", format!("Bearer {reader}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // A file/directory-prefix collision is equally explicit; different stable
    // containers remain readable and the authorized-only projection is intact.
    configure(
        &app,
        &admin,
        private_repo,
        "report.txt",
        "WorkBackup/nested/report.txt",
        "PrivateOnly",
    )
    .await;
    let (status, conflict) = send(
        &app,
        Some(&admin),
        "GET",
        &listing_path("B01", "WorkBackup/nested"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(conflict["error"]["code"], "BACKUP_PATH_CONFLICT");
    assert_eq!(
        listing(&app, &reader, "B01", "WorkBackup/nested").await["entries"][0]["version_id"],
        public_version
    );
    capture_wire("backup-prefix-acl.json", &authorized);
    capture_wire("backup-prefix-conflict.json", &conflict);
}

#[tokio::test]
async fn readers_discover_historical_primary_routes_by_history_identity_and_all_required_backup_prefixes()
 {
    let root = tempfile::tempdir().unwrap();
    let mut config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    for (slot, level, parent) in [("P02", 1, None), ("B02", 2, Some("P02"))] {
        config.disks.push(zxs_contract::DiskConfig {
            slot_id: slot.into(),
            disk_id: new_id("disk"),
            root: root.path().join(slot),
            level,
            upstream_slot_id: parent.map(str::to_owned),
            metadata_member: false,
            binding_generation: 1,
            capacity_bytes: None,
        });
    }
    let app = configured_application(root.path(), config);
    let admin = login(&app, "admin", "integration-test-password").await;
    let repo=success(&app,&admin,"POST","/api/repositories",Some(json!({"name":"Mixed","primary_slot_id":"P01","target_copies":1,"operation_id":new_id("op")}))).await;
    let id = repo["repository_id"].as_str().unwrap();
    manager_upload(&app, &admin, "Mixed/a.txt", b"first primary").await;
    let first = listing(&app, &admin, "P01", "Mixed").await["entries"][0]["version_id"]
        .as_str()
        .unwrap()
        .to_owned();
    success(&app,&admin,"PUT",&format!("/api/primary-placement-rules/{}",new_id("rule")),Some(json!({"repository_id":id,
        "path_prefix":"a.txt","primary_slot_id":"P02","expected_generation":"0","operation_id":new_id("op")}))).await;
    manager_upload(&app, &admin, "Mixed/a.txt", b"second primary").await;
    let second = listing(&app, &admin, "P01", "Mixed").await["entries"][0]["version_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let route = format!("/api/repositories/{id}/backup-configuration");
    let before = success(&app, &admin, "GET", &format!("{route}?path=a.txt"), None).await;
    assert_eq!(
        before["backup_routes"],
        json!([["P01", "B01", "C01"], ["P02", "B02"]])
    );
    assert_eq!(before["backup_slot_ids"], json!(["B01", "B02", "C01"]));
    assert_eq!(before["max_copies"], 2);
    let body = json!({"path":"a.txt","target_copies":2,"backup_prefixes":{"B02":"second"},"expected_scope_id":before["scope"]["id"],
        "expected_generation":"0","operation_id":new_id("op")});
    let (status, error) = send(&app, Some(&admin), "PUT", &route, Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "BACKUP_PREFIX_REQUIRED");
    success(&app,&admin,"PUT",&route,Some(json!({"path":"a.txt","target_copies":2,"backup_prefixes":{"B01":"first","B02":"second"},
        "expected_scope_id":before["scope"]["id"],"expected_generation":"0","operation_id":new_id("op")}))).await;
    // This other history uses the same default P01, but its C01 route must not
    // grant visibility to a reader admitted only to Mixed.
    success(&app,&admin,"POST","/api/repositories",Some(json!({"name":"Hidden","primary_slot_id":"P01","target_copies":3,"operation_id":new_id("op")}))).await;
    manager_upload(&app, &admin, "Hidden/secret.txt", b"not admitted").await;
    success(&app,&admin,"POST","/api/admin/accounts",Some(json!({"username":"route-reader","password":"route-reader-test-password","repositories":[id]}))).await;
    success(&app,&admin,"PUT",&format!("/api/repositories/{id}/grants"),Some(json!({"grants":{"read":["route-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "route-reader", "route-reader-test-password").await;
    let disks = success(&app, &reader, "GET", "/api/file-manager/disks", None).await;
    let visible = disks["disks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|disk| disk["slot_id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(visible, ["P01", "P02", "B01", "B02"].into_iter().collect());
    let old = listing(&app, &reader, "B01", "first").await;
    read_only_listing(&old);
    assert_eq!(old["entries"][0]["version_id"], first);
    let new = listing(&app, &reader, "B02", "second").await;
    read_only_listing(&new);
    assert_eq!(new["entries"][0]["version_id"], second);
    assert_eq!(old["entries"][0]["repository_path"], "a.txt");
    assert_eq!(new["entries"][0]["repository_path"], "a.txt");
    assert_eq!(version_bytes(&app, &reader, &first).await, b"first primary");
    assert_eq!(
        version_bytes(&app, &reader, &second).await,
        b"second primary"
    );
    let actual = success(&app, &reader, "GET", &format!("{route}?path=a.txt"), None).await;
    assert_eq!(
        actual["backup_paths"],
        json!({"B01":"first/a.txt","B02":"second/a.txt"})
    );
    capture_wire("backup-mixed-routes.json", &actual);
    capture_wire("backup-route-disks.json", &disks);
}
