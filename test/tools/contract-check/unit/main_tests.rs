use super::*;
#[test]
fn checked_in_contracts_and_document_examples_are_consistent() {
    run(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
}

#[test]
fn old_branch_and_commit_canonical_bytes_survive_pending_field_defaults() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts");
    let vectors = read(&root.join("vectors.json")).unwrap();
    let value = array(&vectors["positive"])
        .unwrap()
        .iter()
        .find(|case| case["name"] == "legacy-branch-without-pending-map")
        .unwrap()["value"]
        .clone();
    let before = storage_contract::canonical_json(&value).unwrap();
    let branch: storage_contract::Branch = storage_contract::decode_strict(&before).unwrap();
    assert!(branch.pending_file_versions.is_empty());
    assert_eq!(storage_contract::canonical_json(&branch).unwrap(), before);
    assert_eq!(
        storage_contract::object_hash("branch", &branch).unwrap(),
        storage_contract::object_hash("branch", &value).unwrap()
    );
    let mut created = branch;
    created.pending_file_versions.insert(
        storage_contract::new_id("file"),
        storage_contract::new_id("ver"),
    );
    let encoded = storage_contract::canonical_json(&created).unwrap();
    let decoded: storage_contract::Branch = storage_contract::decode_strict(&encoded).unwrap();
    assert_eq!(decoded.pending_file_versions, created.pending_file_versions);
    assert_ne!(encoded, before);
    created.pending_file_versions.clear();
    assert_eq!(storage_contract::canonical_json(&created).unwrap(), before);
    let value = array(&vectors["positive"])
        .unwrap()
        .iter()
        .find(|case| case["name"] == "legacy-commit-without-pending-map")
        .unwrap()["value"]
        .clone();
    let before = storage_contract::canonical_json(&value).unwrap();
    let mut commit: storage_contract::Commit = storage_contract::decode_strict(&before).unwrap();
    assert!(commit.pending_file_versions.is_empty());
    assert_eq!(storage_contract::canonical_json(&commit).unwrap(), before);
    assert_eq!(
        storage_contract::object_hash("commit", &commit).unwrap(),
        storage_contract::object_hash("commit", &value).unwrap()
    );
    commit.pending_file_versions.insert(
        commit.entries[0].file_id.clone(),
        commit.entries[0].version_id.clone(),
    );
    let encoded = storage_contract::canonical_json(&commit).unwrap();
    let decoded: storage_contract::Commit = storage_contract::decode_strict(&encoded).unwrap();
    assert_eq!(decoded.pending_file_versions, commit.pending_file_versions);
    commit.pending_file_versions.clear();
    assert_eq!(storage_contract::canonical_json(&commit).unwrap(), before);
}

