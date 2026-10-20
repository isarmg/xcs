//! Authorization and wire adaptation for the storage engine.
use crate::{ApiError, Backend, Content, auth::Identity};
use serde::Serialize;
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};
use xcs_core::{Engine, Grants, NamespaceOperation, Repository};
pub mod application;
use application::Application;
mod file_manager;

pub struct EngineBackend {
    engine: Mutex<Engine>,
}
impl EngineBackend {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Mutex::new(engine),
        }
    }
    pub fn protect_auth(
        self: &std::sync::Arc<Self>,
        mut auth: crate::auth::AuthStore,
    ) -> Result<crate::auth::AuthStore, ApiError> {
        let backend = self.clone();
        auth.attach_backup(std::sync::Arc::new(move |snapshot| {
            let mut engine = backend.engine.lock().map_err(|_| {
                ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart")
            })?;
            if engine.auth_state().as_ref() == Some(&snapshot) {
                return Ok(());
            }
            engine
                .persist_auth_state(snapshot, "system_auth", &xcs_core::new_id("op"))
                .map_err(error)
        }))?;
        Ok(auth)
    }
    pub fn open_auth(
        self: &std::sync::Arc<Self>,
        root: &Path,
    ) -> Result<crate::auth::AuthStore, ApiError> {
        let snapshot = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?
            .auth_state();
        self.protect_auth(crate::auth::AuthStore::open_with_snapshot(root, snapshot)?)
    }
}
fn error(error: xcs_core::Error) -> ApiError {
    let code = match error.code() {
        "PERMISSION_DENIED" => "FORBIDDEN",
        "UNAUTHENTICATED" => "UNAUTHORIZED",
        "FILE_VERSION_CONFLICT" => "FILE_CONFLICT",
        "BASE_HEAD_MISMATCH" => "HEAD_CONFLICT",
        "FILE_BUSY" => "FILE_WRITE_BUSY",
        "FENCING_TOKEN_INVALID" => "FENCED",
        "FENCING_TOKEN_EXPIRED" => "LEASE_EXPIRED",
        "RESOURCE_LIMIT" => "RESOURCE_LIMIT_EXCEEDED",
        "ROUTE_INCOMPLETE" => "BACKUP_ROUTE_INCOMPLETE",
        "IN_DOUBT" => "COMMIT_IN_DOUBT",
        "GENERATION_MISMATCH" => "CONFIG_CONFLICT",
        "STALE_UPLOAD_GENERATION" => "UPLOAD_GENERATION_CONFLICT",
        other => other,
    };
    let status = if code.ends_with("_NOT_FOUND") || code == "NOT_FOUND" {
        404
    } else {
        match code {
            "UNAUTHORIZED" => 401,
            "FORBIDDEN" => 403,
            "CONTENT_PURGED" => 410,
            "NO_SPACE" => 507,
            "RESOURCE_LIMIT_EXCEEDED" | "QUOTA_EXCEEDED" => 429,
            "CONTENT_UNAVAILABLE"
            | "METADATA_UNAVAILABLE"
            | "RECOVERY_REQUIRED"
            | "REPLICA_TARGET_UNAVAILABLE"
            | "ROUTE_TARGET_UNAVAILABLE"
            | "COMMIT_IN_DOUBT"
            | "BACKUP_ROUTE_INCOMPLETE" => 503,
            "HEAD_CONFLICT"
            | "BRANCH_NOT_ACTIVE"
            | "FENCED"
            | "LEASE_EXPIRED"
            | "UPLOAD_GENERATION_CONFLICT"
            | "FILE_CONFLICT"
            | "FILE_BRANCH_CONFLICT"
            | "DIRECTORY_BRANCH_CONFLICT"
            | "DIRECTORY_ALIAS_CONFLICT"
            | "BATCH_MAPPING_CONFLICT"
            | "BATCH_SCOPE_CONFLICT"
            | "IDEMPOTENCY_KEY_REUSED"
            | "SPLIT_PLAN_STALE"
            | "PURGE_BLOCKED_BY_REFERENCE"
            | "CONTENT_PINNED"
            | "CONFIG_CONFLICT"
            | "BRANCH_WRITE_BUSY"
            | "FILE_WRITE_BUSY"
            | "NAMESPACE_CONFLICT"
            | "NAME_COLLISION"
            | "WORKSPACE_CLOSED"
            | "PLAN_STALE"
            | "PLAN_CLOSED"
            | "TREE_MOVED" => 409,
            "RESTORE_BACKUP_CONFLICT" => 409,
            "DIRECTORY_BRANCH_INCOMPLETE" | "DIRECTORY_SCOPE_REQUIRED" => 409,
            "DIRECTORY_HISTORY_SPLIT_UNSUPPORTED" | "FILE_NODE_HISTORY_SPLIT_UNSUPPORTED" => 400,
            "IO_ERROR"
            | "STATE_INVALID"
            | "INTERNAL_ERROR"
            | "OBJECT_CORRUPT"
            | "LOG_CORRUPT"
            | "PUBLICATION_INVALID" => 500,
            "OPERATION_UNSUPPORTED" | "PLAN_UNSUPPORTED" => 501,
            _ => 400,
        }
    };
    if matches!(
        code,
        "IO_ERROR"
            | "STATE_INVALID"
            | "INTERNAL_ERROR"
            | "OBJECT_CORRUPT"
            | "LOG_CORRUPT"
            | "PUBLICATION_INVALID"
    ) {
        tracing::error!(%error,"storage operation failed");
        ApiError::new(
            status,
            code,
            "storage operation could not be completed; administrator inspection is required",
        )
    } else {
        ApiError::new(status, code, error.message.clone())
    }
}

