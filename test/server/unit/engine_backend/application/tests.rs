use super::*;
use crate::{Backend, engine_backend::EngineBackend};
use serde_json::json;
use std::collections::BTreeSet;
use storage_contract::{Config, new_id};

fn fixture() -> (
    tempfile::TempDir,
    Engine,
    storage_engine::Repository,
    Identity,
) {
    let root = tempfile::tempdir().unwrap();
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
    let repo = engine
        .create_repository("Application", "P01", 3, "owner", &new_id("op"))
        .unwrap();
    let identity = Identity {
        principal: "owner".into(),
        admin: false,
        repositories: BTreeSet::from([repo.repository_id.clone()]),
    };
    (root, engine, repo, identity)
}

#[test]
fn protection_commands_validate_wire_types_and_unknown_fields() {
    let policy = json!({"policy_id":new_id("policy"),"repository_id":new_id("repo"),
            "path_prefix":"","target_copies":3,"expected_generation":"0","operation_id":new_id("op")});
    let rule = json!({"rule_id":new_id("rule"),"repository_id":new_id("repo"),
            "path_prefix":"","primary_slot_id":"P01","expected_generation":"0","operation_id":new_id("op")});
    for (is_policy, valid) in [(true, policy), (false, rule)] {
        let decode = |value: serde_json::Value| {
            if is_policy {
                serde_json::from_value::<UpdateProtectionPolicy>(value).map(|_| ())
            } else {
                serde_json::from_value::<UpdatePlacementRule>(value).map(|_| ())
            }
        };
        assert!(decode(valid.clone()).is_ok());
        for generation in [
            json!(0),
            json!(null),
            json!(""),
            json!("00"),
            json!("01"),
            json!("+1"),
            json!(" 1"),
            json!("-1"),
            json!("18446744073709551616"),
        ] {
            let mut invalid = valid.clone();
            invalid["expected_generation"] = generation;
            assert!(decode(invalid).is_err());
        }
        let mut max = valid.clone();
        max["expected_generation"] = json!(u64::MAX.to_string());
        assert!(decode(max).is_ok());
        for key in ["repository_id", "operation_id"] {
            let mut invalid = valid.clone();
            invalid[key] = json!("");
            assert!(decode(invalid).is_err());
        }
        let mut extra = valid.clone();
        extra["principal"] = json!("admin");
        assert!(decode(extra).is_err());
        let mut invalid = valid.clone();
        invalid["path_prefix"] = json!(null);
        assert!(decode(invalid).is_err());
        if is_policy {
            for copies in [json!("3"), json!(3.5), json!(-1), json!(4294967296_u64)] {
                let mut invalid = valid.clone();
                invalid["target_copies"] = copies;
                assert!(decode(invalid).is_err());
            }
        }
    }
}

