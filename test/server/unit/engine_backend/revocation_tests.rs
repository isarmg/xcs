//! Deterministic account-publication races at the auth -> engine boundary.
use super::*;
use crate::{AuthorizedBackend, BackendAction, auth::AuthStore};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, Barrier},
};
use storage_engine::{CommitRequest, Config, Workspace, new_id};

/// Pause only after AuthorizedBackend has released its account-store lock.
/// Publishing a revocation here must finish before EngineBackend captures a
/// read plan or admits a mutation; neither side may reverse the lock order.
struct PausedBackend {
    inner: Arc<EngineBackend>,
    captured: Arc<Barrier>,
    released: Arc<Barrier>,
}
impl PausedBackend {
    fn enter(&self) {
        self.captured.wait();
        self.released.wait();
    }
}
impl Backend for PausedBackend {
    fn call(
        &self,
        action: BackendAction,
        identity: &Identity,
        input: Value,
    ) -> Result<Value, ApiError> {
        self.enter();
        self.inner.call(action, identity, input)
    }
    fn upload(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, ApiError> {
        self.enter();
        self.inner.upload(identity, workspace, item, path, source)
    }
    fn upload_generation(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
        path: &str,
        source: &Path,
        generation: u64,
    ) -> Result<Value, ApiError> {
        self.enter();
        self.inner
            .upload_generation(identity, workspace, item, path, source, generation)
    }
    fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.enter();
        self.inner.content(identity, version)
    }
    fn range_content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.enter();
        self.inner.range_content(identity, version)
    }
    fn workspace_content(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
    ) -> Result<Content, ApiError> {
        self.enter();
        self.inner.workspace_content(identity, workspace, item)
    }
    fn checksums(&self, identity: &Identity, version: &str) -> Result<Vec<u8>, ApiError> {
        self.enter();
        self.inner.checksums(identity, version)
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    backend: Arc<EngineBackend>,
    auth: Arc<Mutex<AuthStore>>,
    identity: Identity,
    repo: Repository,
    version: String,
    workspace: Workspace,
    item: String,
    source: PathBuf,
}
impl Fixture {
    fn new(admin: bool, protected: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("primary"),
            root.path().join("secondary"),
            root.path().join("tertiary"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let repo = engine
            .create_repository("Authorization", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "owner")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "file.txt", b"published")
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "owner".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "authorization fixture".into(),
                request_digest: None,
            })
            .unwrap();
        let version = engine.list_tree(&repo.main_branch_id).unwrap()[0]
            .version_id
            .clone();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "owner")
            .unwrap();
        let staged = engine
            .stage_bytes(&workspace.workspace_id, "private.txt", b"private")
            .unwrap();
        let source = root.path().join("source");
        std::fs::write(&source, b"new upload").unwrap();
        let backend = Arc::new(EngineBackend::new(engine));
        let mut auth =
            AuthStore::initialize(root.path(), "admin", "integration-test-password").unwrap();
        auth.upsert_account(
            "owner",
            Some("integration-test-password"),
            admin,
            if admin {
                BTreeSet::new()
            } else {
                BTreeSet::from([repo.repository_id.clone()])
            },
            false,
        )
        .unwrap();
        let identity = auth.identity_for_principal("owner").unwrap();
        let auth = if protected {
            backend.protect_auth(auth).unwrap()
        } else {
            auth
        };
        Self {
            _root: root,
            backend,
            auth: Arc::new(Mutex::new(auth)),
            identity,
            repo,
            version,
            workspace,
            item: staged.item_id,
            source,
        }
    }
    fn snapshot(&self) -> (Value, Value, Vec<Vec<u8>>) {
        let engine = self.backend.engine.lock().unwrap();
        (
            json_codec::to_value(engine.state()).unwrap(),
            json_codec::to_value(engine.workspace(&self.workspace.workspace_id).unwrap()).unwrap(),
            engine
                .config()
                .disks
                .iter()
                .filter(|disk| disk.metadata_member)
                .map(|disk| {
                    std::fs::read(disk.root.join("metadata/application-log/journal.bin")).unwrap()
                })
                .collect(),
        )
    }
}