fn encode(value: impl Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(ApiError::invalid)
}
/// Response ownership is independent from the version currently visible in a
/// branch. An unwritten branch reads its parent's immutable version.
fn content_wire(engine: &Engine, visible: Option<&str>, source: Option<&str>) -> Value {
    let visible = source.or(visible).filter(|version| !version.is_empty());
    let inherited = source.is_some();
    let state = visible
        .and_then(|version| engine.state().content_states.get(version))
        .map(String::as_str)
        .unwrap_or("LIVE");
    json!({"version_id":visible,"materialized":!inherited && visible.is_some(),
        "inherited":inherited,"own_version_id":(!inherited).then_some(visible).flatten(),
        "source_version_id":source,"content_state":if inherited { "PENDING" } else { state },
        "visible_content_state":state})
}
fn branch_wire(engine: &Engine, branch: &xcs_contract::Branch) -> Result<Value, ApiError> {
    let mut result = encode(branch)?;
    let snapshot = engine.state().commits.get(&branch.head_commit_id);
    let mut pending = snapshot
        .map(|commit| commit.pending_file_versions.clone())
        .unwrap_or_default();
    pending.extend(branch.pending_file_versions.clone());
    result["pending_file_versions"] = encode(&pending)?;
    result["materialized"] = pending.is_empty().into();
    result["inherited"] = (!pending.is_empty()).into();
    result["content_state"] = if pending.is_empty() {
        "LIVE"
    } else {
        "PENDING"
    }
    .into();
    if let Some(file) = branch.file_id.as_deref() {
        result["read_only"] = (branch.state != "active"
            || engine
                .state()
                .file_branch_aliases
                .contains_key(&branch.branch_id))
        .into();
        result["alias_of"] = engine
            .state()
            .file_branch_aliases
            .get(&branch.branch_id)
            .cloned()
            .into();
        let visible = snapshot
            .and_then(|commit| commit.entries.iter().find(|entry| entry.file_id == file))
            .map(|entry| entry.version_id.as_str());
        result["node_id"] = branch
            .file_node_id
            .clone()
            .or_else(|| visible.map(str::to_owned))
            .into();
        result.as_object_mut().unwrap().extend(
            content_wire(engine, visible, pending.get(file).map(String::as_str))
                .as_object()
                .unwrap()
                .clone(),
        );
    }
    Ok(result)
}
fn pending_entry_source<'a>(
    engine: &'a Engine,
    branch: &str,
    commit: &'a xcs_contract::Commit,
    entry: &xcs_contract::Entry,
) -> Option<&'a str> {
    if entry.kind != "file" {
        return None;
    }
    let state = engine.state();
    let visible = commit.entries.iter().find(|visible| {
        visible.lookup_key == entry.lookup_key && visible.file_id == entry.file_id
    })?;
    let current = state.branches.get(branch)?;
    if current.head_commit_id == commit.commit_id {
        if let Some(repo) = state
            .repositories
            .values()
            .find(|repo| repo.main_branch_id == branch)
        {
            if let Some(context) = engine.file_context(&repo.repository_id, &entry.path) {
                return if context.inherited_from.is_some() {
                    Some(visible.version_id.as_str())
                } else {
                    state
                        .branches
                        .get(&context.branch_id)?
                        .pending_file_versions
                        .get(&entry.file_id)
                        .map(String::as_str)
                };
            }
        }
    } else if let Some(context) = state
        .directory_trees
        .values()
        .filter(|tree| tree.commit_id == commit.commit_id)
        .find_map(|tree| tree.file_contexts.get(&entry.lookup_key))
    {
        return context
            .inherited_from
            .as_ref()
            .map(|_| visible.version_id.as_str());
    }
    commit
        .pending_file_versions
        .get(&entry.file_id)
        .filter(|version| **version == entry.version_id)
        .map(String::as_str)
        .or_else(|| {
            (current.head_commit_id == commit.commit_id)
                .then(|| engine.pending_file_source(branch, &entry.file_id))
                .flatten()
        })
}

fn current_entry_source<'a>(
    engine: &'a Engine,
    branch: &str,
    entry: &xcs_contract::Entry,
) -> Option<&'a str> {
    let current = engine.state().branches.get(branch)?;
    let commit = engine.state().commits.get(&current.head_commit_id)?;
    pending_entry_source(engine, branch, commit, entry)
}
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::invalid(format!("missing string field {key}")))
}
fn operation(input: &Value) -> Result<&str, ApiError> {
    field(input, "operation_id")
}
fn keys(input: &Value, allowed: &[&str]) -> Result<(), ApiError> {
    let object = input
        .as_object()
        .ok_or_else(|| ApiError::invalid("expected object"))?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(ApiError::invalid(format!("unknown field {key}")));
    }
    Ok(())
}
fn sanitized<T: Serialize>(value: T) -> Result<Value, ApiError> {
    let mut value = encode(value)?;
    redact(&mut value);
    Ok(value)
}
fn redact(value: &mut Value) {
    match value {
        Value::Object(object) => {
            object.remove("source_path");
            for value in object.values_mut() {
                redact(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                redact(value);
            }
        }
        _ => {}
    }
}
fn repository<'a>(engine: &'a Engine, id: &str) -> Result<&'a Repository, ApiError> {
    engine
        .state()
        .repositories
        .get(id)
        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "repository not found"))
}
fn repo_for_history<'a>(engine: &'a Engine, id: &str) -> Result<&'a Repository, ApiError> {
    let history = engine
        .state()
        .histories
        .get(id)
        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "history not found"))?;
    repository(engine, &history.repository_id)
}
fn repo_for_branch<'a>(engine: &'a Engine, id: &str) -> Result<&'a Repository, ApiError> {
    let branch = engine
        .state()
        .branches
        .get(id)
        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "branch not found"))?;
    if branch.state == "migrated" {
        return Err(ApiError::new(
            409,
            "TREE_MOVED",
            "branch has moved to an independent history domain",
        ));
    }
    repo_for_history(engine, &branch.history_tree_id)
}
fn authorize(
    identity: &Identity,
    repo: &Repository,
    write: bool,
    manage: bool,
) -> Result<(), ApiError> {
    authorize_access(
        identity,
        repo,
        if manage {
            RepositoryAccess::Manage
        } else if write {
            RepositoryAccess::Write
        } else {
            RepositoryAccess::Read
        },
    )
}

/// Account admission and ACL evaluation shared by all transports and commands.
#[derive(Debug, Clone, Copy)]
pub enum RepositoryAccess {
    Read,
    Write,
    Manage,
}