#[test]
fn protection_commands_preserve_core_replay_and_transport_contract() {
    for is_policy in [true, false] {
        let (_root, mut engine, repo, owner) = fixture();
        let operation = new_id("op");
        let id = new_id(if is_policy { "policy" } else { "rule" });
        let (action, input, original) = if is_policy {
            let original = engine
                .update_protection_policy(storage_engine::ProtectionPolicyUpdate {
                    policy_id: &id,
                    repository_id: &repo.repository_id,
                    path_prefix: "",
                    target_copies: 3,
                    expected_generation: 0,
                    principal: &owner.principal,
                    operation_id: &operation,
                })
                .unwrap();
            (
                crate::BackendAction::UpdatePolicy,
                json!({"policy_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                        "target_copies":3,"expected_generation":"0","operation_id":operation}),
                original,
            )
        } else {
            let original = engine
                .update_placement_rule(storage_engine::PlacementRuleUpdate {
                    rule_id: &id,
                    repository_id: &repo.repository_id,
                    path_prefix: "",
                    primary_slot_id: "P01",
                    expected_generation: 0,
                    principal: &owner.principal,
                    operation_id: &operation,
                })
                .unwrap();
            (
                crate::BackendAction::UpdatePlacementRule,
                json!({"rule_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                        "primary_slot_id":"P01","expected_generation":"0","operation_id":operation}),
                original,
            )
        };
        let backend = EngineBackend::new(engine);
        let replay = backend.call(action, &owner, input.clone()).unwrap();
        assert_eq!(replay["request_digest"], original.request_digest);
        assert_eq!(replay["result"], original.result);
        assert_eq!(
            replay["publication_sequence"],
            original.sequence.to_string()
        );
        assert_eq!(replay["operation_kind"], original.kind);
        let mut changed = input.clone();
        changed["path_prefix"] = json!("Other");
        assert_eq!(
            backend.call(action, &owner, changed).unwrap_err().code,
            "IDEMPOTENCY_KEY_REUSED"
        );
        let mut extra = input.clone();
        extra["unexpected"] = json!(true);
        assert_eq!(backend.call(action, &owner, extra).unwrap_err().status, 400);
        let stranger = Identity {
            principal: "stranger".into(),
            admin: false,
            repositories: owner.repositories.clone(),
        };
        assert_eq!(
            backend
                .call(action, &stranger, input.clone())
                .unwrap_err()
                .status,
            403
        );
        let mut malformed = input.clone();
        malformed["expected_generation"] = json!(1);
        assert_eq!(
            backend
                .call(action, &stranger, malformed.clone())
                .unwrap_err()
                .status,
            403
        );
        assert_eq!(
            backend.call(action, &owner, malformed).unwrap_err().status,
            400
        );
        // Transport-independent callers also need both account admission and Manage.
        let mut engine = backend.engine.lock().unwrap();
        let denied = if is_policy {
            Application::new(&mut engine, &stranger)
                .update_protection_policy(serde_json::from_value(input).unwrap())
        } else {
            Application::new(&mut engine, &stranger)
                .update_placement_rule(serde_json::from_value(input).unwrap())
        };
        assert_eq!(denied.unwrap_err().status, 403);
        assert_eq!(
            engine.state().operations[&operation].request_digest,
            original.request_digest
        );
        drop(engine);
        let mut update = if is_policy {
            json!({"policy_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                    "target_copies":3,"expected_generation":"1","operation_id":new_id("op")})
        } else {
            json!({"rule_id":id,"repository_id":repo.repository_id,"path_prefix":"",
                    "primary_slot_id":"P01","expected_generation":"1","operation_id":new_id("op")})
        };
        let unadmitted_owner = Identity {
            principal: owner.principal.clone(),
            admin: false,
            repositories: BTreeSet::new(),
        };
        assert_eq!(
            backend
                .call(action, &unadmitted_owner, update.clone())
                .unwrap_err()
                .status,
            403
        );
        let applied = backend.call(action, &owner, update.clone()).unwrap();
        let record = if is_policy { "policy" } else { "rule" };
        assert_eq!(applied["result"][record]["generation"], "2");
        update["operation_id"] = json!(new_id("op"));
        let stale = backend.call(action, &owner, update).unwrap_err();
        assert_eq!(stale.code, "CONFIG_CONFLICT");
        assert_eq!(stale.status, 409);
    }
}

#[test]
fn public_commit_roundtrip_preserves_canonical_envelope_and_executes() {
    let (_root, mut engine, repo, identity) = fixture();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, &identity.principal)
        .unwrap();
    Application::new(&mut engine, &identity)
        .stage_namespace(StageNamespace {
            workspace_id: workspace.workspace_id.clone(),
            change: NamespaceOperation {
                kind: "mkdir".into(),
                path: "Folder".into(),
                destination: None,
            },
        })
        .unwrap();
    let mut body = engine
        .seal_workspace(&workspace.workspace_id, &identity.principal)
        .unwrap();
    body["request_digest"] = storage_engine::commit_request_digest(&workspace.workspace_id, &body)
        .unwrap()
        .into();
    let command: PublicCommitWorkspace = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(serde_json::to_value(&command).unwrap(), body);
    let result = Application::new(&mut engine, &identity)
        .commit_workspace_public(command.clone())
        .unwrap();
    let replay = Application::new(&mut engine, &identity)
        .commit_workspace_public(command)
        .unwrap();
    assert_eq!(replay.commit_id, result.commit_id);
    assert!(
        engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .iter()
            .any(|entry| entry.path == "Folder")
    );
}

#[test]
fn workspace_ownership_and_account_admission_apply_to_typed_and_wire_paths() {
    let (_root, mut engine, repo, owner) = fixture();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, &owner.principal)
        .unwrap();
    let intruder = Identity {
        principal: "intruder".into(),
        ..owner.clone()
    };
    let result = Application::new(&mut engine, &intruder).stage_namespace(StageNamespace {
        workspace_id: workspace.workspace_id.clone(),
        change: NamespaceOperation {
            kind: "mkdir".into(),
            path: "TypedDenied".into(),
            destination: None,
        },
    });
    assert_eq!(result.unwrap_err().code, "FORBIDDEN");
    let unadmitted_owner = Identity {
        repositories: BTreeSet::new(),
        ..owner.clone()
    };
    let result =
        Application::new(&mut engine, &unadmitted_owner).commit_workspace(CommitWorkspace {
            workspace_id: workspace.workspace_id.clone(),
            operation_id: new_id("op"),
            expected_head: workspace.base_head.clone(),
            fencing_token: workspace.fencing_token,
            message: String::new(),
            request_digest: None,
        });
    assert_eq!(result.unwrap_err().code, "FORBIDDEN");
    let backend = EngineBackend::new(engine);
    for identity in [&intruder, &unadmitted_owner] {
        let result = backend.call(
            crate::BackendAction::Namespace,
            identity,
            json!({"workspace_id":workspace.workspace_id,
                "kind":"mkdir", "path":"WireDenied"}),
        );
        assert_eq!(result.unwrap_err().code, "FORBIDDEN");
    }
    let engine = backend.engine.lock().unwrap();
    assert!(
        engine
            .workspace(&workspace.workspace_id)
            .unwrap()
            .namespace_operations
            .is_empty()
    );
}

#[test]
fn public_commit_decode_rejects_unknown_missing_and_numeric_decimal_fields() {
    let body = json!({"workspace_id":"ws", "operation_id":"op", "repository_id":"repo", "branch_id":"branch",
            "expected_head":"head", "fencing_token":"1", "message":"", "requested_protection":"strict",
            "file_placement":{}, "expected_topology_generation":"1", "sealed_manifest_hash":"hash", "request_digest":"digest"});
    assert!(serde_json::from_value::<PublicCommitWorkspace>(body.clone()).is_ok());
    for field in [
        "workspace_id",
        "operation_id",
        "expected_head",
        "message",
        "request_digest",
    ] {
        let mut missing = body.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<PublicCommitWorkspace>(missing).is_err(),
            "{field}"
        );
    }
    for field in ["fencing_token", "expected_topology_generation"] {
        let mut numeric = body.clone();
        numeric[field] = json!(1);
        assert!(
            serde_json::from_value::<PublicCommitWorkspace>(numeric).is_err(),
            "{field}"
        );
    }
    let mut extra = body;
    extra["principal"] = "someone-else".into();
    assert!(serde_json::from_value::<PublicCommitWorkspace>(extra).is_err());
}

#[test]
fn typed_branch_requires_manage_and_rejects_cross_repository_source() {
    let (_root, mut engine, repo, owner) = fixture();
    let other = engine
        .create_repository("Other", "P01", 3, &owner.principal, &new_id("op"))
        .unwrap();
    let source = engine.state().branches[&other.main_branch_id]
        .head_commit_id
        .clone();
    let result = Application::new(&mut engine, &owner).create_branch(CreateBranch {
        repository_id: repo.repository_id,
        source_commit_id: source,
        name: "Cross scope".into(),
        operation_id: new_id("op"),
    });
    assert_eq!(result.unwrap_err().code, "HISTORY_NODE_NOT_FOUND");
}
#[test]
fn upload_finalization_rechecks_completed_account_revocation() {
    for disabled in [false, true] {
        let (root, mut engine, repo, owner) = fixture();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, &owner.principal)
            .unwrap();
        let source = root.path().join("upload-source");
        std::fs::write(&source, b"prepared bytes").unwrap();
        let item = new_id("entry");
        let backend = std::sync::Arc::new(EngineBackend::new(engine));
        let mut auth =
            crate::auth::AuthStore::initialize(root.path(), "admin", "integration-test-password")
                .unwrap();
        auth.upsert_account(
            "owner",
            Some("integration-test-password"),
            false,
            owner.repositories.clone(),
            false,
        )
        .unwrap();
        let mut auth = backend.protect_auth(auth).unwrap();
        let plan = {
            let engine = backend.engine.lock().unwrap();
            engine
                .prepare_upload(&workspace.workspace_id, &item, "file.txt", &source, 1)
                .unwrap()
        };
        // This update completes while byte preparation holds no engine lock.
        auth.upsert_account(
            "owner",
            None,
            false,
            if disabled {
                owner.repositories.clone()
            } else {
                BTreeSet::new()
            },
            disabled,
        )
        .unwrap();
        let prepared = plan.execute().unwrap();
        let mut engine = backend.engine.lock().unwrap();
        let result = crate::engine_backend::finish_authorized_upload(
            &mut engine,
            &owner,
            &workspace.workspace_id,
            prepared,
        );
        assert_eq!(
            result.unwrap_err().code,
            if disabled {
                "UNAUTHORIZED"
            } else {
                "FORBIDDEN"
            }
        );
        assert!(
            engine
                .workspace(&workspace.workspace_id)
                .unwrap()
                .files
                .is_empty()
        );
    }
}
#[test]
fn commit_preparation_releases_engine_and_rechecks_completed_revocation() {
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;
    for public in [false, true] {
        for revoke in [None, Some(false), Some(true)] {
            let (root, mut engine, repo, owner) = fixture();
            let workspace = engine
                .create_workspace(&repo.main_branch_id, &owner.principal)
                .unwrap();
            engine
                .stage_bytes(&workspace.workspace_id, "file.txt", b"commit bytes")
                .unwrap();
            let operation = new_id("op");
            let input = if public {
                let mut input = engine
                    .seal_workspace(&workspace.workspace_id, &owner.principal)
                    .unwrap();
                input["operation_id"] = operation.clone().into();
                input["request_digest"] =
                    storage_engine::commit_request_digest(&workspace.workspace_id, &input)
                        .unwrap()
                        .into();
                input
            } else {
                json!({"workspace_id":workspace.workspace_id,"operation_id":operation,
                        "expected_head":workspace.base_head,"fencing_token":workspace.fencing_token.to_string(),"message":"commit"})
            };
            let backend = Arc::new(EngineBackend::new(engine));
            let mut auth = crate::auth::AuthStore::initialize(
                root.path(),
                "admin",
                "integration-test-password",
            )
            .unwrap();
            auth.upsert_account(
                "owner",
                Some("integration-test-password"),
                false,
                owner.repositories.clone(),
                false,
            )
            .unwrap();
            let mut auth = backend.protect_auth(auth).unwrap();
            // A request can retain an Identity captured before revocation.
            // Reject it before preparing bytes, even though concurrent auth
            // writes would also be fenced once preparation has started.
            if let Some(disabled) = revoke {
                auth.upsert_account(
                    "owner",
                    None,
                    false,
                    if disabled {
                        owner.repositories.clone()
                    } else {
                        BTreeSet::new()
                    },
                    disabled,
                )
                .unwrap();
                let result = backend.call(
                    if public {
                        crate::BackendAction::CommitWorkspace
                    } else {
                        crate::BackendAction::CommitWorkspaceInternal
                    },
                    &owner,
                    input,
                );
                assert_eq!(
                    result.unwrap_err().code,
                    if disabled {
                        "UNAUTHORIZED"
                    } else {
                        "FORBIDDEN"
                    }
                );
                let engine = backend.engine.lock().unwrap();
                assert!(!engine.state().operations.contains_key(&operation));
                assert!(engine.list_tree(&repo.main_branch_id).unwrap().is_empty());
                continue;
            }
            let (started_tx, started_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let release_rx = Mutex::new(release_rx);
            *backend.commit_preparation_hook.lock().unwrap() = Some(Arc::new(move || {
                started_tx.send(()).unwrap();
                release_rx.lock().unwrap().recv().unwrap();
            }));
            let writer_backend = backend.clone();
            let writer_owner = owner.clone();
            let writer = std::thread::spawn(move || {
                writer_backend.call(
                    if public {
                        crate::BackendAction::CommitWorkspace
                    } else {
                        crate::BackendAction::CommitWorkspaceInternal
                    },
                    &writer_owner,
                    input,
                )
            });
            started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let reader_backend = backend.clone();
            let reader_owner = owner.clone();
            let (query_tx, query_rx) = mpsc::channel();
            let reader = std::thread::spawn(move || {
                query_tx
                    .send(reader_backend.call(
                        crate::BackendAction::Repositories,
                        &reader_owner,
                        json!({}),
                    ))
                    .unwrap();
            });
            let queried = query_rx.recv_timeout(Duration::from_secs(2));
            if queried.is_err() {
                release_tx.send(()).unwrap();
                let _ = writer.join();
                let _ = reader.join();
                panic!("repository query blocked behind detached commit preparation");
            }
            assert!(queried.unwrap().is_ok());
            reader.join().unwrap();
            let before_busy = auth.snapshot().unwrap();
            let busy = auth
                .upsert_account("owner", None, false, owner.repositories.clone(), false)
                .unwrap_err();
            assert_eq!(busy.code, "COMMIT_PREPARATION_BUSY");
            assert!(busy.retryable);
            assert_eq!(auth.snapshot().unwrap(), before_busy);
            assert!(auth.identity_for_principal("admin").is_ok());
            release_tx.send(()).unwrap();
            let result = writer.join().unwrap();
            let engine = backend.engine.lock().unwrap();
            assert!(result.is_ok(), "{result:?}");
            assert!(engine.state().operations.contains_key(&operation));
        }
    }
}
