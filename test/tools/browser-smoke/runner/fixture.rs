//! Test-only persisted file fixture. No product upload/download CLI is required.
//! The caller owns a fresh private TempDir; never accept a user pool path.
use super::{Read, Result};
use storage_engine::{CommitRequest, Engine, new_id};

const PRINCIPAL: &str = "smoke-admin";
const FILE: &str = "browser-smoke.txt";
const PAYLOADS: [&str; 2] = ["first real version\n", "second real version: 测试\n"];

pub(super) fn seed_files(temporary: &tempfile::TempDir) -> Result<()> {
    let pool = temporary.path().join("pool");
    let mut engine = Engine::open(&pool)?;
    if !engine.state().repositories.is_empty() {
        return Err("browser fixture requires a fresh disposable pool with no repositories".into());
    }
    let repository =
        engine.create_repository("browser-fixtures", "P01", 3, PRINCIPAL, &new_id("op"))?;
    for payload in PAYLOADS {
        let workspace = engine.create_workspace(&repository.main_branch_id, PRINCIPAL)?;
        engine.stage_bytes(&workspace.workspace_id, FILE, payload.as_bytes())?;
        engine.commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: PRINCIPAL.into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "Browser metadata fixture".into(),
            request_digest: None,
        })?;
    }
    // Reopen from disk before serving: an in-memory-only fixture is insufficient.
    drop(engine);
    let engine = Engine::open(&pool)?;
    let entries = engine.list_tree(&repository.main_branch_id)?;
    let [entry] = entries.as_slice() else {
        return Err("fixture must persist exactly one file".into());
    };
    if entry.path != FILE || engine.file_history(&entry.file_id)?.len() != 2 {
        return Err("fixture file path or two-version history did not survive reopen".into());
    }
    let mut content = engine.open_content(&entry.version_id)?;
    let mut bytes = Vec::new();
    content.file.read_to_end(&mut bytes)?;
    if bytes != PAYLOADS[1].as_bytes() {
        return Err("fixture current bytes did not survive reopen".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "../unit/fixture_tests.rs"]
mod tests;