pub fn authorize_access(
    identity: &Identity,
    repo: &Repository,
    access: RepositoryAccess,
) -> Result<(), ApiError> {
    if identity.admin {
        return Ok(());
    }
    let admitted = identity.can_access(&repo.repository_id)
        || repo
            .account_scope_repository_id
            .as_deref()
            .is_some_and(|source| identity.can_access(source));
    if !admitted {
        return Err(ApiError::forbidden());
    }
    let grants = &repo.grants;
    let allowed = match access {
        RepositoryAccess::Manage => grants.manage.contains(&identity.principal),
        RepositoryAccess::Write => {
            grants.write.contains(&identity.principal)
                || grants.manage.contains(&identity.principal)
        }
        RepositoryAccess::Read => {
            grants.read.contains(&identity.principal)
                || grants.write.contains(&identity.principal)
                || grants.manage.contains(&identity.principal)
        }
    };
    if allowed {
        Ok(())
    } else {
        Err(ApiError::forbidden())
    }
}

#[cfg(test)]
mod authorization_tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn upload_generation_and_lease_errors_preserve_conflict_status() {
        for (core_code, api_code) in [
            ("STALE_UPLOAD_GENERATION", "UPLOAD_GENERATION_CONFLICT"),
            ("UPLOAD_GENERATION_CONFLICT", "UPLOAD_GENERATION_CONFLICT"),
            ("FENCING_TOKEN_EXPIRED", "LEASE_EXPIRED"),
        ] {
            let error = error(xcs_core::Error::new(core_code, "retry needs new state"));
            assert_eq!(error.status, 409, "{core_code}");
            assert_eq!(error.code, api_code);
        }
    }

    #[test]
    fn split_repository_accepts_independent_account_admission_and_still_requires_its_own_acl() {
        let mut repository = Repository {
            repository_id: "new repository".into(),
            name: "Independent split".into(),
            history_tree_id: "history".into(),
            main_branch_id: "branch".into(),
            primary_slot_id: "P01".into(),
            target_copies: 3,
            name_policy: "PortableNameV2".into(),
            unicode_version: "16.0.0".into(),
            grants: Grants::owner("reader"),
            account_scope_repository_id: Some("source repository".into()),
            smb_enabled: false,
            file_manager_root: false,
        };
        let direct = Identity {
            principal: "reader".into(),
            admin: false,
            repositories: BTreeSet::from([repository.repository_id.clone()]),
        };
        let inherited = Identity {
            repositories: BTreeSet::from(["source repository".into()]),
            ..direct.clone()
        };
        assert!(authorize(&direct, &repository, false, false).is_ok());
        assert!(authorize(&inherited, &repository, false, false).is_ok());
        let unrelated = Identity {
            repositories: BTreeSet::from(["unrelated repository".into()]),
            ..direct.clone()
        };
        assert!(authorize(&unrelated, &repository, false, false).is_err());
        repository.grants = Grants::default();
        assert!(authorize(&direct, &repository, false, false).is_err());
        assert!(authorize(&inherited, &repository, false, false).is_err());
    }
}
fn workspace_authorize(
    engine: &Engine,
    identity: &Identity,
    id: &str,
) -> Result<xcs_core::Workspace, ApiError> {
    let workspace = engine.workspace(id).map_err(error)?;
    if workspace.principal != identity.principal && !identity.admin {
        return Err(ApiError::forbidden());
    }
    authorize(
        identity,
        repo_for_branch(engine, &workspace.branch_id)?,
        true,
        false,
    )?;
    Ok(workspace)
}

fn operation_wire(_engine: &Engine, op: xcs_core::OperationResult) -> Result<Value, ApiError> {
    let health = match op.current_health.as_str() {
        "HEALTHY" | "DEGRADED" | "UNAVAILABLE" | "UNKNOWN" => op.current_health.as_str(),
        _ => "UNKNOWN",
    };
    let current_health = json!({"state":health,"checked_at":time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).map_err(ApiError::invalid)?});
    let content_save =
        op.kind == "workspace.commit" && op.protection_at_publish != "NOT_APPLICABLE";
    if content_save {
        let complete = matches!(
            op.commit_state.as_str(),
            "PUBLISHED" | "ACKED" | "PUBLISHED_DEGRADED"
        );
        let state = if complete {
            "completed"
        } else if matches!(op.commit_state.as_str(), "IN_DOUBT" | "RECOVERY_REQUIRED") {
            "in_doubt"
        } else {
            "processing"
        };
        let result = if complete {
            let versions = op
                .result
                .get("version_protection")
                .filter(|versions| versions.is_array())
                .ok_or_else(|| {
                    ApiError::new(
                        500,
                        "STATE_INVALID",
                        "frozen publication protection evidence is missing",
                    )
                })?;
            json!({"outcome":if op.protection_at_publish=="TARGET_MET"{"COMMITTED_AND_PROTECTED"}else{"COMMITTED_DEGRADED"},"commit_id":op.commit_id,"branch_id":op.branch_id,"branch_generation":field(&op.result,"branch_generation")?,"publication_sequence":op.sequence.to_string(),"membership_epoch":field(&op.result,"membership_epoch")?,"protection_at_publish":op.protection_at_publish,"versions":versions})
        } else {
            Value::Null
        };
        Ok(
            json!({"operation_id":op.operation_id,"operation_kind":"content_save","request_digest":op.request_digest,"state":state,"commit_state":op.commit_state,"result":result,"error":null,"current_health":current_health}),
        )
    } else {
        Ok(
            json!({"operation_id":op.operation_id,"operation_kind":op.kind,"request_digest":op.request_digest,"state":if matches!(op.commit_state.as_str(),"PUBLISHED"|"ACKED"){"completed"}else{"in_doubt"},"commit_state":op.commit_state,"protection_at_publish":op.protection_at_publish,"publication_sequence":op.sequence.to_string(),"result":op.result,"error":null,"current_health":current_health}),
        )
    }
}

