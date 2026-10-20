//! Frozen operation receipts and request-scoped, leased health observations.
//! Neither a receipt nor a read plan is an authorization grant. Network callers
//! must authorize the receipt before capturing health and recheck after slow I/O.
use super::{ContentReadPlan, Engine};
use crate::{journal::*, publication_evidence::verified_object, *};
use std::{collections::BTreeSet, sync::Arc};

/// Only the selected operation and its owner, never a clone of the whole State.
/// The publication facts remain unchanged when current content health changes.
pub struct OperationRecord {
    owner: Option<String>,
    result: OperationResult,
}

impl OperationRecord {
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    pub fn result(&self) -> &OperationResult {
        &self.result
    }

    pub fn into_result(self) -> OperationResult {
        self.result
    }
}

/// Owns one immutable descriptor and read lease per distinct committed version.
/// Configuration is shared across versions; no payload or proof is read during
/// capture. Drop cancels the observation and releases all leases.
pub struct OperationReadPlan {
    record: OperationRecord,
    versions: Vec<(ContentReadPlan, u32)>,
    unavailable: bool,
}

impl OperationReadPlan {
    /// All disk identity, publication proof and complete payload verification
    /// happens here, after the caller has released its Engine mutex.
    pub fn check(mut self) -> OperationRecord {
        let mut degraded = false;
        for (plan, target_copies) in &self.versions {
            match plan.health(*target_copies) {
                "UNAVAILABLE" => self.unavailable = true,
                "DEGRADED" => degraded = true,
                _ => (),
            }
        }
        if self.unavailable {
            self.record.result.current_health = "UNAVAILABLE".into();
        } else if degraded {
            self.record.result.current_health = "DEGRADED".into();
        }
        self.record
    }
}

impl Engine {
    pub fn operation_owner(&self, id: &str) -> Option<String> {
        if let Some(owner) = self.state.operation_principals.get(id) {
            return Some(owner.clone());
        }
        let (members, snapshot, _) = self.pending_operation_snapshot(id).ok()?;
        snapshot_entry(&members, &snapshot, "operation_principals", id).ok()?
    }

    /// Published records are metadata-only. An uncertain decision reads only
    /// the hash-bound owner/operation pages, not the candidate's entire State.
    pub fn operation_record(&self, id: &str) -> Result<OperationRecord> {
        if let Some(result) = self.state.operations.get(id) {
            return Ok(OperationRecord {
                owner: self.state.operation_principals.get(id).cloned(),
                result: result.clone(),
            });
        }
        let (members, snapshot, candidate) = self.pending_operation_snapshot(id)?;
        let owner = snapshot_entry(&members, &snapshot, "operation_principals", id)?;
        let mut result: OperationResult = snapshot_entry(&members, &snapshot, "operations", id)?
            .ok_or_else(operation_not_found)?;
        if result.operation_id != id
            || result.request_digest != candidate.request_digest
            || result.sequence != candidate.sequence
        {
            return Err(Error::new(
                "OBJECT_CORRUPT",
                "operation differs from accepted candidate",
            ));
        }
        result.commit_state = storage_contract::CommitState::InDoubt;
        result.protection_at_publish = storage_contract::ProtectionAtPublish::Unconfirmed;
        result.current_health = "RECOVERING".into();
        Ok(OperationRecord { owner, result })
    }

    /// Capture this immediately after authorizing `record`, in the same Engine
    /// critical section. The record is response data, not reusable admission.
    pub fn prepare_operation_health(&self, record: OperationRecord) -> OperationReadPlan {
        let mut plan = OperationReadPlan {
            record,
            versions: Vec::new(),
            unavailable: false,
        };
        // A pending candidate has no publication to inspect, even if its staged
        // commit happens to resemble an existing public commit.
        if plan.record.result.commit_state == storage_contract::CommitState::InDoubt {
            return plan;
        }
        let Some(commit) = plan
            .record
            .result
            .commit_id
            .as_ref()
            .and_then(|id| self.state.commits.get(id))
        else {
            return plan;
        };
        let config = Arc::new(self.config.clone());
        let mut selected = BTreeSet::new();
        for entry in commit.entries.iter().filter(|entry| entry.kind == "file") {
            if !selected.insert(&entry.version_id) {
                continue;
            }
            let prepared = self.prepare_content_with_config(&entry.version_id, Arc::clone(&config));
            match prepared {
                Ok(content) => {
                    let target = self.state.placements[&entry.version_id].target_copies;
                    plan.versions.push((content, target));
                }
                // Purged/migrated, missing or inconsistent descriptors all fail
                // closed, without rewriting the original publication receipt.
                Err(_) => plan.unavailable = true,
            }
        }
        plan
    }

    /// Synchronous convenience for trusted local callers. Shared backends must
    /// capture and execute the read plan separately to release their mutex.
    pub fn operation(&self, id: &str) -> Result<OperationResult> {
        Ok(self
            .prepare_operation_health(self.operation_record(id)?)
            .check()
            .into_result())
    }

