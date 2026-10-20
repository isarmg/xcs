//! Operation ownership and current repository access must precede health I/O.
use super::{authorize, repo_for_history};
use crate::{ApiError, auth::Identity};
use storage_engine::{Engine, OperationRecord};

pub(super) fn authorize_record(
    engine: &Engine,
    identity: &Identity,
    record: &OperationRecord,
) -> Result<(), ApiError> {
    if !identity.admin && record.owner() != Some(identity.principal.as_str()) {
        return Err(ApiError::forbidden());
    }
    if let Some(history) = &record.result().history_tree_id {
        authorize(identity, repo_for_history(engine, history)?, false, false)
    } else {
        identity.require_admin()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Backend, BackendAction, auth::Identity, engine_backend::EngineBackend};
    use serde_json::{Value, json};
    use std::{
        collections::BTreeSet,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    use storage_contract::{Config, new_id};
    use storage_engine::{CommitRequest, Engine, Grants};

    fn fixture() -> (
        tempfile::TempDir,
        Arc<EngineBackend>,
        storage_engine::Repository,
        Identity,
        Value,
    ) {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let repo = engine
            .create_repository("Operation", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "owner")
            .unwrap();
        engine
            .stage_bytes(
                &workspace.workspace_id,
                "data.txt",
                b"fully verified content",
            )
            .unwrap();
        let operation = new_id("op");
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: operation.clone(),
                principal: "owner".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: String::new(),
                request_digest: None,
            })
            .unwrap();
        let identity = Identity {
            principal: "owner".into(),
            admin: false,
            repositories: BTreeSet::from([repo.repository_id.clone()]),
        };
        (
            root,
            Arc::new(EngineBackend::new(engine)),
            repo,
            identity,
            json!({"operation_id": operation}),
        )
    }

    #[test]
    fn unauthorized_operations_never_start_payload_observation() {
        let (_root, backend, repo, identity, input) = fixture();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = calls.clone();
        *backend.operation_health_hook.lock().unwrap() = Some(Arc::new(move || {
            probe.fetch_add(1, Ordering::SeqCst);
        }));
        let foreign = Identity {
            principal: "other".into(),
            ..identity.clone()
        };
        let denied_account = Identity {
            repositories: BTreeSet::new(),
            ..identity.clone()
        };
        for denied in [&foreign, &denied_account] {
            assert_eq!(
                backend
                    .call(BackendAction::Operation, denied, input.clone())
                    .unwrap_err()
                    .code,
                "FORBIDDEN"
            );
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            backend
                .call(
                    BackendAction::Operation,
                    &identity,
                    json!({"operation_id": new_id("op")})
                )
                .unwrap_err()
                .code,
            "OPERATION_NOT_FOUND"
        );
        backend
            .engine
            .lock()
            .unwrap()
            .set_grants(
                &repo.repository_id,
                Grants {
                    manage: BTreeSet::from(["replacement".into()]),
                    ..Default::default()
                },
                "owner",
                &new_id("op"),
            )
            .unwrap();
        assert_eq!(
            backend
                .call(BackendAction::Operation, &identity, input)
                .unwrap_err()
                .code,
            "FORBIDDEN"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn slow_operation_health_releases_engine_and_rechecks_revoked_acl() {
        use std::sync::mpsc;
        use std::time::Duration;
        let (_root, backend, repo, identity, input) = fixture();
        let (started, receive_started) = mpsc::sync_channel(0);
        let (release, receive_release) = mpsc::sync_channel(0);
        let receive_release = std::sync::Mutex::new(receive_release);
        *backend.operation_health_hook.lock().unwrap() = Some(Arc::new(move || {
            started.send(()).unwrap();
            receive_release.lock().unwrap().recv().unwrap();
        }));
        std::thread::scope(|scope| {
            let request = scope.spawn(|| backend.call(BackendAction::Operation, &identity, input));
            receive_started
                .recv_timeout(Duration::from_secs(10))
                .unwrap();
            // Deterministic barrier, not a timing-based large-file benchmark.
            // A different thread can acquire Engine while the read is paused.
            let available = backend.engine.try_lock();
            if let Ok(mut engine) = available {
                engine
                    .set_grants(
                        &repo.repository_id,
                        Grants {
                            manage: BTreeSet::from(["replacement".into()]),
                            ..Default::default()
                        },
                        "owner",
                        &new_id("op"),
                    )
                    .unwrap();
            } else {
                release.send(()).unwrap();
                panic!("health observation retained the Engine mutex");
            }
            release.send(()).unwrap();
            assert_eq!(request.join().unwrap().unwrap_err().code, "FORBIDDEN");
        });
    }

    #[test]
    fn health_observation_rechecks_protected_account_after_unlocked_io() {
        use crate::auth::AuthStore;
        use std::{sync::mpsc, time::Duration};
        for disabled in [true, false] {
            let (root, backend, repo, identity, input) = fixture();
            let mut auth =
                AuthStore::initialize(root.path(), "admin", "integration-test-password").unwrap();
            auth.upsert_account(
                "owner",
                Some("integration-test-password"),
                false,
                BTreeSet::from([repo.repository_id.clone()]),
                false,
            )
            .unwrap();
            let mut auth = backend.protect_auth(auth).unwrap();
            let (started, receive_started) = mpsc::sync_channel(0);
            let (release, receive_release) = mpsc::sync_channel(0);
            let receive_release = std::sync::Mutex::new(receive_release);
            *backend.operation_health_hook.lock().unwrap() = Some(Arc::new(move || {
                started.send(()).unwrap();
                receive_release.lock().unwrap().recv().unwrap();
            }));
            std::thread::scope(|scope| {
                let request =
                    scope.spawn(|| backend.call(BackendAction::Operation, &identity, input));
                receive_started
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
                // Auth publishes its protected snapshot through the free Engine
                // mutex. The read path must never reacquire an AuthStore lock.
                let revoked = auth.upsert_account("owner", None, false, BTreeSet::new(), disabled);
                release.send(()).unwrap();
                revoked.unwrap();
                assert_eq!(
                    request.join().unwrap().unwrap_err().code,
                    if disabled {
                        "UNAUTHORIZED"
                    } else {
                        "FORBIDDEN"
                    }
                );
            });
        }
    }

    #[test]
    fn operation_observation_preserves_publication_wire_facts() {
        let (_root, backend, _, identity, input) = fixture();
        let before = backend
            .call(BackendAction::Operation, &identity, input.clone())
            .unwrap();
        assert_eq!(before["current_health"]["state"], "HEALTHY");
        {
            let engine = backend.engine.lock().unwrap();
            for disk in &engine.config().disks {
                // Remove replicas after publication without changing the frozen
                // receipt. A new observation must never reuse prior health.
                std::fs::remove_dir_all(disk.root.join("histories")).unwrap();
            }
        }
        let after = backend
            .call(BackendAction::Operation, &identity, input)
            .unwrap();
        assert_eq!(after["current_health"]["state"], "UNAVAILABLE");
        assert_eq!(after["commit_state"], before["commit_state"]);
        assert_eq!(after["result"], before["result"]);
        assert_eq!(after["request_digest"], before["request_digest"]);
    }
}