#[test]
fn inherited_file_manager_projection_distinguishes_visible_parent_from_own_content() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let mut api = read(&path).unwrap();
    let item = api["paths"]["/api/file-manager/disks/{slot}/entries"]["get"]["responses"]["200"]
        ["content"]["application/json"]["schema"]["properties"]["entries"]["items"]
        .clone();
    api["$defs"]["entry_projection"] = item;
    api["$ref"] = "#/$defs/entry_projection".into();
    let schema = validator(&api).unwrap();
    let source = storage_contract::new_id("ver");
    let inherited = json!({
        "entry_id":"entry-example","name":"a.txt","path":"a.txt","kind":"file",
        "container":true,"size":"12","file_id":storage_contract::new_id("file"),
        "version_id":source,"own_version_id":null,"inherited":true,"materialized":false,
        "content_state":"PENDING","visible_content_state":"AVAILABLE",
        "source_version_id":source,"selected_branch_id":storage_contract::new_id("br")
    });
    let accepts = |value: &Value| schema.is_valid(value) && validate_content_view(value).is_ok();
    assert!(accepts(&inherited));
    for (field, value) in [
        ("version_id", Value::Null),
        ("version_id", json!("")),
        ("version_id", json!(storage_contract::new_id("ver"))),
        ("own_version_id", json!(source)),
        ("inherited", json!(false)),
        ("source_version_id", Value::Null),
        ("content_state", json!("AVAILABLE")),
        ("visible_content_state", json!("PENDING")),
    ] {
        let mut invalid = inherited.clone();
        invalid[field] = value;
        assert!(!accepts(&invalid), "invalid inherited {field}");
    }
    let mut written = inherited.clone();
    let own = storage_contract::new_id("ver");
    written["materialized"] = true.into();
    written["inherited"] = false.into();
    written["version_id"] = own.clone().into();
    written["own_version_id"] = own.into();
    written["source_version_id"] = Value::Null;
    written["content_state"] = "AVAILABLE".into();
    written["size"] = "0".into();
    // A genuinely written zero-byte FULL is valid and distinguishable.
    assert!(accepts(&written));
    let mut wrong_own = written.clone();
    wrong_own["own_version_id"] = storage_contract::new_id("ver").into();
    assert!(!accepts(&wrong_own));
    let mut wrong_state = written.clone();
    wrong_state["visible_content_state"] = "PURGED".into();
    assert!(!accepts(&wrong_state));
    let mut purged_parent = inherited.clone();
    purged_parent["visible_content_state"] = "PURGED".into();
    assert!(accepts(&purged_parent));
    written["version_id"] = Value::Null;
    assert!(!accepts(&written));
    assert!(schema.is_valid(&json!({
        "entry_id":"virtual-directory","name":"P01","path":"P01","kind":"directory",
        "size":"0","file_id":"","version_id":""
    })));
    let creation_shape = api["paths"]["/api/file-manager/files/{file}/branches"]["post"]
        ["responses"]["200"]["content"]["application/json"]["schema"]["properties"]
        ["result"]
        .clone();
    let directory_branch_shape = api["paths"]
        ["/api/file-manager/disks/{slot}/directory-branches"]["post"]["responses"]["200"]
        ["content"]["application/json"]["schema"]["properties"]["result"]["properties"]
        ["branches"]["items"]
        .clone();
    api["$defs"]["creation"] = creation_shape;
    api["$defs"]["directory_branch_creation"] = directory_branch_shape;
    api["$ref"] = "#/$defs/creation".into();
    let creation_schema = validator(&api).unwrap();
    let creation = json!({
        "file_id":inherited["file_id"],"branch_id":inherited["selected_branch_id"],
        "commit_id":storage_contract::new_id("commit"),"history_tree_id":storage_contract::new_id("history"),
        "copied_files":"0","placeholder_files":"1","materialized":false,"selected":false,
        "inherited":true,"own_version_id":null,"version_id":source,
        "source_version_id":source,"content_state":"PENDING"
    });
    assert!(creation_schema.is_valid(&creation));
    let mut rewritten_receipt = creation.clone();
    rewritten_receipt["materialized"] = true.into();
    rewritten_receipt["inherited"] = false.into();
    rewritten_receipt["own_version_id"] = storage_contract::new_id("ver").into();
    assert!(!creation_schema.is_valid(&rewritten_receipt));
    api["$ref"] = "#/$defs/directory_branch_creation".into();
    let directory_branch_schema = validator(&api).unwrap();
    assert!(directory_branch_schema.is_valid(&json!({
        "file_id":inherited["file_id"],"branch_id":inherited["selected_branch_id"],"path":"a.txt",
        "materialized":false,"inherited":true,"own_version_id":null,
        "version_id":source,"source_version_id":source
    })));
}
#[test]
fn semantic_upload_overflow_is_rejected() {
    assert!(semantics("UploadStatus",&json!({"logical_size":u64::MAX.to_string(),"state":"COMPLETE","durable_ranges":[{"offset":u64::MAX.to_string(),"length":"1"}]})).is_err());
}
#[test]
fn openapi_duplicate_operation_ids_and_unbound_paths_are_rejected() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let mut value = read(&root).unwrap();
    value["paths"]["/api/workspaces/{workspace}/commit"]["post"]["operationId"] =
        "getContentSaveOperation".into();
    assert!(openapi(&value).is_err());
    let mut value = read(&root).unwrap();
    value["paths"]["/api/workspaces/{workspace}/commit"]["post"]["parameters"] = json!([]);
    assert!(openapi(&value).is_err());
}

