//! Operation ownership and current repository access must precede health I/O.
use super::{RepositoryAccess, authorize_access, repo_for_history};
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
        authorize_access(
            identity,
            repo_for_history(engine, history)?,
            RepositoryAccess::Read,
        )
    } else {
        identity.require_admin()
    }
}

#[cfg(test)]
#[path = "../../../../test/server/unit/engine_backend/operations/tests.rs"]
mod tests;