impl Backend for EngineBackend {
    fn call(&self, action: &str, identity: &Identity, input: Value) -> Result<Value, ApiError> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        match action {
            action if action.starts_with("file_manager_") => {
                file_manager::call(&mut engine, identity, action, input)
            }
            "monitor_disks" => {
                identity.require_admin()?;
                let mut disks = Vec::new();
                for disk in engine.disk_status().map_err(error)? {
                    let capacity = engine.disk_capacity(&disk.slot_id).ok();
                    let (power_watts, power_unavailable_reason) = crate::telemetry::disk_power(
                        &disk.physical_device,
                        disk.simulation,
                        disk.online,
                    );
                    let usage_percent = capacity
                        .filter(|capacity| capacity.0 > 0)
                        .map(|capacity| capacity.2 as f64 / capacity.0 as f64 * 100.0);
                    disks.push(json!({"id":disk.slot_id,"name":disk.slot_id,"online":disk.online,"read_only":disk.level!=1,"simulation":disk.simulation,
                        "total_bytes":capacity.map(|c|c.0.to_string()),"available_bytes":capacity.map(|c|c.1.to_string()),"used_bytes":capacity.map(|c|c.2.to_string()),"usage_percent":usage_percent,"power_watts":power_watts,"power_unavailable_reason":power_unavailable_reason}));
                }
                Ok(json!({"disks":disks}))
            }
            "capabilities" => Ok(
                json!({"protocol_version":"1","disk_format_version":2,"name_policy_version":"PortableNameV2","path_policy_version":"PortablePathV1","unicode_data_version":"16.0.0","supported_save_modes":["full"],"full_only":true,"full_branches":false,"branch_creation":"record-until-write","branch_content":"inherit-parent-until-first-write","file_branches":true,"branch_scope":"file","directory_branches":true,"directory_branch_creation":"inherit-directory-tree","directory_subtrees":true,"namespace_batch":true,"smb_default_branch":"selected-root-directory","file_manager":true,"historical_save_creates_branch":true,"merge_behavior":"explicit-file-node-adoption","file_branch_merge":true,"directory_branch_merge":false,"directory_merge_status":"design-pending","file_node_tags":true,"backup_configuration":true,"range_upload":false,"complete_upload":true,"max_upload_bytes":engine.config().max_file_bytes.to_string(),"authentication":"session-bearer-or-cookie","platforms":{"web":true,"cli":true,"native_app":"requires-platform-build"},"operations":["repositories","tree","workspace","commit","history","graph","branch","merge-link","file-branch-merge","file-node-tags","backup-configuration","purge-plan","split-plan","version-range-read","file-manager","file-branches","file-branch-rename","file-path-nodes","directory-branches","directory-subtrees","namespace-batch","default-branch","file-notes","system-status"]}),
            ),
            "ready" => {
                if engine.is_read_only() {
                    return Err(ApiError::new(
                        503,
                        "RECOVERY_REQUIRED",
                        "storage is open for verified recovery reads only",
                    ));
                }
                let status = engine.disk_status().map_err(error)?;
                let members = status
                    .iter()
                    .filter(|disk| disk.metadata_member && disk.online)
                    .count();
                if members < 3 {
                    return Err(ApiError::new(
                        503,
                        "METADATA_UNAVAILABLE",
                        "all three metadata members are required for normal writes",
                    ));
                }
                Ok(
                    json!({"ready":true,"mode":engine.config().mode,"sequence":engine.state().sequence.to_string()}),
                )
            }
            "repositories" => {
                let values = engine
                    .state()
                    .repositories
                    .values()
                    .filter(|repo| authorize(identity, repo, false, false).is_ok())
                    .collect::<Vec<_>>();
                Ok(json!({"repositories":values}))
            }
            "repository" => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, false, false)?;
                encode(repo)
            }
            "branches" => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, false, false)?;
                Ok(
                    json!({"branches":engine.state().branches.values().filter(|branch|branch.history_tree_id==repo.history_tree_id).map(|branch| branch_wire(&engine, branch)).collect::<Result<Vec<_>, _>>()?}),
                )
            }
            "tree" => {
                let branch = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch)?, false, false)?;
                let branch_info = engine.state().branches.get(branch).unwrap();
                let commit = input
                    .get("query")
                    .and_then(|v| v.get("commit_id"))
                    .and_then(Value::as_str)
                    .unwrap_or(&branch_info.head_commit_id);
                let snapshot = engine
                    .state()
                    .commits
                    .get(commit)
                    .filter(|commit| commit.history_tree_id == branch_info.history_tree_id)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "commit not found"))?;
                let entries = engine.list_tree_at(branch, commit).map_err(error)?;
                let pending_file_versions = entries
                    .iter()
                    .filter_map(|entry| {
                        pending_entry_source(&engine, branch, snapshot, entry)
                            .map(|source| (entry.file_id.as_str(), source))
                    })
                    .collect::<std::collections::BTreeMap<_, _>>();
                Ok(
                    json!({"branch_id":branch,"commit_id":commit,"entries":entries,"pending_file_versions":pending_file_versions}),
                )
            }
            "smb_binding" => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, false, true)?;
                let history = engine
                    .state()
                    .histories
                    .get(&repo.history_tree_id)
                    .ok_or_else(|| {
                        ApiError::new(500, "STATE_INVALID", "repository history is missing")
                    })?;
                Ok(
                    json!({"repository_id":repo.repository_id,"branch_id":repo.main_branch_id,"enabled":repo.smb_enabled,"history_generation":history.generation.to_string()}),
                )
            }
            "default_branch" => {
                let repo = repository(&engine, field(&input, "repository_id")?)?;
                authorize(identity, repo, true, false)?;
                let selected = engine.state().directory_selections.get(&repo.repository_id);
                let name = selected
                    .and_then(|id| engine.state().directory_branches.get(id))
                    .map(|branch| branch.name.as_str())
                    .unwrap_or("main");
                let branches = engine
                    .directory_branches(&repo.repository_id, "")
                    .map_err(error)?;
                Ok(
                    json!({"repository_id":repo.repository_id,"namespace_branch_id":repo.main_branch_id,
                    "directory_branch_id":selected,"name":name,"directory_branches":branches}),
                )
            }
            "directory_branch_scopes" => file_manager::directory_branch_scopes(
                &engine,
                identity,
                field(&input, "repository_id")?,
                input
                    .get("query")
                    .and_then(|query| query.get("path"))
                    .and_then(Value::as_str),
            ),
            "initialize_directory_scope" => {
                keys(&input, &["repository_id", "path", "operation_id"])?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, false)?;
                // The empty path explicitly identifies the repository root.
                let path = input
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::invalid("path must be a string"))?;
                if !path.is_empty() {
                    xcs_naming::validate_path(path).map_err(ApiError::invalid)?;
                }
                let result = engine
                    .initialize_directory_scope(repo, path, &identity.principal, operation(&input)?)
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            "update_default_branch" => {
                keys(
                    &input,
                    &["repository_id", "directory_branch_id", "operation_id"],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, false)?;
                let result = engine
                    .select_default_directory_branch(
                        repo,
                        field(&input, "directory_branch_id")?,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            "rename_directory_branch" => {
                keys(
                    &input,
                    &[
                        "repository_id",
                        "directory_branch_id",
                        "name",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, false)?;
                let result = engine
                    .rename_directory_branch(
                        repo,
                        field(&input, "directory_branch_id")?,
                        field(&input, "name")?,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            "update_smb_binding" => {
                identity.require_admin()?;
                keys(
                    &input,
                    &[
                        "repository_id",
                        "branch_id",
                        "enabled",
                        "expected_history_generation",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, true)?;
                let enabled = input
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| ApiError::invalid("enabled must be a boolean"))?;
                let generation =
                    xcs_contract::parse_u64_decimal(field(&input, "expected_history_generation")?)
                        .map_err(ApiError::invalid)?;
                let result = engine
                    .set_smb_binding(
                        repo,
                        field(&input, "branch_id")?,
                        enabled,
                        generation,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            "set_grants" => {
                keys(&input, &["repository_id", "grants", "operation_id"])?;
                let repository_id = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repository_id)?, true, true)?;
                let grants: Grants = serde_json::from_value(
                    input
                        .get("grants")
                        .cloned()
                        .ok_or_else(|| ApiError::invalid("grants required"))?,
                )
                .map_err(ApiError::invalid)?;
                encode(
                    engine
                        .set_grants(
                            repository_id,
                            grants,
                            &identity.principal,
                            operation(&input)?,
                        )
                        .map_err(error)?,
                )
            }
            "resolve" => {
                keys(&input, &["branch_id", "path", "commit_id"])?;
                let branch_id = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch_id)?, false, false)?;
                let branch = engine.state().branches.get(branch_id).unwrap();
                let path = field(&input, "path")?;
                xcs_naming::validate_path(path).map_err(ApiError::invalid)?;
                let commit_id = input
                    .get("commit_id")
                    .and_then(Value::as_str)
                    .unwrap_or(&branch.head_commit_id);
                let commit = engine
                    .state()
                    .commits
                    .get(commit_id)
                    .filter(|commit| commit.history_tree_id == branch.history_tree_id)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "commit not found"))?;
                let key = xcs_naming::path_key(path);
                let entries = engine.list_tree_at(branch_id, commit_id).map_err(error)?;
                let entry = entries
                    .iter()
                    .find(|entry| entry.lookup_key == key)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "entry not found"))?;
                let mut response = json!({"branch_id":branch_id,"commit_id":commit_id,"entry":entry,"version":engine.state().versions.get(&entry.version_id)});
                if entry.kind == "file" {
                    response.as_object_mut().unwrap().extend(
                        content_wire(
                            &engine,
                            Some(&entry.version_id),
                            pending_entry_source(&engine, branch_id, commit, entry),
                        )
                        .as_object()
                        .unwrap()
                        .clone(),
                    );
                }
                Ok(response)
            }
            "workspace" => sanitized(workspace_authorize(
                &engine,
                identity,
                field(&input, "workspace_id")?,
            )?),
            "plan" => {
                let plan = engine.plan(field(&input, "plan_id")?).map_err(error)?;
                if plan.principal != identity.principal && !identity.admin {
                    return Err(ApiError::forbidden());
                }
                authorize(
                    identity,
                    repo_for_history(&engine, field(&plan.input, "history_tree_id")?)?,
                    true,
                    plan.kind != "historical_save",
                )?;
                encode(plan)
            }
            "create_repository" => {
                identity.require_admin()?;
                keys(
                    &input,
                    &["name", "primary_slot_id", "target_copies", "operation_id"],
                )?;
                let replicas = input
                    .get("target_copies")
                    .and_then(Value::as_u64)
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or_else(|| ApiError::invalid("target_copies must be a positive integer"))?;
                encode(
                    engine
                        .create_repository(
                            field(&input, "name")?,
                            field(&input, "primary_slot_id")?,
                            replicas,
                            &identity.principal,
                            operation(&input)?,
                        )
                        .map_err(error)?,
                )
            }
            "write_session" => {
                keys(&input, &["branch_id"])?;
                let branch = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch)?, true, false)?;
                let workspace = engine
                    .create_workspace(branch, &identity.principal)
                    .map_err(error)?;
                Ok(
                    json!({"write_session_id":workspace.workspace_id,"workspace_id":workspace.workspace_id,"branch_id":workspace.branch_id,"principal":workspace.principal,"fencing_token":workspace.fencing_token.to_string(),"expires_at":workspace.created_at.saturating_add(engine.config().workspace_ttl_seconds).to_string(),"scope":"workspace","state":workspace.state}),
                )
            }
            "compare" => {
                keys(&input, &["left_commit_id", "right_commit_id"])?;
                let left = field(&input, "left_commit_id")?;
                let right = field(&input, "right_commit_id")?;
                for id in [left, right] {
                    let commit = engine.state().commits.get(id).ok_or_else(|| {
                        ApiError::new(404, "NOT_FOUND", "comparison commit not found")
                    })?;
                    authorize(
                        identity,
                        repo_for_history(&engine, &commit.history_tree_id)?,
                        false,
                        false,
                    )?;
                }
                engine.compare_commits(left, right).map_err(error)
            }
            "plan_cancel" => {
                keys(&input, &["plan_id", "operation_id"])?;
                let plan = engine.plan(field(&input, "plan_id")?).map_err(error)?;
                if plan.principal != identity.principal && !identity.admin {
                    return Err(ApiError::forbidden());
                }
                authorize(
                    identity,
                    repo_for_history(&engine, field(&plan.input, "history_tree_id")?)?,
                    true,
                    plan.kind != "historical_save",
                )?;
                encode(
                    engine
                        .cancel_plan(
                            field(&input, "plan_id")?,
                            &identity.principal,
                            operation(&input)?,
                        )
                        .map_err(error)?,
                )
            }
            "create_workspace" => {
                keys(&input, &["branch_id", "source_commit_id"])?;
                let branch = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, branch)?, true, false)?;
                let workspace =
                    if let Some(source) = input.get("source_commit_id").and_then(Value::as_str) {
                        engine
                            .create_historical_workspace(branch, source, &identity.principal)
                            .map_err(error)?
                    } else {
                        engine
                            .create_workspace(branch, &identity.principal)
                            .map_err(error)?
                    };
                sanitized(workspace)
            }
            "branch" => {
                let id = field(&input, "branch_id")?;
                authorize(identity, repo_for_branch(&engine, id)?, false, false)?;
                branch_wire(
                    &engine,
                    engine
                        .state()
                        .branches
                        .get(id)
                        .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "branch not found"))?,
                )
            }
            "historical_save_plan" => {
                keys(
                    &input,
                    &[
                        "history_tree_id",
                        "source_commit_id",
                        "name",
                        "workspace_id",
                        "primary_slot_id",
                        "target_copies",
                    ],
                )?;
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, true, false)?;
                let mut options = input.clone();
                options.as_object_mut().unwrap().remove("history_tree_id");
                let options: xcs_core::HistoricalSaveOptions =
                    serde_json::from_value(options).map_err(ApiError::invalid)?;
                encode(
                    engine
                        .create_historical_save_plan(history, options, &identity.principal)
                        .map_err(error)?,
                )
            }
            "workspace_authorize" => {
                workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                Ok(json!({"authorized":true}))
            }
            "workspace_selection_authorize" => {
                keys(&input, &["workspace_id", "path"])?;
                let id = field(&input, "workspace_id")?;
                workspace_authorize(&engine, identity, id)?;
                engine
                    .validate_workspace_file_selection(id, field(&input, "path")?)
                    .map_err(error)?;
                Ok(json!({"authorized":true}))
            }
            "upload_authorize" => {
                keys(&input, &["workspace_id", "item_id", "logical_size"])?;
                let workspace =
                    workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                xcs_contract::validate_id(field(&input, "item_id")?, "entry")
                    .map_err(ApiError::invalid)?;
                if workspace.state != "open" {
                    return Err(ApiError::conflict(
                        "WORKSPACE_CLOSED",
                        "workspace is no longer open for upload",
                    ));
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(ApiError::invalid)?
                    .as_secs();
                if now.saturating_sub(workspace.created_at) > engine.config().workspace_ttl_seconds
                {
                    return Err(ApiError::conflict(
                        "LEASE_EXPIRED",
                        "workspace lease expired; retained data remains available",
                    ));
                }
                if input.get("logical_size").is_some() {
                    let size = xcs_contract::parse_u64_decimal(field(&input, "logical_size")?)
                        .map_err(ApiError::invalid)?;
                    if size > engine.config().max_file_bytes
                        || size > engine.config().max_workspace_bytes
                    {
                        return Err(ApiError::new(
                            413,
                            "RESOURCE_LIMIT_EXCEEDED",
                            "declared upload exceeds the pool file or workspace limit",
                        ));
                    }
                }
                Ok(json!({"authorized":true}))
            }
            "upload_status" => {
                let workspace =
                    workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                let item = field(&input, "item_id")?;
                let staged = workspace
                    .files
                    .get(item)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "upload item not found"))?;
                let sealed = matches!(workspace.state.as_str(), "sealed" | "committed");
                let manifest = if sealed {
                    Some(xcs_core::workspace_manifest_hash(&workspace).map_err(error)?)
                } else {
                    None
                };
                Ok(
                    json!({"workspace_id":workspace.workspace_id,"item_id":item,"upload_generation":staged.dirty_generation.to_string(),"logical_size":staged.size.to_string(),"content_hash":staged.content_hash,"state":if sealed{"SEALED"}else{"COMPLETE"},"durable_ranges":if staged.size>0{vec![json!({"offset":"0","length":staged.size.to_string(),"content_hash":staged.content_hash})]}else{vec![]},"sealed_manifest_hash":manifest}),
                )
            }
            "namespace" => {
                keys(&input, &["workspace_id", "kind", "path", "destination"])?;
                let workspace = field(&input, "workspace_id")?;
                let change = NamespaceOperation {
                    kind: field(&input, "kind")?.into(),
                    path: field(&input, "path")?.into(),
                    destination: input
                        .get("destination")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                };
                sanitized(Application::new(&mut engine, identity).stage_namespace(
                    application::StageNamespace {
                        workspace_id: workspace.into(),
                        change,
                    },
                )?)
            }
            "seal_workspace" => {
                keys(&input, &["workspace_id"])?;
                let workspace =
                    workspace_authorize(&engine, identity, field(&input, "workspace_id")?)?;
                engine
                    .seal_workspace(&workspace.workspace_id, &workspace.principal)
                    .map_err(error)
            }
            "commit_workspace" | "commit_workspace_internal" => {
                if action == "commit_workspace" {
                    keys(
                        &input,
                        &[
                            "workspace_id",
                            "operation_id",
                            "repository_id",
                            "branch_id",
                            "expected_head",
                            "fencing_token",
                            "message",
                            "requested_protection",
                            "file_placement",
                            "expected_topology_generation",
                            "sealed_manifest_hash",
                            "request_digest",
                        ],
                    )?;
                } else {
                    keys(
                        &input,
                        &[
                            "workspace_id",
                            "operation_id",
                            "expected_head",
                            "fencing_token",
                            "message",
                            "request_digest",
                        ],
                    )?;
                }
                // Keep strict string presence checks at the compatibility wire edge.
                for key in [
                    "workspace_id",
                    "operation_id",
                    "expected_head",
                    "fencing_token",
                ] {
                    field(&input, key)?;
                }
                let result = if action == "commit_workspace" {
                    let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                    Application::new(&mut engine, identity).commit_workspace_public(command)?
                } else {
                    let command = application::CommitWorkspace {
                        workspace_id: field(&input, "workspace_id")?.into(),
                        operation_id: operation(&input)?.into(),
                        expected_head: field(&input, "expected_head")?.into(),
                        fencing_token: xcs_contract::parse_u64_decimal(field(
                            &input,
                            "fencing_token",
                        )?)
                        .map_err(ApiError::invalid)?,
                        message: input
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .into(),
                        request_digest: input
                            .get("request_digest")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    };
                    Application::new(&mut engine, identity).commit_workspace(command)?
                };
                operation_wire(&engine, result)
            }
            "abort_workspace" => {
                keys(&input, &["workspace_id"])?;
                let id = field(&input, "workspace_id")?;
                workspace_authorize(&engine, identity, id)?;
                engine
                    .abort_workspace(id, &identity.principal)
                    .map_err(error)?;
                Ok(json!({"workspace_id":id,"state":"aborted"}))
            }
            "create_branch" => {
                for key in ["repository_id", "source_commit_id", "name", "operation_id"] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                let branch = Application::new(&mut engine, identity).create_branch(command)?;
                branch_wire(&engine, &branch)
            }
            "archive" => {
                for key in ["branch_id", "operation_id"] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                encode(Application::new(&mut engine, identity).archive_branch(command)?)
            }
            "merge_link" => {
                for key in [
                    "history_tree_id",
                    "source_branch_id",
                    "source_commit_id",
                    "target_branch_id",
                    "target_commit_id",
                    "operation_id",
                ] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                encode(Application::new(&mut engine, identity).merge_link(command)?)
            }
            "history" => {
                let file = field(&input, "file_id")?;
                let versions = engine.file_history(file).map_err(error)?;
                let visible = versions
                    .into_iter()
                    .filter(|version| {
                        repo_for_history(&engine, &version.history_tree_id)
                            .is_ok_and(|repo| authorize(identity, repo, false, false).is_ok())
                    })
                    .collect::<Vec<_>>();
                if visible.is_empty() {
                    return Err(ApiError::new(404, "NOT_FOUND", "file history not found"));
                }
                Ok(json!({"file_id":file,"versions":visible}))
            }
            "graph" => {
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, false, false)?;
                encode(engine.graph(history).map_err(error)?)
            }
            "commit" => {
                let commit = engine
                    .state()
                    .commits
                    .get(field(&input, "commit_id")?)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "commit not found"))?;
                authorize(
                    identity,
                    repo_for_history(&engine, &commit.history_tree_id)?,
                    false,
                    false,
                )?;
                encode(commit)
            }
            "version" | "placement" => {
                let id = field(&input, "version_id")?;
                let version = engine
                    .state()
                    .versions
                    .get(id)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "version not found"))?;
                authorize(
                    identity,
                    repo_for_history(&engine, &version.history_tree_id)?,
                    false,
                    false,
                )?;
                if action == "version" {
                    let mut value = encode(version)?;
                    value["content_state"] = engine
                        .state()
                        .content_states
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| "available".into())
                        .into();
                    Ok(value)
                } else {
                    encode(
                        engine.state().placements.get(id).ok_or_else(|| {
                            ApiError::new(404, "NOT_FOUND", "placement not found")
                        })?,
                    )
                }
            }
            "operation" => {
                let operation_id = field(&input, "operation_id")?;
                let operation = engine.operation(operation_id).map_err(error)?;
                if !identity.admin
                    && engine.operation_owner(operation_id).as_deref() != Some(&identity.principal)
                {
                    return Err(ApiError::forbidden());
                }
                if let Some(history) = &operation.history_tree_id {
                    authorize(identity, repo_for_history(&engine, history)?, false, false)?;
                } else {
                    identity.require_admin()?;
                }
                operation_wire(&engine, operation)
            }
            "purge_plan" => {
                keys(&input, &["history_tree_id", "version_ids"])?;
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, true, true)?;
                let versions: Vec<String> = serde_json::from_value(
                    input
                        .get("version_ids")
                        .cloned()
                        .ok_or_else(|| ApiError::invalid("version_ids required"))?,
                )
                .map_err(ApiError::invalid)?;
                encode(
                    engine
                        .create_purge_plan(history, versions, &identity.principal)
                        .map_err(error)?,
                )
            }
            "split_plan" => {
                keys(&input, &["history_tree_id", "cut_commit_id"])?;
                let history = field(&input, "history_tree_id")?;
                authorize(identity, repo_for_history(&engine, history)?, true, true)?;
                encode(
                    engine
                        .create_split_plan(
                            history,
                            field(&input, "cut_commit_id")?,
                            &identity.principal,
                        )
                        .map_err(error)?,
                )
            }
            "plan_commit" => {
                for key in ["plan_id", "operation_id", "plan_digest"] {
                    field(&input, key)?;
                }
                let command = serde_json::from_value(input).map_err(ApiError::invalid)?;
                encode(Application::new(&mut engine, identity).commit_plan(command)?)
            }
            "policy" => {
                let policy = engine
                    .state()
                    .protection_policies
                    .get(field(&input, "policy_id")?)
                    .ok_or_else(|| {
                        ApiError::new(404, "NOT_FOUND", "protection policy not found")
                    })?;
                authorize(
                    identity,
                    repository(&engine, &policy.repository_id)?,
                    false,
                    false,
                )?;
                encode(policy)
            }
            "placement_rule" => {
                let rule = engine
                    .state()
                    .placement_rules
                    .get(field(&input, "rule_id")?)
                    .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "placement rule not found"))?;
                authorize(
                    identity,
                    repository(&engine, &rule.repository_id)?,
                    false,
                    false,
                )?;
                encode(rule)
            }
            "update_policy" => {
                keys(
                    &input,
                    &[
                        "policy_id",
                        "repository_id",
                        "path_prefix",
                        "target_copies",
                        "expected_generation",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, true)?;
                let generation =
                    xcs_contract::parse_u64_decimal(field(&input, "expected_generation")?)
                        .map_err(ApiError::invalid)?;
                let copies = input
                    .get("target_copies")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| ApiError::invalid("target_copies must be an integer"))?;
                let prefix = input
                    .get("path_prefix")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::invalid("path_prefix must be a string"))?;
                let result = engine
                    .update_protection_policy(
                        field(&input, "policy_id")?,
                        repo,
                        prefix,
                        copies,
                        generation,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            "update_placement_rule" => {
                keys(
                    &input,
                    &[
                        "rule_id",
                        "repository_id",
                        "path_prefix",
                        "primary_slot_id",
                        "expected_generation",
                        "operation_id",
                    ],
                )?;
                let repo = field(&input, "repository_id")?;
                authorize(identity, repository(&engine, repo)?, true, true)?;
                let generation =
                    xcs_contract::parse_u64_decimal(field(&input, "expected_generation")?)
                        .map_err(ApiError::invalid)?;
                let prefix = input
                    .get("path_prefix")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ApiError::invalid("path_prefix must be a string"))?;
                let result = engine
                    .update_placement_rule(
                        field(&input, "rule_id")?,
                        repo,
                        prefix,
                        field(&input, "primary_slot_id")?,
                        generation,
                        &identity.principal,
                        operation(&input)?,
                    )
                    .map_err(error)?;
                operation_wire(&engine, result)
            }
            "check" => {
                identity.require_admin()?;
                encode(engine.check().map_err(error)?)
            }
            "disks" => {
                identity.require_admin()?;
                encode(json!({"disks":engine.disk_status().map_err(error)?}))
            }
            "topology" => {
                identity.require_admin()?;
                Ok(
                    json!({"topology_id":"default","generation":engine.state().topology_generation.to_string(),"slots":engine.config().disks.iter().map(|disk|json!({"slot_id":disk.slot_id,"disk_id":disk.disk_id,"level":disk.level,"upstream_slot_id":disk.upstream_slot_id,"metadata_member":disk.metadata_member,"binding_generation":disk.binding_generation.to_string()})).collect::<Vec<_>>()}),
                )
            }
            "events" => {
                let after = xcs_contract::parse_u64_decimal(
                    input
                        .get("query")
                        .and_then(|query| query.get("after"))
                        .and_then(Value::as_str)
                        .unwrap_or("0"),
                )
                .map_err(ApiError::invalid)?;
                if after > engine.state().sequence {
                    return Err(ApiError::new(
                        400,
                        "INVALID_CURSOR",
                        "event cursor is ahead of the published sequence",
                    ));
                }
                let mut events = Vec::new();
                let mut inspected = after;
                for event in engine
                    .state()
                    .events
                    .iter()
                    .filter(|event| event.sequence > after)
                {
                    inspected = event.sequence;
                    if identity.admin
                        || engine
                            .state()
                            .operations
                            .get(&event.operation_id)
                            .and_then(|op| op.history_tree_id.as_ref())
                            .and_then(|history| repo_for_history(&engine, history).ok())
                            .is_some_and(|repo| authorize(identity, repo, false, false).is_ok())
                    {
                        events.push(event);
                    }
                    if events.len() >= 1000 {
                        break;
                    }
                }
                Ok(json!({"events":events,"next_cursor":inspected.to_string()}))
            }
            "register_disk" => {
                identity.require_admin()?;
                Err(ApiError::new(
                    409,
                    "OFFLINE_CONFIGURATION_REQUIRED",
                    "disk registration requires an explicit offline configuration and topology migration",
                ))
            }
            _ => Err(ApiError::unsupported()),
        }
    }
    fn upload(
        &self,
        identity: &Identity,
        workspace: &str,
        _item: &str,
        path: &str,
        source: &Path,
    ) -> Result<Value, ApiError> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        workspace_authorize(&engine, identity, workspace)?;
        sanitized(
            engine
                .stage_file_as(workspace, _item, path, source)
                .map_err(error)?,
        )
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
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        workspace_authorize(&engine, identity, workspace)?;
        sanitized(
            engine
                .stage_upload(workspace, item, path, source, generation)
                .map_err(error)?,
        )
    }
    fn content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        self.range_content(identity, version)?.fully_verified()
    }
    fn range_content(&self, identity: &Identity, version: &str) -> Result<Content, ApiError> {
        let engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        let record = engine
            .state()
            .versions
            .get(version)
            .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "version not found"))?;
        authorize(
            identity,
            repo_for_history(&engine, &record.history_tree_id)?,
            false,
            false,
        )?;
        let plan = engine.prepare_content(version).map_err(error)?;
        let repo = repo_for_history(&engine, &record.history_tree_id)?;
        let entries = engine.list_tree(&repo.main_branch_id).unwrap_or_default();
        let preferred = engine
            .state()
            .file_locations
            .get(&record.file_id)
            .and_then(|path| {
                entries.iter().find(|entry| {
                    entry.kind == "file"
                        && entry.file_id == record.file_id
                        && entry.lookup_key == xcs_naming::path_key(path)
                })
            });
        let download_name = preferred
            .or_else(|| {
                entries
                    .iter()
                    .find(|entry| entry.kind == "file" && entry.file_id == record.file_id)
            })
            .map(|entry| entry.name.clone())
            .or_else(|| engine.state().file_names.get(&record.file_id).cloned())
            .unwrap_or_else(|| record.creation_name.clone());
        drop(engine);
        let content = plan.open().map_err(error)?;
        Ok(Content {
            checksum_manifest: Some(content.manifest),
            fallback_files: content.fallback_files,
            file: content.file,
            size: content.size,
            hash: content.content_hash,
            lease: Box::new(content.lease),
            download_name: Some(download_name),
        })
    }
    fn workspace_content(
        &self,
        identity: &Identity,
        workspace: &str,
        item: &str,
    ) -> Result<Content, ApiError> {
        let engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        let workspace_record = workspace_authorize(&engine, identity, workspace)?;
        let plan = engine
            .prepare_workspace_content(workspace, item, &workspace_record.principal)
            .map_err(error)?;
        drop(engine);
        let opened = plan.open().map_err(error)?;
        Ok(Content {
            checksum_manifest: None,
            fallback_files: Vec::new(),
            file: opened.file,
            size: opened.size,
            hash: opened.content_hash,
            lease: Box::new(opened.lease),
            download_name: None,
        })
    }
    fn checksums(&self, identity: &Identity, version: &str) -> Result<Vec<u8>, ApiError> {
        let engine = self
            .engine
            .lock()
            .map_err(|_| ApiError::new(500, "INTERNAL_ERROR", "storage engine requires restart"))?;
        let record = engine
            .state()
            .versions
            .get(version)
            .ok_or_else(|| ApiError::new(404, "NOT_FOUND", "version not found"))?;
        authorize(
            identity,
            repo_for_history(&engine, &record.history_tree_id)?,
            false,
            false,
        )?;
        let plan = engine.prepare_content(version).map_err(error)?;
        drop(engine);
        plan.checksums().map_err(error)
    }
}
