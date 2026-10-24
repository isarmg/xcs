//! Audited engine-to-HTTP boundary policy, not an enumeration of every
//! internal error or a second error envelope. Unlisted codes use the default
//! 400/pass-through policy; *_NOT_FOUND retains the existing 404 convention.

pub(super) const ALIASES: &[(&str, &str)] = &[
    ("PERMISSION_DENIED", "FORBIDDEN"),
    ("UNAUTHENTICATED", "UNAUTHORIZED"),
    ("FILE_VERSION_CONFLICT", "FILE_CONFLICT"),
    ("BASE_HEAD_MISMATCH", "HEAD_CONFLICT"),
    ("FILE_BUSY", "FILE_WRITE_BUSY"),
    ("FENCING_TOKEN_INVALID", "FENCED"),
    ("FENCING_TOKEN_EXPIRED", "LEASE_EXPIRED"),
    ("RESOURCE_LIMIT", "RESOURCE_LIMIT_EXCEEDED"),
    ("ROUTE_INCOMPLETE", "BACKUP_ROUTE_INCOMPLETE"),
    ("IN_DOUBT", "COMMIT_IN_DOUBT"),
    ("GENERATION_MISMATCH", "CONFIG_CONFLICT"),
    ("STALE_UPLOAD_GENERATION", "UPLOAD_GENERATION_CONFLICT"),
];

// Public code, HTTP status. 500 entries also suppress internal diagnostics.
pub(super) const PUBLIC_STATUS: &[(&str, u16)] = &[
    ("UNAUTHORIZED", 401),
    ("FORBIDDEN", 403),
    ("CONTENT_PURGED", 410),
    ("NO_SPACE", 507),
    ("RESOURCE_LIMIT_EXCEEDED", 429),
    ("QUOTA_EXCEEDED", 429),
    ("CONTENT_UNAVAILABLE", 503),
    ("METADATA_UNAVAILABLE", 503),
    ("RECOVERY_REQUIRED", 503),
    ("REPLICA_TARGET_UNAVAILABLE", 503),
    ("ROUTE_TARGET_UNAVAILABLE", 503),
    ("COMMIT_IN_DOUBT", 503),
    ("BACKUP_ROUTE_INCOMPLETE", 503),
    ("COMMIT_PREPARATION_BUSY", 409),
    ("COMMIT_PREPARATION_STALE", 409),
    ("PREPARATION_STALE", 409),
    ("HEAD_CONFLICT", 409),
    ("BRANCH_NOT_ACTIVE", 409),
    ("FENCED", 409),
    ("LEASE_EXPIRED", 409),
    ("UPLOAD_GENERATION_CONFLICT", 409),
    ("FILE_CONFLICT", 409),
    ("FILE_BRANCH_CONFLICT", 409),
    ("DIRECTORY_BRANCH_CONFLICT", 409),
    ("DIRECTORY_ALIAS_CONFLICT", 409),
    ("BATCH_MAPPING_CONFLICT", 409),
    ("BATCH_SCOPE_CONFLICT", 409),
    ("IDEMPOTENCY_KEY_REUSED", 409),
    ("SPLIT_PLAN_STALE", 409),
    ("PURGE_BLOCKED_BY_REFERENCE", 409),
    ("CONTENT_PINNED", 409),
    ("CONFIG_CONFLICT", 409),
    ("BRANCH_WRITE_BUSY", 409),
    ("FILE_WRITE_BUSY", 409),
    ("NAMESPACE_CONFLICT", 409),
    ("NAME_COLLISION", 409),
    ("WORKSPACE_CLOSED", 409),
    ("PLAN_STALE", 409),
    ("PLAN_CLOSED", 409),
    ("TREE_MOVED", 409),
    ("RESTORE_BACKUP_CONFLICT", 409),
    ("DIRECTORY_BRANCH_INCOMPLETE", 409),
    ("DIRECTORY_SCOPE_REQUIRED", 409),
    ("DIRECTORY_HISTORY_SPLIT_UNSUPPORTED", 400),
    ("FILE_NODE_HISTORY_SPLIT_UNSUPPORTED", 400),
    ("IO_ERROR", 500),
    // Publication occurred; durability is unknown and I/O diagnostics stay private.
    ("DURABILITY_UNKNOWN", 500),
    ("STATE_INVALID", 500),
    ("INTERNAL_ERROR", 500),
    ("OBJECT_CORRUPT", 500),
    ("LOG_CORRUPT", 500),
    ("PUBLICATION_INVALID", 500),
    ("OPERATION_UNSUPPORTED", 501),
    ("PLAN_UNSUPPORTED", 501),
];

pub(super) fn public_code(internal: &str) -> &str {
    ALIASES
        .iter()
        .find(|(source, _)| *source == internal)
        .map_or(internal, |(_, public)| *public)
}
pub(super) fn status(code: &str) -> u16 {
    if code == "NOT_FOUND" || code.ends_with("_NOT_FOUND") {
        404
    } else {
        PUBLIC_STATUS
            .iter()
            .find(|(public, _)| *public == code)
            .map_or(400, |(_, status)| *status)
    }
}

#[cfg(test)]
#[path = "../../../../test/server/unit/engine_backend/error_registry.rs"]
mod tests;