#[derive(Clone, Copy, Debug)]
enum Request {
    Metadata,
    Administrator,
    Tree,
    Version,
    Namespace,
    Content,
    RangeContent,
    WorkspaceContent,
    Checksums,
    Upload,
    UploadGeneration,
}
impl Request {
    fn execute(self, fixture: &Fixture, backend: &dyn Backend) -> Result<(), ApiError> {
        let identity = &fixture.identity;
        let workspace = &fixture.workspace.workspace_id;
        match self {
            Self::Metadata => backend
                .call(
                    BackendAction::SetGrants,
                    identity,
                    json!({
                        "repository_id":fixture.repo.repository_id,"grants":fixture.repo.grants,
                        "operation_id":new_id("op")
                    }),
                )
                .map(|_| ()),
            Self::Administrator => backend
                .call(
                    BackendAction::CreateRepository,
                    identity,
                    json!({
                        "name":"Forbidden new repository","primary_slot_id":"P01","target_copies":3,
                        "operation_id":new_id("op")
                    }),
                )
                .map(|_| ()),
            Self::Tree => backend
                .call(
                    BackendAction::Tree,
                    identity,
                    json!({
                        "branch_id":fixture.repo.main_branch_id
                    }),
                )
                .map(|_| ()),
            // SMB's AuthorizeRead uses Version for file handles and Tree for
            // directory/write handles; raw content opens share this boundary.
            Self::Version => backend
                .call(
                    BackendAction::Version,
                    identity,
                    json!({
                        "version_id":fixture.version
                    }),
                )
                .map(|_| ()),
            Self::Namespace => backend
                .call(
                    BackendAction::Namespace,
                    identity,
                    json!({
                        "workspace_id":workspace,"kind":"mkdir","path":"forbidden"
                    }),
                )
                .map(|_| ()),
            Self::Content => backend.content(identity, &fixture.version).map(|_| ()),
            Self::RangeContent => backend
                .range_content(identity, &fixture.version)
                .map(|_| ()),
            Self::WorkspaceContent => backend
                .workspace_content(identity, workspace, &fixture.item)
                .map(|_| ()),
            Self::Checksums => backend.checksums(identity, &fixture.version).map(|_| ()),
            Self::Upload => backend
                .upload(identity, workspace, "new-item", "new.txt", &fixture.source)
                .map(|_| ()),
            Self::UploadGeneration => backend
                .upload_generation(
                    identity,
                    workspace,
                    "next-item",
                    "next.txt",
                    &fixture.source,
                    1,
                )
                .map(|_| ()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Revocation {
    Disable,
    Demote,
    RemoveScope,
}

#[test]
fn completed_account_revocation_precedes_all_engine_admissions() {
    for revocation in [
        Revocation::Disable,
        Revocation::Demote,
        Revocation::RemoveScope,
    ] {
        let fixture = Arc::new(Fixture::new(matches!(revocation, Revocation::Demote), true));
        let mut requests = vec![
            Request::Metadata,
            Request::Tree,
            Request::Version,
            Request::Namespace,
            Request::Content,
            Request::RangeContent,
            Request::WorkspaceContent,
            Request::Checksums,
            Request::Upload,
            Request::UploadGeneration,
        ];
        if matches!(revocation, Revocation::Demote) {
            requests.push(Request::Administrator);
        }
        let captured = Arc::new(Barrier::new(requests.len() + 1));
        let released = Arc::new(Barrier::new(requests.len() + 1));
        let backend = Arc::new(AuthorizedBackend {
            backend: Arc::new(PausedBackend {
                inner: fixture.backend.clone(),
                captured: captured.clone(),
                released: released.clone(),
            }),
            auth: fixture.auth.clone(),
        });
        let threads = requests
            .into_iter()
            .map(|request| {
                let fixture = fixture.clone();
                let backend = backend.clone();
                (
                    request,
                    std::thread::spawn(move || request.execute(&fixture, backend.as_ref())),
                )
            })
            .collect::<Vec<_>>();
        captured.wait();
        let revoked = fixture.auth.lock().unwrap().upsert_account(
            "owner",
            None,
            false,
            BTreeSet::new(),
            matches!(revocation, Revocation::Disable),
        );
        let before = fixture.snapshot();
        released.wait();
        revoked.unwrap();
        for (request, thread) in threads {
            let error = thread
                .join()
                .unwrap()
                .expect_err("revoked request must be rejected");
            assert_eq!(
                error.code,
                if matches!(revocation, Revocation::Disable) {
                    "UNAUTHORIZED"
                } else {
                    "FORBIDDEN"
                },
                "{revocation:?}: {request:?}"
            );
        }
        assert!(
            fixture.snapshot() == before,
            "rejected requests changed state, workspace or journals"
        );
    }
}

#[test]
fn current_account_scope_controls_queries_without_disabling_public_probes() {
    let fixture = Fixture::new(true, true);
    fixture
        .auth
        .lock()
        .unwrap()
        .upsert_account(
            "owner",
            None,
            false,
            BTreeSet::from([fixture.repo.repository_id.clone()]),
            false,
        )
        .unwrap();
    // Demotion does not remove the account's independently granted repository access.
    let listed = fixture
        .backend
        .call(BackendAction::Repositories, &fixture.identity, json!({}))
        .unwrap();
    assert_eq!(listed["repositories"].as_array().unwrap().len(), 1);
    assert!(
        Request::Metadata
            .execute(&fixture, fixture.backend.as_ref())
            .is_ok()
    );
    assert!(
        Request::Content
            .execute(&fixture, fixture.backend.as_ref())
            .is_ok()
    );
    assert_eq!(
        Request::Administrator
            .execute(&fixture, fixture.backend.as_ref())
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
    fixture
        .auth
        .lock()
        .unwrap()
        .upsert_account("owner", None, false, BTreeSet::new(), false)
        .unwrap();
    let listed = fixture
        .backend
        .call(BackendAction::Repositories, &fixture.identity, json!({}))
        .unwrap();
    assert!(listed["repositories"].as_array().unwrap().is_empty());
    let events = fixture
        .backend
        .call(BackendAction::Events, &fixture.identity, json!({}))
        .unwrap();
    assert!(events["events"].as_array().unwrap().is_empty());
    fixture
        .auth
        .lock()
        .unwrap()
        .upsert_account("owner", None, false, BTreeSet::new(), true)
        .unwrap();
    assert_eq!(
        fixture
            .backend
            .call(BackendAction::Repositories, &fixture.identity, json!({}))
            .unwrap_err()
            .code,
        "UNAUTHORIZED"
    );
    let anonymous = Identity {
        principal: String::new(),
        admin: false,
        repositories: BTreeSet::new(),
    };
    for action in [BackendAction::Capabilities, BackendAction::Ready] {
        assert!(fixture.backend.call(action, &anonymous, json!({})).is_ok());
    }
}

#[test]
fn embedded_backend_without_protected_accounts_uses_explicit_caller_identity() {
    let fixture = Fixture::new(false, false);
    assert!(
        fixture
            .backend
            .engine
            .lock()
            .unwrap()
            .auth_state()
            .is_none()
    );
    // An unrelated, unattached auth store is not this embedded engine's authority.
    fixture
        .auth
        .lock()
        .unwrap()
        .upsert_account("owner", None, false, BTreeSet::new(), true)
        .unwrap();
    assert!(
        Request::Metadata
            .execute(&fixture, fixture.backend.as_ref())
            .is_ok()
    );
    assert!(
        Request::Content
            .execute(&fixture, fixture.backend.as_ref())
            .is_ok()
    );
    let unadmitted = Identity {
        repositories: BTreeSet::new(),
        ..fixture.identity.clone()
    };
    assert_eq!(
        fixture
            .backend
            .call(
                BackendAction::Repository,
                &unadmitted,
                json!({
                    "repository_id":fixture.repo.repository_id
                })
            )
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
}

#[test]
fn private_disk_sampling_cannot_grant_public_monitoring_access() {
    let fixture = Fixture::new(true, true);
    let before = fixture.snapshot();
    let observed = fixture
        .backend
        .observe_disks(&crate::InternalDiskObservation(()))
        .unwrap();
    assert_eq!(observed["disks"].as_array().unwrap().len(), 3);
    assert!(fixture.snapshot() == before);

    let synthetic = Identity {
        principal: "zxs-system-telemetry".into(),
        admin: true,
        repositories: BTreeSet::new(),
    };
    assert_eq!(
        fixture
            .backend
            .call(BackendAction::MonitorDisks, &synthetic, json!({}))
            .unwrap_err()
            .code,
        "UNAUTHORIZED"
    );
    for disabled in [false, true] {
        fixture
            .auth
            .lock()
            .unwrap()
            .upsert_account("owner", None, false, BTreeSet::new(), disabled)
            .unwrap();
        assert_eq!(
            fixture
                .backend
                .call(BackendAction::MonitorDisks, &fixture.identity, json!({}))
                .unwrap_err()
                .code,
            if disabled {
                "UNAUTHORIZED"
            } else {
                "FORBIDDEN"
            }
        );
        let before = fixture.snapshot();
        assert!(
            fixture
                .backend
                .observe_disks(&crate::InternalDiskObservation(()))
                .is_ok()
        );
        assert!(fixture.snapshot() == before);
    }
}