#[test]
fn file_history_contract_requires_true_parent_arrays_without_limiting_multi_parent_edges() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let mut api = read(&path).unwrap();
    let version_pointer = "/paths/~1api~1file-manager~1files~1{file}~1history/get/responses/200/content/application~1json/schema/properties/versions/items";
    let schema = api.pointer(version_pointer).unwrap();
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("source_version_ids"))
    );
    assert_eq!(
        schema["properties"]["source_version_ids"]["uniqueItems"],
        true
    );
    assert!(
        schema["properties"]["source_version_ids"]
            .get("maxItems")
            .is_none()
    );
    // Route braces are legal OpenAPI path keys, but are not literal URI
    // fragment characters. Give the extracted response a plain local ref.
    api["$defs"]["file_history_version"] = schema.clone();
    api["$ref"] = "#/$defs/file_history_version".into();
    let schema = validator(&api).unwrap();
    let mut version = json!({
        "version_id":storage_contract::new_id("ver"),"file_id":storage_contract::new_id("file"),
        "source_version_ids":[],"branch_id":null,"created_at":"2026-10-20T00:00:00Z","message":"",
        "size":"1","sequence":"3","current":false,"content_state":"AVAILABLE",
        "content_hash":"a".repeat(64),"note":"","download_name":"a.txt",
        "storage_name":"v000001_a.txt"
    });
    assert!(schema.is_valid(&version));
    let mut obsolete = version.clone();
    obsolete["legacy"] = json!(false);
    assert!(!schema.is_valid(&obsolete));
    for timestamp in [Value::Null, json!(1792400000)] {
        obsolete = version.clone();
        obsolete["created_at"] = timestamp;
        assert!(!schema.is_valid(&obsolete));
    }
    let first = storage_contract::new_id("ver");
    let second = storage_contract::new_id("ver");
    version["source_version_ids"] = json!([first, second]);
    assert!(schema.is_valid(&version));
    version["source_version_ids"] = json!([first, first]);
    assert!(!schema.is_valid(&version));
    version["source_version_ids"] = Value::Null;
    assert!(!schema.is_valid(&version));
    version
        .as_object_mut()
        .unwrap()
        .remove("source_version_ids");
    assert!(!schema.is_valid(&version));
}

#[test]
fn note_writes_enforce_bytes_and_control_characters_without_rejecting_legacy_reads() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let api = read(&path).unwrap();
    for route in [
        "/api/file-manager/files/{file}/metadata",
        "/api/file-manager/versions/{version}/metadata",
    ] {
        let note = &api["paths"][route]["post"]["requestBody"]["content"]["application/json"]["schema"]
            ["properties"]["note"];
        let schema = validator(note).unwrap();
        // JSON Schema maxLength counts Unicode characters, not UTF-8 bytes.
        let byte_limit = note["x-maxUtf8Bytes"].as_u64().unwrap() as usize;
        assert_eq!(byte_limit, 128);
        let accepts = |value: &str| schema.is_valid(&json!(value)) && value.len() <= byte_limit;
        for valid in [
            String::new(),
            "a".repeat(128),
            format!("{}ab", "文".repeat(42)),
        ] {
            assert!(accepts(&valid), "valid note rejected: {route}");
        }
        for invalid in [
            "a".repeat(129),
            "文".repeat(43),
            "line\nnext".into(),
            "line\rnext".into(),
            "a\tb".into(),
            "a\0b".into(),
            "a\u{7f}b".into(),
            "a\u{85}b".into(),
        ] {
            assert!(!accepts(&invalid), "invalid note accepted: {route}");
        }
    }
    let entries = &api["paths"]["/api/file-manager/disks/{slot}/entries"]["get"]["responses"]["200"]
        ["content"]["application/json"]["schema"]["properties"]["entries"]["items"]["properties"]["note"];
    let history = &api["paths"]["/api/file-manager/files/{file}/history"]["get"]["responses"]["200"]
        ["content"]["application/json"]["schema"]["properties"];
    for note in [
        entries,
        &history["note"],
        &history["versions"]["items"]["properties"]["note"],
    ] {
        let schema = validator(note).unwrap();
        assert!(schema.is_valid(&json!(format!("{}\nlegacy", "文".repeat(60)))));
        assert!(schema.is_valid(&json!("a".repeat(4096))));
        assert!(note.get("x-maxUtf8Bytes").is_none());
    }
}

