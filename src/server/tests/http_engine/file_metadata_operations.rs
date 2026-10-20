use super::*;

async fn branch(app: &Router, token: &str, file: &str, version: &str, name: &str) -> String {
    success(app,token,"POST",&format!("/api/file-manager/files/{file}/branches"),Some(json!({"source_version_id":version,"source_node_id":version,"name":name,"operation_id":new_id("op")}))).await["result"]["branch_id"].as_str().unwrap().into()
}
async fn select(app: &Router, token: &str, file: &str, branch: &str) {
    success(
        app,
        token,
        "POST",
        &format!("/api/file-manager/files/{file}/select"),
        Some(json!({"branch_id":branch,"operation_id":new_id("op")})),
    )
    .await;
}
async fn history(app: &Router, token: &str, file: &str) -> Value {
    success(
        app,
        token,
        "GET",
        &format!("/api/file-manager/files/{file}/history"),
        None,
    )
    .await
}

#[tokio::test]
async fn tags_and_file_merges_publish_reviewed_node_graphs_without_new_bodies() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "report.txt", b"base body").await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/api/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let file = listing["entries"][0]["file_id"].as_str().unwrap();
    let base = listing["entries"][0]["version_id"].as_str().unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let left = branch(&app, &token, file, base, "left").await;
    let right = branch(&app, &token, file, base, "right").await;
    select(&app, &token, file, &left).await;
    let tag_route = format!("/api/file-manager/files/{file}/tags");
    let tagged=success(&app,&token,"POST",&tag_route,Some(json!({"targets":[{"branch_id":left,"source_node_id":base,"expected_node_id":base}],"add":["已审核","work"],"remove":[],"operation_id":new_id("op")}))).await;
    let tag_node = tagged["result"]["nodes"][0]["node_id"].as_str().unwrap();
    assert!(tag_node.starts_with("fmeta_"));
    let initial = history(&app, &token, file).await;
    assert_eq!(initial["versions"].as_array().unwrap().len(), 1);
    assert_eq!(initial["node_id"], tag_node);
    let snapshot = initial["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_id"] == tag_node)
        .unwrap();
    assert_eq!(snapshot["kind"], "tags");
    assert_eq!(snapshot["tags"], json!(["work", "已审核"]));
    assert_eq!(snapshot["parent_node_ids"], json!([base]));
    success(
        &app,
        &token,
        "PATCH",
        &format!("/api/file-manager/files/{file}/nodes/{tag_node}"),
        Some(json!({"note":"独立标签备注","operation_id":new_id("op")})),
    )
    .await;
    success(&app,&token,"POST","/api/file-manager/disks/P01/namespace",Some(json!({"kind":"rename","path":"report.txt","destination":"renamed.txt","operation_id":new_id("op")}))).await;
    let renamed = history(&app, &token, file).await;
    let path_node = renamed["node_id"].as_str().unwrap();
    manager_upload(&app, &token, "renamed.txt", b"left body").await;
    let written = history(&app, &token, file).await;
    let left_node = written["node_id"].as_str().unwrap();
    assert_eq!(written["nodes"][0]["tags"], json!(["work", "已审核"]));
    assert_eq!(written["nodes"][0]["parent_node_ids"], json!([path_node]));
    let historical=success(&app,&token,"POST",&tag_route,Some(json!({"targets":[{"branch_id":left,"source_node_id":tag_node,"expected_node_id":left_node}],"add":["归档"],"remove":["work"],"operation_id":new_id("op")}))).await;
    assert_eq!(historical["result"]["nodes"][0]["advanced"], false);
    assert_eq!(history(&app, &token, file).await["node_id"], left_node);
    let before_invalid = history(&app, &token, file).await;
    for bad in [" bad".to_string(), "bad\nlabel".into(), "文".repeat(43)] {
        assert_eq!(send(&app,Some(&token),"POST",&tag_route,Some(json!({"targets":[{"branch_id":left,"source_node_id":left_node,"expected_node_id":left_node}],"add":[bad],"remove":[],"operation_id":new_id("op")}))).await.0,StatusCode::BAD_REQUEST);
    }
    assert_eq!(history(&app, &token, file).await, before_invalid);
    select(&app, &token, file, &right).await;
    manager_upload(&app, &token, "renamed.txt", b"right body").await;
    let right_history = history(&app, &token, file).await;
    let right_node = right_history["node_id"].as_str().unwrap();
    let merge_route = format!("/api/file-manager/files/{file}/merge");
    let request = json!({"inputs":[{"branch_id":left,"expected_node_id":left_node},{"branch_id":right,"expected_node_id":right_node}],"chosen_node_id":tag_node,"surviving_branch_id":left,"confirmed":true,"repository_id":repo,"path":"renamed.txt","operation_id":new_id("op")});
    let mut unconfirmed = request.clone();
    unconfirmed["confirmed"] = false.into();
    assert_eq!(
        send(&app, Some(&token), "POST", &merge_route, Some(unconfirmed))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let mut stale = request.clone();
    stale["inputs"][1]["expected_node_id"] = base.into();
    assert_eq!(
        send(&app, Some(&token), "POST", &merge_route, Some(stale))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let merged = success(&app, &token, "POST", &merge_route, Some(request.clone())).await;
    assert_eq!(
        success(&app, &token, "POST", &merge_route, Some(request)).await["result"],
        merged["result"]
    );
    let node = merged["result"]["node_id"].as_str().unwrap();
    assert!(node.starts_with("fmeta_"));
    let after = history(&app, &token, file).await;
    assert_eq!(after["versions"].as_array().unwrap().len(), 3);
    assert_eq!(after["node_id"], node);
    assert_eq!(after["version_id"], base);
    let result = after["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["node_id"] == node)
        .unwrap();
    assert_eq!(result["kind"], "merge");
    assert_eq!(
        result["path"], "report.txt",
        "merge metadata describes its chosen source, independent of parent UUID order"
    );
    assert_eq!(result["tags"], json!(["work", "已审核"]));
    let alias = after["branches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["branch_id"] == right)
        .unwrap();
    assert_eq!(alias["read_only"], true);
    assert_eq!(alias["alias_of"], left);
    assert_eq!(alias["node_id"], right_node);
    assert_eq!(after["merges"].as_array().unwrap().len(), 1);
    assert_eq!(version_bytes(&app, &token, base).await, b"base body");
    assert_eq!(
        send(
            &app,
            Some(&token),
            "POST",
            &format!("/api/file-manager/files/{file}/select"),
            Some(json!({"branch_id":right,"operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(
            &app,
            Some(&token),
            "PATCH",
            &format!("/api/file-manager/files/{file}/branches/{right}"),
            Some(json!({"name":"forbidden label","operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    success(&app,&token,"POST","/api/admin/accounts",Some(json!({"username":"node-reader","password":"node-reader-test-password","repositories":[repo]}))).await;
    success(&app,&token,"PUT",&format!("/api/repositories/{repo}/grants"),Some(json!({"grants":{"read":["node-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "node-reader", "node-reader-test-password").await;
    assert_eq!(history(&app, &reader, file).await["node_id"], node);
    assert_eq!(
        send(
            &app,
            Some(&reader),
            "POST",
            &tag_route,
            Some(json!({"targets":[],"add":[],"remove":[],"operation_id":new_id("op")}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let audit = success(&app, &token, "GET", "/api/file-manager/history", None).await;
    for action in ["file.branches.merge", "file.tags.update"] {
        assert!(
            audit["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["action"] == action)
        );
    }
    capture_wire("file-merge-tag-history.json", &after);
    drop(app);
    let engine = reopen_after_router_drop(&root.path().join("system")).await;
    assert_eq!(engine.file_branch_head(&left).unwrap(), node);
}

#[tokio::test]
async fn scoped_backup_configuration_checks_scope_identity_generation_and_manage_grants() {
    let root = tempfile::tempdir().unwrap();
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "docs/nested/report.txt", b"first body").await;
    manager_upload(&app, &token, "docs/nested/report.txt", b"second body").await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/api/file-manager/disks/P01/entries?path=docs/nested",
        None,
    )
    .await;
    let file = listing["entries"][0]["file_id"].as_str().unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let route = format!("/api/repositories/{repo}/backup-configuration");
    let before = success(&app, &token, "GET", &format!("{route}?path=docs"), None).await;
    assert_eq!(before["can_manage"], true);
    assert_eq!(before["generation"], "0");
    let request = json!({"path":"docs","target_copies":3,"backup_prefixes":{"B01":"工作备份","C01":"archive/docs"},"expected_generation":"0","expected_scope_id":before["scope"]["id"],"operation_id":new_id("op")});
    let configured = success(&app, &token, "PUT", &route, Some(request.clone())).await;
    assert_eq!(
        success(&app, &token, "PUT", &route, Some(request.clone())).await["result"],
        configured["result"]
    );
    let after = success(&app, &token, "GET", &format!("{route}?path=docs"), None).await;
    assert_eq!(after["generation"], "1");
    assert_eq!(after["target_copies"], 3);
    let inherited = success(
        &app,
        &token,
        "GET",
        &format!("{route}?path=docs/nested/report.txt"),
        None,
    )
    .await;
    assert_eq!(inherited["generation"], "0");
    assert_eq!(inherited["effective_generation"], "1");
    assert_eq!(inherited["inherited"], true);
    assert_eq!(
        inherited["backup_paths"]["B01"],
        "工作备份/nested/report.txt"
    );
    assert_eq!(
        inherited["backup_paths"]["C01"],
        "archive/docs/nested/report.txt"
    );
    let mut stale = request.clone();
    stale["operation_id"] = new_id("op").into();
    assert_eq!(
        send(&app, Some(&token), "PUT", &route, Some(stale)).await.0,
        StatusCode::CONFLICT
    );
    let mut wrong = request.clone();
    wrong["operation_id"] = new_id("op").into();
    wrong["expected_scope_id"] = new_id("entry").into();
    wrong["expected_generation"] = "1".into();
    assert_eq!(
        send(&app, Some(&token), "PUT", &route, Some(wrong)).await.0,
        StatusCode::CONFLICT
    );
    for prefixes in [
        json!({"B01":"../bad","C01":"archive"}),
        json!({"B01":"missing C"}),
    ] {
        assert_eq!(send(&app,Some(&token),"PUT",&route,Some(json!({"path":"docs","target_copies":3,"backup_prefixes":prefixes,"expected_generation":"1","expected_scope_id":before["scope"]["id"],"operation_id":new_id("op")}))).await.0,StatusCode::BAD_REQUEST);
    }
    assert_eq!(
        success(&app, &token, "GET", &format!("{route}?path=docs"), None).await,
        after
    );
    manager_upload(&app, &token, "docs/new.txt", b"new body inherits prefixes").await;
    let new_file = success(
        &app,
        &token,
        "GET",
        &format!("{route}?path=docs/new.txt"),
        None,
    )
    .await;
    assert_eq!(new_file["backup_paths"]["B01"], "工作备份/new.txt");
    success(&app,&token,"POST","/api/admin/accounts",Some(json!({"username":"backup-reader","password":"backup-reader-test-password","repositories":[repo]}))).await;
    success(&app,&token,"PUT",&format!("/api/repositories/{repo}/grants"),Some(json!({"grants":{"read":["backup-reader"],"write":[],"manage":["admin"]},"operation_id":new_id("op")}))).await;
    let reader = login(&app, "backup-reader", "backup-reader-test-password").await;
    assert_eq!(
        success(&app, &reader, "GET", &format!("{route}?path=docs"), None).await["can_manage"],
        false
    );
    assert_eq!(
        send(&app, Some(&reader), "PUT", &route, Some(request))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    capture_wire("scoped-backup-configuration.json", &inherited);
    drop(app);
    let engine = reopen_after_router_drop(&root.path().join("system")).await;
    assert_eq!(engine.file_history(file).unwrap().len(), 2);
    assert_eq!(
        engine.state().file_backup_paths[file]["B01"],
        "工作备份/nested/report.txt"
    );
}

#[tokio::test]
async fn contextual_branch_selection_preserves_current_file_when_target_body_is_lost() {
    let root = tempfile::tempdir().unwrap();
    let system = root.path().join("system");
    let app = application(root.path());
    let token = login(&app, "admin", "integration-test-password").await;
    manager_upload(&app, &token, "report.txt", b"original body").await;
    let listing = success(
        &app,
        &token,
        "GET",
        "/api/file-manager/disks/P01/entries",
        None,
    )
    .await;
    let file = listing["entries"][0]["file_id"].as_str().unwrap();
    let version = listing["entries"][0]["version_id"].as_str().unwrap();
    let repo = listing["context"]["repository_id"].as_str().unwrap();
    let original_branch = listing["entries"][0]["selected_branch_id"]
        .as_str()
        .unwrap();
    let other = branch(&app, &token, file, version, "other").await;
    select(&app, &token, file, &other).await;
    manager_upload(&app, &token, "report.txt", b"still readable body").await;
    let before = history(&app, &token, file).await;
    drop(app);
    let engine = reopen_after_router_drop(&system).await;
    let history_id = &engine.state().versions[version].history_tree_id;
    for disk in &engine.config().disks {
        let directory = disk
            .root
            .join("histories")
            .join(history_id)
            .join("versions")
            .join(version);
        if directory.exists() {
            std::fs::remove_file(fixture_payload(&directory, engine.state())).unwrap();
        }
    }
    let backend = Arc::new(EngineBackend::new(engine));
    let auth = backend.open_auth(&system).unwrap();
    let app = storage_server::router(backend, auth, ServerConfig::loopback(&system)).unwrap();
    let token = login(&app, "admin", "integration-test-password").await;
    let route = format!("/api/file-manager/files/{file}/select");
    for contextual in [false, true] {
        let mut request = json!({"branch_id":original_branch,"operation_id":new_id("op")});
        if contextual {
            request["repository_id"] = repo.into();
            request["path"] = "report.txt".into();
        }
        let (status, response) = send(&app, Some(&token), "POST", &route, Some(request)).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{response}");
        assert_eq!(response["error"]["code"], "CONTENT_UNAVAILABLE");
    }
    let after = history(&app, &token, file).await;
    assert_eq!(after["selected_branch_id"], before["selected_branch_id"]);
    assert_eq!(after["current_node_id"], before["current_node_id"]);
    assert_eq!(after["versions"], before["versions"]);
}