    /// Only accepted, hash-bound decisions can supply an unpublished owner.
    /// Current format omits owner/history from Candidate, so this exceptional
    /// path must read the journal, candidate and selected snapshot pages. It
    /// never traverses version/commit/authentication pages or payload objects.
    fn pending_operation_snapshot(
        &self,
        id: &str,
    ) -> Result<(Vec<DiskConfig>, Snapshot, Candidate)> {
        if !self.blocked {
            return Err(operation_not_found());
        }
        let members = self.trusted_evidence_members();
        for disk in &members {
            if let Ok(scan) = scan(disk) {
                for (record, _) in scan.records.iter().rev().filter(|(record, _)| {
                    matches!(record.record_type.as_str(), "ACCEPT" | "PUBLICATION")
                }) {
                    if let Ok(candidate) =
                        verified_object::<Candidate>(&members, "candidate", &record.candidate_hash)
                        && candidate.operation_id == id
                        && candidate.pool_id == self.config.pool_id
                        && candidate.sequence == record.sequence
                    {
                        let snapshot: Snapshot =
                            verified_object(&members, "snapshot", &candidate.snapshot_hash)?;
                        if snapshot.sequence != candidate.sequence
                            || snapshot
                                .scalars
                                .get("sequence")
                                .and_then(serde_json::Value::as_str)
                                != Some(candidate.sequence.to_string().as_str())
                        {
                            return Err(Error::new("OBJECT_CORRUPT", "snapshot sequence differs"));
                        }
                        return Ok((members, snapshot, candidate));
                    }
                }
            }
        }
        Err(operation_not_found())
    }
}

fn operation_not_found() -> Error {
    Error::new("OPERATION_NOT_FOUND", "operation is not registered")
}