#[test]
fn directory_batch_contract_requires_frozen_explicit_mappings() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let mut api = read(&path).unwrap();
    api["$ref"]="#/paths/~1api~1file-manager~1disks~1%7Bslot%7D~1namespace-batch/post/requestBody/content/application~1json/schema".into();
    let schema = validator(&api).unwrap();
    let source = json!({"directory_branch_id":storage_contract::new_id("dbr"),"expected_tree_id":storage_contract::new_id("dtree"),"path":"a/report.txt","entry_id":storage_contract::new_id("entry")});
    let target = json!({"directory_branch_id":storage_contract::new_id("dbr"),"expected_tree_id":storage_contract::new_id("dtree"),"path":"b/report.txt"});
    let valid = json!({"kind":"move","operation_id":storage_contract::new_id("op"),"items":[{"source":source,"target":target}]});
    assert!(schema.is_valid(&valid));
    for kind in ["copy", "rename"] {
        let mut body = valid.clone();
        body["kind"] = kind.into();
        assert!(schema.is_valid(&body));
    }
    let mut delete = valid.clone();
    delete["kind"] = "delete".into();
    delete["items"][0].as_object_mut().unwrap().remove("target");
    assert!(schema.is_valid(&delete));
    for pointer in ["expected_tree_id", "entry_id"] {
        let mut invalid = valid.clone();
        invalid["items"][0]["source"]
            .as_object_mut()
            .unwrap()
            .remove(pointer);
        assert!(!schema.is_valid(&invalid));
    }
    let mut invalid = valid.clone();
    invalid["items"][0]["source"]["all_branches"] = true.into();
    assert!(!schema.is_valid(&invalid));
    invalid = valid.clone();
    invalid["items"] = json!([]);
    assert!(!schema.is_valid(&invalid));
    invalid = valid.clone();
    invalid["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("target");
    assert!(!schema.is_valid(&invalid));
    invalid = valid.clone();
    invalid["kind"] = "delete".into();
    assert!(!schema.is_valid(&invalid));
    invalid = valid;
    invalid["kind"] = "overwrite".into();
    assert!(!schema.is_valid(&invalid));
}

#[test]
fn file_history_graph_distinguishes_path_nodes_from_real_content_versions() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let api = read(&path).unwrap();
    let mut document = api.clone();
    document["$ref"] = "#/components/schemas/FileHistoryNode".into();
    let schema = validator(&document).unwrap();
    let file = storage_contract::new_id("file");
    let version = storage_contract::new_id("ver");
    let renamed = storage_contract::new_id("fnode");
    let content = json!({"node_id":version,"version_id":version,"file_id":file,"kind":"content","parent_node_ids":[],"path":"a/report.txt","previous_path":null,"note":"","created_at":"2026-10-20T00:00:00Z","sequence":"1","current":false});
    let rename = json!({"node_id":renamed,"version_id":version,"file_id":file,"kind":"rename","parent_node_ids":[version],"path":"a/new.txt","previous_path":"a/report.txt","note":"Path note","created_at":"1792400000","sequence":"2","current":true});
    assert!(schema.is_valid(&content));
    assert!(schema.is_valid(&rename));
    let mut child = content.clone();
    child["node_id"] = storage_contract::new_id("ver").into();
    child["version_id"] = child["node_id"].clone();
    child["parent_node_ids"] = json!([renamed]);
    child["source_version_ids"] = json!([version]);
    assert!(schema.is_valid(&child));
    let mut invalid = rename.clone();
    invalid["version_id"] = renamed.clone().into();
    assert!(!schema.is_valid(&invalid));
    invalid = rename.clone();
    invalid["node_id"] = version.clone().into();
    assert!(!schema.is_valid(&invalid));
    invalid = rename.clone();
    invalid["parent_node_ids"] = json!([]);
    assert!(!schema.is_valid(&invalid));
    invalid = child.clone();
    invalid["source_version_ids"] = json!([renamed]);
    assert!(!schema.is_valid(&invalid));
    invalid = child;
    invalid["parent_node_ids"] = json!([version, version]);
    assert!(!schema.is_valid(&invalid));
    let note = &api["paths"]["/api/file-manager/files/{file}/nodes/{node}"]["patch"]["requestBody"]
        ["content"]["application/json"]["schema"]["properties"]["note"];
    assert_eq!(note["x-maxUtf8Bytes"], 128);
    let note_schema = validator(note).unwrap();
    assert!(note_schema.is_valid(&json!("")));
    assert!(!note_schema.is_valid(&json!("bad\nline")));
    let source = &api["paths"]["/api/file-manager/files/{file}/branches"]["post"]["requestBody"]["content"]
        ["application/json"]["schema"]["properties"]["source_node_id"];
    let mut source_document = api.clone();
    source_document["$ref"] = source["$ref"].clone();
    let source_schema = validator(&source_document).unwrap();
    assert!(source_schema.is_valid(&json!(version)));
    assert!(source_schema.is_valid(&json!(renamed)));
    assert!(!source_schema.is_valid(&json!(file)));
}

