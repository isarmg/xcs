//! Engine errors, request fields, and public response projections.
use crate::ApiError;
use serde::Serialize;
use serde_json::{Value, json};
use storage_engine::Engine;

pub(super) fn error(error: storage_engine::Error) -> ApiError {
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
            "COMMIT_PREPARATION_BUSY"
            | "COMMIT_PREPARATION_STALE"
            | "PREPARATION_STALE"
            | "HEAD_CONFLICT"
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
        let mut mapped = ApiError::new(status, code, error.message.clone());
        mapped.retryable |= error.retryable;
        mapped
    }
}

pub(super) fn encode(value: impl Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value).map_err(ApiError::invalid)
}
/// Response ownership is independent from the version currently visible in a
/// branch. An unwritten branch reads its parent's immutable version.
pub(super) fn content_wire(engine: &Engine, visible: Option<&str>, source: Option<&str>) -> Value {
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
pub(super) fn branch_wire(
    engine: &Engine,
    branch: &storage_contract::Branch,
) -> Result<Value, ApiError> {
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
pub(super) fn pending_entry_source<'a>(
    engine: &'a Engine,
    branch: &str,
    commit: &'a storage_contract::Commit,
    entry: &storage_contract::Entry,
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
            && let Some(context) = engine.file_context(&repo.repository_id, &entry.path)
        {
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

pub(super) fn current_entry_source<'a>(
    engine: &'a Engine,
    branch: &str,
    entry: &storage_contract::Entry,
) -> Option<&'a str> {
    let current = engine.state().branches.get(branch)?;
    let commit = engine.state().commits.get(&current.head_commit_id)?;
    pending_entry_source(engine, branch, commit, entry)
}
pub(super) fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::invalid(format!("missing string field {key}")))
}
pub(super) fn operation(input: &Value) -> Result<&str, ApiError> {
    field(input, "operation_id")
}
pub(super) fn keys(input: &Value, allowed: &[&str]) -> Result<(), ApiError> {
    let object = input
        .as_object()
        .ok_or_else(|| ApiError::invalid("expected object"))?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(ApiError::invalid(format!("unknown field {key}")));
    }
    Ok(())
}
pub(super) fn sanitized<T: Serialize>(value: T) -> Result<Value, ApiError> {
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
pub(super) fn operation_wire(
    _engine: &Engine,
    op: storage_engine::OperationResult,
) -> Result<Value, ApiError> {
    let health = match op.current_health.as_str() {
        "HEALTHY" | "DEGRADED" | "UNAVAILABLE" | "UNKNOWN" => op.current_health.as_str(),
        _ => "UNKNOWN",
    };
    let current_health = json!({"state":health,"checked_at":time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).map_err(ApiError::invalid)?});
    let content_save = op.kind == "workspace.commit"
        && op.protection_at_publish != storage_contract::ProtectionAtPublish::NotApplicable;
    if content_save {
        let complete = matches!(
            op.commit_state,
            storage_contract::CommitState::Published
                | storage_contract::CommitState::Acked
                | storage_contract::CommitState::PublishedDegraded
        );
        let state = if complete {
            "completed"
        } else if matches!(
            op.commit_state,
            storage_contract::CommitState::InDoubt
                | storage_contract::CommitState::RecoveryRequired
        ) {
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
            json!({"outcome":if op.protection_at_publish==storage_contract::ProtectionAtPublish::TargetMet{"COMMITTED_AND_PROTECTED"}else{"COMMITTED_DEGRADED"},"commit_id":op.commit_id,"branch_id":op.branch_id,"branch_generation":field(&op.result,"branch_generation")?,"publication_sequence":op.sequence.to_string(),"membership_epoch":field(&op.result,"membership_epoch")?,"protection_at_publish":op.protection_at_publish,"versions":versions})
        } else {
            Value::Null
        };
        Ok(
            json!({"operation_id":op.operation_id,"operation_kind":"content_save","request_digest":op.request_digest,"state":state,"commit_state":op.commit_state,"result":result,"error":null,"current_health":current_health}),
        )
    } else {
        Ok(
            json!({"operation_id":op.operation_id,"operation_kind":op.kind,"request_digest":op.request_digest,"state":if matches!(op.commit_state,storage_contract::CommitState::Published|storage_contract::CommitState::Acked){"completed"}else{"in_doubt"},"commit_state":op.commit_state,"protection_at_publish":op.protection_at_publish,"publication_sequence":op.sequence.to_string(),"result":op.result,"error":null,"current_health":current_health}),
        )
    }
}