// Verify exact bounded pages one at a time. Do not materialize unrelated maps,
// or trust an unverified index/cache as a substitute for the accepted root.
fn snapshot_entry<T: serde::Serialize + serde::de::DeserializeOwned>(
    members: &[DiskConfig],
    snapshot: &Snapshot,
    field: &str,
    id: &str,
) -> Result<Option<T>> {
    if snapshot.scalars.contains_key(field) {
        return Err(Error::new(
            "OBJECT_CORRUPT",
            "operation snapshot field is not paged",
        ));
    }
    let hashes = snapshot
        .pages
        .get(field)
        .ok_or_else(|| Error::new("OBJECT_CORRUPT", "operation snapshot field is missing"))?;
    let mut found = None;
    for hash in hashes {
        let mut page: std::collections::BTreeMap<String, T> =
            verified_object(members, "state-page", hash)?;
        if page.len() > STATE_PAGE_ENTRIES {
            return Err(Error::new(
                "OBJECT_CORRUPT",
                "operation snapshot page exceeds entry limit",
            ));
        }
        if let Some(value) = page.remove(id)
            && found.replace(value).is_some()
        {
            return Err(Error::new(
                "OBJECT_CORRUPT",
                "duplicate operation snapshot entity",
            ));
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::version_dir;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, Engine, Repository) {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let repo = engine
            .create_repository("Health", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        (root, engine, repo)
    }

    fn save(engine: &mut Engine, branch: &str, bytes: &[u8]) -> OperationResult {
        let workspace = engine.create_workspace(branch, "owner").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "data.txt", bytes)
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "owner".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: String::new(),
                request_digest: None,
            })
            .unwrap()
    }

    fn assert_frozen(observed: &OperationResult, published: &OperationResult) {
        let mut normalized = observed.clone();
        normalized.current_health = published.current_health.clone();
        assert_eq!(&normalized, published);
    }

    #[test]
    fn operation_read_plan_blocks_purge_until_observation_finishes() {
        let (_root, mut engine, repo) = fixture();
        let original = save(&mut engine, &repo.main_branch_id, b"old payload");
        let version = engine.list_tree(&repo.main_branch_id).unwrap()[0]
            .version_id
            .clone();
        save(&mut engine, &repo.main_branch_id, b"new payload");
        let purge = engine
            .create_purge_plan(&repo.history_tree_id, vec![version.clone()], "owner")
            .unwrap();
        let plan = engine
            .prepare_operation_health(engine.operation_record(&original.operation_id).unwrap());
        assert_eq!(engine.reads.lock().unwrap().get(&version), Some(&1));
        assert_eq!(
            engine
                .commit_plan(&purge.plan_id, "owner", &new_id("op"))
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        let observed = plan.check().into_result();
        assert_eq!(observed.current_health, "HEALTHY");
        assert_frozen(&observed, &original);
        assert!(!engine.reads.lock().unwrap().contains_key(&version));
        engine
            .commit_plan(&purge.plan_id, "owner", &new_id("op"))
            .unwrap();
        let observed = engine.operation(&original.operation_id).unwrap();
        assert_eq!(observed.current_health, "UNAVAILABLE");
        assert_frozen(&observed, &original);
    }

    #[test]
    fn operation_capture_is_metadata_only_deduplicated_and_cancel_safe() {
        let (_root, mut engine, repo) = fixture();
        let operation = save(&mut engine, &repo.main_branch_id, b"content");
        let commit = engine
            .state
            .commits
            .get_mut(operation.commit_id.as_ref().unwrap())
            .unwrap();
        let entry = commit.entries[0].clone();
        // One immutable version may be referenced more than once. It must not
        // cause repeated full hashing or one lease per directory appearance.
        commit.entries.push(entry.clone());
        for disk in &engine.config.disks {
            fs::remove_dir_all(disk.root.join("histories")).unwrap();
        }
        SNAPSHOT_LOAD_COUNT.with(|count| count.set(0));
        let plan = engine
            .prepare_operation_health(engine.operation_record(&operation.operation_id).unwrap());
        assert_eq!(plan.versions.len(), 1);
        assert_eq!(
            engine.reads.lock().unwrap().get(&entry.version_id),
            Some(&1)
        );
        SNAPSHOT_LOAD_COUNT.with(|count| assert_eq!(count.get(), 0));
        drop(plan);
        assert!(engine.reads.lock().unwrap().is_empty());
        let observed = engine.operation(&operation.operation_id).unwrap();
        assert_eq!(observed.current_health, "UNAVAILABLE");
        assert_frozen(&observed, &operation);
    }

    #[test]
    fn health_reverifies_each_replica_and_never_rewrites_publication_facts() {
        let (_root, mut engine, repo) = fixture();
        let operation = save(&mut engine, &repo.main_branch_id, b"content");
        let version_id = engine.list_tree(&repo.main_branch_id).unwrap()[0]
            .version_id
            .clone();
        let version = &engine.state.versions[&version_id];
        let directories: Vec<_> = engine
            .config
            .disks
            .iter()
            .map(|disk| version_dir(disk, &version.history_tree_id, &version_id))
            .collect();
        let plan = engine
            .prepare_operation_health(engine.operation_record(&operation.operation_id).unwrap());
        // Corrupt the complete payload after capture, keeping its length. The
        // observation must actually hash bytes, not trust a prior good result.
        let path = crate::container_storage::content_path(&directories[0], version).unwrap();
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"changed").unwrap();
        let observed = plan.check().into_result();
        assert_eq!(observed.current_health, "DEGRADED");
        assert_frozen(&observed, &operation);
        // A checksum header without its bound hash and an unrelated publication
        // proof are not sufficient health evidence, even if a body is present.
        fs::write(directories[1].join("checksums.bin"), b"untrusted").unwrap();
        fs::write(directories[2].join("publication-proof.json"), b"{}").unwrap();
        let observed = engine.operation(&operation.operation_id).unwrap();
        assert_eq!(observed.current_health, "UNAVAILABLE");
        assert_frozen(&observed, &operation);
        assert_eq!(engine.state.operations[&operation.operation_id], operation);
        assert!(engine.reads.lock().unwrap().is_empty());
    }

    #[test]
    fn pending_receipt_reads_only_exact_operation_pages_and_owner_first() {
        let (_root, mut engine, _repo) = fixture();
        let operation = new_id("op");
        engine.inject_fault_once("after_accept_1");
        assert!(
            engine
                .create_repository("pending", "P01", 3, "owner", &operation)
                .is_err()
        );
        let (members, snapshot, _) = engine.pending_operation_snapshot(&operation).unwrap();
        for field in ["repositories", "commits"] {
            for hash in &snapshot.pages[field] {
                for disk in &members {
                    fs::write(
                        object_path(disk, "state-page", hash),
                        b"unrelated broken page",
                    )
                    .unwrap();
                }
            }
        }
        SNAPSHOT_LOAD_COUNT.with(|count| count.set(0));
        let record = engine.operation_record(&operation).unwrap();
        assert_eq!(record.owner(), Some("owner"));
        assert_eq!(
            record.result().commit_state,
            storage_contract::CommitState::InDoubt
        );
        assert_eq!(
            record.result().protection_at_publish,
            storage_contract::ProtectionAtPublish::Unconfirmed
        );
        assert_eq!(record.result().current_health, "RECOVERING");
        let plan = engine.prepare_operation_health(record);
        assert!(plan.versions.is_empty());
        assert!(engine.reads.lock().unwrap().is_empty());
        SNAPSHOT_LOAD_COUNT.with(|count| assert_eq!(count.get(), 0));
        // Ownership lookup does not load potentially large operation results.
        for hash in &snapshot.pages["operations"] {
            for disk in &members {
                fs::write(object_path(disk, "state-page", hash), b"broken results").unwrap();
            }
        }
        assert_eq!(engine.operation_owner(&operation).as_deref(), Some("owner"));
        assert!(engine.operation_record(&operation).is_err());
    }
}