#[test]
fn file_merge_tags_and_scope_backups_require_explicit_reviewed_baselines() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let api = read(&path).unwrap();
    let request_schema = |route: &str, method: &str| {
        let mut document = api.clone();
        document["allOf"] = json!([
            api["paths"][route][method]["requestBody"]["content"]["application/json"]["schema"]
        ]);
        validator(&document).unwrap()
    };
    let file = storage_contract::new_id("file");
    let branch = storage_contract::new_id("br");
    let other = storage_contract::new_id("br");
    let version = storage_contract::new_id("ver");
    let meta = storage_contract::new_id("fmeta");
    let operation = storage_contract::new_id("op");
    let merge = request_schema("/api/file-manager/files/{file}/merge", "post");
    let body = json!({"inputs":[{"branch_id":branch,"expected_node_id":version},{"branch_id":other,"expected_node_id":meta}],"chosen_node_id":meta,"surviving_branch_id":branch,"confirmed":true,"operation_id":operation});
    assert!(merge.is_valid(&body));
    for key in ["confirmed", "surviving_branch_id", "chosen_node_id"] {
        let mut invalid = body.clone();
        invalid.as_object_mut().unwrap().remove(key);
        assert!(!merge.is_valid(&invalid));
    }
    let mut invalid = body.clone();
    invalid["confirmed"] = false.into();
    assert!(!merge.is_valid(&invalid));
    invalid = body.clone();
    invalid["inputs"][0]
        .as_object_mut()
        .unwrap()
        .remove("expected_node_id");
    assert!(!merge.is_valid(&invalid));
    invalid = body.clone();
    invalid["inputs"] = json!([body["inputs"][0]]);
    assert!(!merge.is_valid(&invalid));
    invalid = body;
    invalid["repository_id"] = storage_contract::new_id("repo").into();
    assert!(!merge.is_valid(&invalid));
    let tags = request_schema("/api/file-manager/files/{file}/tags", "post");
    let body = json!({"targets":[{"branch_id":branch,"source_node_id":version,"expected_node_id":meta}],"add":["审核"],"remove":[],"operation_id":operation});
    assert!(tags.is_valid(&body));
    let mut invalid = body.clone();
    invalid["targets"][0]
        .as_object_mut()
        .unwrap()
        .remove("expected_node_id");
    assert!(!tags.is_valid(&invalid));
    invalid = body.clone();
    invalid["add"] = json!(["bad\nlabel"]);
    assert!(!tags.is_valid(&invalid));
    invalid = body;
    invalid["add"] = json!((0..17).map(|n| format!("tag{n}")).collect::<Vec<_>>());
    assert!(!tags.is_valid(&invalid));
    let backups = request_schema("/api/repositories/{repo}/backup-configuration", "put");
    let body = json!({"path":"docs/report.txt","target_copies":3,"backup_prefixes":{"B01":"工作备份","C01":"archive"},"expected_scope_id":file,"expected_generation":"0","operation_id":operation});
    assert!(backups.is_valid(&body));
    for key in ["expected_scope_id", "expected_generation"] {
        let mut invalid = body.clone();
        invalid.as_object_mut().unwrap().remove(key);
        assert!(!backups.is_valid(&invalid));
    }
    let mut document = api.clone();
    document["$ref"] = "#/components/schemas/FileHistoryNode".into();
    let node = validator(&document).unwrap();
    let body = json!({"node_id":meta,"version_id":version,"file_id":file,"kind":"tags","parent_node_ids":[version],"path":"report.txt","previous_path":null,"note":"","created_at":"1792400000","sequence":"5","current":true,"tags":["审核"]});
    assert!(node.is_valid(&body));
    let mut invalid = body;
    invalid["version_id"] = meta.into();
    assert!(!node.is_valid(&invalid));
    // Reviewed response fixtures are source inputs, not optional local evidence.
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/tools/contract-check/fixtures/merge-tags-backup");
    for (route, name) in [
        (
            "/api/file-manager/files/{file}/history",
            "file-merge-tag-history.json",
        ),
        (
            "/api/repositories/{repo}/backup-configuration",
            "scoped-backup-configuration.json",
        ),
        (
            "/api/repositories/{repo}/backup-configuration",
            "backup-prefix-configuration.json",
        ),
        (
            "/api/repositories/{repo}/backup-configuration",
            "backup-mixed-routes.json",
        ),
        (
            "/api/file-manager/disks/{slot}/entries",
            "backup-prefix-files.json",
        ),
        (
            "/api/file-manager/disks/{slot}/entries",
            "backup-prefix-retained.json",
        ),
        (
            "/api/file-manager/disks/{slot}/entries",
            "backup-prefix-acl.json",
        ),
        ("/api/file-manager/disks", "backup-route-disks.json"),
    ] {
        let wire = read(&fixtures.join(name))
            .unwrap_or_else(|error| panic!("required response fixture {name}: {error}"));
        let mut schema = api.clone();
        schema["allOf"] = json!([
            api["paths"][route]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
        ]);
        validator(&schema)
            .unwrap()
            .validate(&wire)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn monitoring_contracts_share_snapshots_and_preserve_unavailable_values_and_auth() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/contracts/openapi.json");
    let api = read(&path).unwrap();
    let response_schema = |route: &str| {
        let mut schema = api.clone();
        let route = route.replace('~', "~0").replace('/', "~1");
        schema["$ref"] =
            format!("#/paths/{route}/get/responses/200/content/application~1json/schema").into();
        validator(&schema).unwrap()
    };
    let snapshot_schema = response_schema("/api/system/status");
    let snapshot = json!({
        "collected_at":"2026-10-19T00:00:00Z",
        "cpu":{"usage_percent":12.5,"logical_count":4,"power_watts":15.5,"power_unavailable_reason":null},
        "memory":{"total_bytes":"18446744073709551615","used_bytes":"10","available_bytes":"18446744073709551605"},
        "disks":[{"id":"P01","name":"P01","online":true,"read_only":false,"simulation":true,"total_bytes":"100","available_bytes":"50","used_bytes":"50","usage_percent":50,"power_watts":null,"power_unavailable_reason":"simulated_disk"}]
    });
    assert!(snapshot_schema.is_valid(&snapshot));
    let mut unavailable = snapshot.clone();
    unavailable["cpu"]["usage_percent"] = Value::Null;
    unavailable["cpu"]["power_watts"] = Value::Null;
    unavailable["cpu"]["power_unavailable_reason"] = "sampling".into();
    unavailable["disks"][0]["usage_percent"] = Value::Null;
    assert!(snapshot_schema.is_valid(&unavailable));
    let mut zero = snapshot.clone();
    zero["cpu"]["usage_percent"] = 0.into();
    zero["cpu"]["power_watts"] = 0.into();
    assert!(snapshot_schema.is_valid(&zero));
    for (pointer, value) in [
        ("/cpu/power_watts", json!(-1)),
        ("/cpu/usage_percent", json!(101)),
        (
            "/cpu/power_unavailable_reason",
            json!("/sys/private/sensor"),
        ),
        ("/disks/0/usage_percent", json!(-1)),
        ("/disks/0/power_watts", json!(-1)),
    ] {
        let mut invalid = snapshot.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(!snapshot_schema.is_valid(&invalid));
    }
    let mut leaked_path = snapshot.clone();
    leaked_path["disks"][0]["physical_device"] = "/sys/private/device".into();
    assert!(!snapshot_schema.is_valid(&leaked_path));

    let history = &api["paths"]["/api/system/history"]["get"];
    let status = &api["paths"]["/api/system/status"]["get"];
    assert_eq!(history["operationId"], "getSystemHistory");
    for code in ["401", "403"] {
        assert_eq!(history["responses"][code], status["responses"][code]);
    }
    assert_eq!(
        history.get("security").unwrap_or(&api["security"]),
        status.get("security").unwrap_or(&api["security"])
    );
    assert_eq!(
        history["parameters"][0]["schema"]["enum"],
        json!(["1min", "30min", "1h", "24h", "1month"])
    );
    assert_eq!(history["parameters"][0]["schema"]["default"], "1min");
    let range_schema = validator(&history["parameters"][0]["schema"]).unwrap();
    assert!(!range_schema.is_valid(&json!("1year")));
    let history_schema = response_schema("/api/system/history");
    let response = json!({"range":"1h","interval_seconds":10,"from":"2026-10-18T23:00:00Z","to":"2026-10-19T00:00:00Z","samples":[snapshot,unavailable]});
    assert!(history_schema.is_valid(&response));
    let mut empty = response.clone();
    empty["samples"] = json!([]);
    assert!(history_schema.is_valid(&empty));
    let mut too_many = response.clone();
    too_many["samples"] = json!(vec![response["samples"][0].clone(); 362]);
    assert!(!history_schema.is_valid(&too_many));
}
