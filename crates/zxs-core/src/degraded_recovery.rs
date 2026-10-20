//! Explicit loss acknowledgement for an already decided, unpublished candidate.
//! This path never authorizes a new business write with fewer required copies.
use crate::{disk::*, journal::*, *};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryPlan {
    pub recovery_id: String,
    pub digest: String,
    pub operation_id: String,
    pub candidate_hash: String,
    pub principal: String,
    pub reason: String,
    /// A unique fence persisted by every current member while all original OS locks are held.
    pub isolation_generation: String,
    #[serde(with = "zxs_contract::u64_string")]
    pub sequence: u64,
    pub version_protection: Vec<Value>,
    pub loss_details: Vec<Value>,
    members: Vec<MemberWitness>,
    original_accepts: Vec<Receipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberWitness {
    disk_id: String,
    slot_id: String,
    #[serde(with = "zxs_contract::u64_string")]
    binding_generation: u64,
    #[serde(with = "zxs_contract::u64_string")]
    log_bytes: u64,
    last_frame_hash: String,
    records: Vec<Receipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryProof {
    pub plan: RecoveryPlan,
    pub reprotect: Vec<Receipt>,
}
fn required(message: impl Into<String>) -> Error {
    Error::new("RECOVERY_REQUIRED", message)
}
fn plan_digest(plan: &RecoveryPlan) -> Result<String> {
    let mut plan = plan.clone();
    plan.digest.clear();
    object_hash("recovery-plan-digest", &plan)
}
fn receipts_ids(receipts: &[Receipt]) -> BTreeSet<String> {
    receipts.iter().map(|r| r.disk_id.clone()).collect()
}
fn plan_path(root: &std::path::Path, id: &str) -> std::path::PathBuf {
    root.join("recovery-plans").join(format!("{id}.json"))
}
fn member_plan_path(disk: &DiskConfig, hash: &str) -> std::path::PathBuf {
    disk.root
        .join("metadata/recovery-plans")
        .join(format!("{hash}.json"))
}
fn observe(engine: &Engine, candidate: &Candidate) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for evidence in &candidate.versions {
        let mut slots = Vec::new();
        for slot in &evidence.placement.route_slot_ids {
            let disk = engine.disk(slot)?;
            validate_seal(&engine.config, disk)?;
            if evidence.placement.binding_generations.get(slot) != Some(&disk.binding_generation) {
                return Err(required("candidate payload binding generation differs"));
            }
            let dir = version_dir(
                disk,
                &evidence.version.history_tree_id,
                &evidence.version.version_id,
            );
            let state = match verify_payload(&dir, &evidence.version) {
                Ok(()) => "COMPLETE_VERIFIED",
                Err(error) => {
                    // An inaccessible device or an arbitrary I/O failure is not permanent loss.
                    let content = crate::container_storage::content_path(&dir, &evidence.version)?;
                    match fs::symlink_metadata(&content) {
                        Err(io) if io.kind() == std::io::ErrorKind::NotFound => "MISSING",
                        Err(_) => {
                            return Err(required(format!(
                                "cannot classify payload slot {slot}: {error}"
                            )));
                        }
                        Ok(meta) if meta.is_file() => "CORRUPT",
                        Ok(_) => {
                            return Err(required(
                                "untrusted payload path requires manual quarantine",
                            ));
                        }
                    }
                }
            };
            slots.push(json!({"slot_id":slot,"copy_level":disk.level,"disk_id":disk.disk_id,"binding_generation":disk.binding_generation.to_string(),"state":state}));
        }
        out.push(json!({"version_id":evidence.version.version_id,"primary_slot_id":evidence.placement.primary_slot_id,"target_copies":evidence.placement.target_copies,"required_slots":evidence.placement.route_slot_ids,"topology_generation":evidence.placement.topology_generation.to_string(),"slots":slots}));
    }
    Ok(out)
}
fn freeze_observation(mut observed: Vec<Value>) -> (Vec<Value>, Vec<Value>) {
    let mut details = Vec::new();
    for version in &mut observed {
        let id = version["version_id"].clone();
        for slot in version["slots"].as_array_mut().expect("observed slots") {
            if slot["state"] != "COMPLETE_VERIFIED" {
                details
                    .push(json!({"version_id":id,"slot_id":slot["slot_id"],"cause":slot["state"]}));
                slot["state"] = json!("UNAVAILABLE");
            }
        }
    }
    (observed, details)
}
pub(crate) fn discover_publication(
    engine: &Engine,
    candidate: &Candidate,
    hash: &str,
) -> Result<Option<Publication>> {
    let mut found: Option<Publication> = None;
    for disk in engine.members() {
        let directory = disk.root.join("metadata/objects/publication");
        if directory.exists() {
            for entry in fs::read_dir(directory)? {
                let path = entry?.path();
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| required("invalid publication object name"))?;
                let publication: Publication =
                    load_object(&disk, "publication", name).map_err(|_| {
                        required("an unreadable publication object prevents proving absence")
                    })?;
                if publication.candidate_hash == hash {
                    if found.as_ref().is_some_and(|old| {
                        object_hash("publication", old).ok()
                            != object_hash("publication", &publication).ok()
                    }) {
                        return Err(required("conflicting publication results exist"));
                    }
                    found = Some(publication);
                }
            }
        }
    }
    for evidence in &candidate.versions {
        for slot in &evidence.placement.route_slot_ids {
            let directory = version_dir(
                engine.disk(slot)?,
                &evidence.version.history_tree_id,
                &evidence.version.version_id,
            );
            let path = local_proof_directory(&directory, hash)?.join("publication-proof.json");
            if path.exists() {
                let local: LocalPublication = read_json(&path)
                    .map_err(|_| required("unreadable local publication evidence"))?;
                if object_hash("candidate", &local.candidate)? != hash {
                    return Err(required("conflicting local publication evidence"));
                }
                if found.as_ref().is_some_and(|old| {
                    object_hash("publication", old).ok()
                        != object_hash("publication", &local.publication).ok()
                }) {
                    return Err(required("local and metadata publication results conflict"));
                }
                found = Some(local.publication);
            }
        }
    }
    Ok(found)
}
fn exclude_publication(engine: &Engine, candidate: &Candidate, hash: &str) -> Result<()> {
    if discover_publication(engine, candidate, hash)?.is_some() {
        return Err(required(
            "original publication evidence exists; replay that result",
        ));
    }
    Ok(())
}
fn require_permanent_loss(engine: &Engine, candidate: &Candidate, health: &[Value]) -> Result<()> {
    let mut unavailable = false;
    for version in &candidate.versions {
        let state = health
            .iter()
            .find(|v| v["version_id"] == version.version.version_id)
            .ok_or_else(|| required("health coverage missing"))?;
        let slots = state["slots"]
            .as_array()
            .ok_or_else(|| required("health slots missing"))?;
        if slots.iter().all(|s| s["state"] == "COMPLETE_VERIFIED") {
            continue;
        }
        // Use exactly the same source search as normal repair, including old
        // workspace generations and registered repair quarantine directories.
        if engine
            .pending_payload_source(&version.version, &version.placement)?
            .is_some()
        {
            return Err(required(
                "verified full bytes survive; repair the original protection instead of acknowledging permanent loss",
            ));
        }
        unavailable = true;
    }
    if !unavailable {
        return Err(required("no permanent content loss was established"));
    }
    Ok(())
}
impl RecoveryPlan {
    pub(crate) fn accepts(&self) -> &[Receipt] {
        &self.original_accepts
    }
}
impl RecoveryProof {
    pub(crate) fn verify(&self, c: &Candidate, p: &Publication) -> Result<()> {
        let bad = || {
            Error::new(
                "PUBLICATION_INVALID",
                "degraded publication recovery evidence is invalid",
            )
        };
        let plan = &self.plan;
        let candidate = object_hash("candidate", c)?;
        let ids = c.metadata_disk_ids.iter().cloned().collect::<BTreeSet<_>>();
        if plan.digest != plan_digest(plan)?
            || plan.candidate_hash != candidate
            || plan.sequence != c.sequence
            || plan.operation_id != c.operation_id
            || plan.reason.trim().is_empty()
            || plan.principal.is_empty()
            || plan.members.len() != 3
            || plan.original_accepts.len() < 2
            || plan.original_accepts.len() > 3
            || receipts_ids(&plan.original_accepts).len() != plan.original_accepts.len()
            || !receipts_ids(&plan.original_accepts).is_subset(&ids)
            || self.reprotect.len() != 3
            || receipts_ids(&self.reprotect) != ids
            || p.finalize.recovery_id.as_deref() != Some(plan.recovery_id.as_str())
            || canonical_json(&p.finalize.accepts)? != canonical_json(&plan.original_accepts)?
            || p.result.commit_state != "PUBLISHED_DEGRADED"
            || p.result.protection_at_publish != "BELOW_TARGET"
            || p.result.result["recovery_id"].as_str() != Some(plan.recovery_id.as_str())
            || p.result.result["recovery_plan_digest"].as_str() != Some(plan.digest.as_str())
            || canonical_json(&p.result.result["version_protection"])?
                != canonical_json(&plan.version_protection)?
        {
            return Err(bad());
        }
        validate_id(&plan.recovery_id, "recovery")?;
        validate_id(&plan.isolation_generation, "fence")?;
        let plan_hash = object_hash("recovery-plan", plan)?;
        let mut members = BTreeSet::new();
        for member in &plan.members {
            if !members.insert(member.disk_id.clone()) || member.records.is_empty() {
                return Err(bad());
            }
            let mut previous = member.records[0].record.previous_frame_hash.clone();
            for record in &member.records {
                if record.disk_id != member.disk_id
                    || record.slot_id != member.slot_id
                    || record.record.record_type == "PUBLICATION"
                    || record.record.record_type == "REPROTECT"
                    || record.record.previous_frame_hash != previous
                    || !receipt_binds(
                        record,
                        &record.record.record_type,
                        c.sequence,
                        &candidate,
                        &record.record.object_hash,
                    )?
                {
                    return Err(bad());
                }
                previous = record.frame_hash.clone();
            }
            if previous != member.last_frame_hash {
                return Err(bad());
            }
            let fence = self
                .reprotect
                .iter()
                .find(|r| r.disk_id == member.disk_id)
                .ok_or_else(bad)?;
            if fence.slot_id != member.slot_id
                || fence.record.previous_frame_hash != member.last_frame_hash
                || !receipt_binds(fence, "REPROTECT", c.sequence, &candidate, &plan_hash)?
            {
                return Err(bad());
            }
        }
        if members != ids {
            return Err(bad());
        }
        for accept in &plan.original_accepts {
            if !receipt_binds(accept, "ACCEPT", c.sequence, &candidate, &candidate)?
                || !plan
                    .members
                    .iter()
                    .any(|m| m.records.iter().any(|r| r.frame_hash == accept.frame_hash))
            {
                return Err(bad());
            }
        }
        if plan.version_protection.len() != c.versions.len() {
            return Err(bad());
        }
        let mut expected = BTreeSet::new();
        let mut below = false;
        for version in &c.versions {
            let observation = plan
                .version_protection
                .iter()
                .find(|v| v["version_id"].as_str() == Some(version.version.version_id.as_str()))
                .ok_or_else(bad)?;
            let slots = observation["slots"].as_array().ok_or_else(bad)?;
            if slots.len() != version.placement.route_slot_ids.len()
                || observation["required_slots"] != json!(version.placement.route_slot_ids)
                || observation["target_copies"] != json!(version.placement.target_copies)
                || observation["primary_slot_id"] != json!(version.placement.primary_slot_id)
            {
                return Err(bad());
            }
            let mut slot_set = BTreeSet::new();
            for slot in slots {
                let id = slot["slot_id"].as_str().ok_or_else(bad)?;
                if !slot_set.insert(id) || !version.placement.route_slot_ids.iter().any(|s| s == id)
                {
                    return Err(bad());
                }
                match slot["state"].as_str() {
                    Some("COMPLETE_VERIFIED") => {
                        expected.insert((version.version.version_id.clone(), id.to_string()));
                    }
                    Some("UNAVAILABLE") => below = true,
                    _ => return Err(bad()),
                }
            }
        }
        let actual = p
            .payload_receipts
            .iter()
            .map(|r| (r.version_id.clone(), r.slot_id.clone()))
            .collect::<BTreeSet<_>>();
        if !below || actual != expected || actual.len() != p.payload_receipts.len() {
            return Err(bad());
        }
        Ok(())
    }
}
impl Engine {
    fn pending_recovery_candidate(
        &self,
        operation: &str,
        resume: Option<&RecoveryPlan>,
    ) -> Result<(Candidate, State, Vec<MemberWitness>, Vec<Receipt>)> {
        self.require_recovery_locks()?;
        let mut candidate = None;
        let mut witnesses = Vec::new();
        let mut accepts = Vec::new();
        for disk in self.members() {
            let scanned = scan(&disk)?;
            if scanned.torn_tail {
                return Err(required("a torn log cannot exclude an earlier publication"));
            }
            let mut found = None;
            for (record, _) in &scanned.records {
                let c: Candidate = load_object(&disk, "candidate", &record.candidate_hash)?;
                if c.operation_id == operation {
                    found = Some(c);
                }
            }
            let c = found.ok_or_else(|| {
                required("candidate must be present in all three original member logs")
            })?;
            let hash = object_hash("candidate", &c)?;
            if candidate.as_ref().is_some_and(|existing: &Candidate| {
                object_hash("candidate", existing).ok().as_deref() != Some(hash.as_str())
            }) {
                return Err(required("original members disagree about the candidate"));
            }
            if c.previous_snapshot != self.snapshot || c.pool_id != self.config.pool_id {
                return Err(required(
                    "candidate does not extend the applied publication prefix",
                ));
            }
            let tail = scanned
                .records
                .last()
                .ok_or_else(|| required("member journal is empty"))?;
            if tail.0.sequence != c.sequence {
                return Err(required("recovery candidate is not the last journal slot"));
            }
            let mut records = scanned
                .records
                .iter()
                .filter(|(r, _)| r.sequence == c.sequence)
                .map(|(record, frame_hash)| Receipt {
                    disk_id: disk.disk_id.clone(),
                    slot_id: disk.slot_id.clone(),
                    frame_hash: frame_hash.clone(),
                    record: record.clone(),
                })
                .collect::<Vec<_>>();
            if let Some(plan) = resume {
                let expected = plan
                    .members
                    .iter()
                    .find(|m| m.disk_id == disk.disk_id)
                    .ok_or_else(|| required("recovery member missing"))?;
                if records.len() < expected.records.len()
                    || canonical_json(&records[..expected.records.len()])?
                        != canonical_json(&expected.records)?
                {
                    return Err(required("original recovery log prefix differs"));
                }
                let plan_hash = object_hash("recovery-plan", plan)?;
                for record in &records[expected.records.len()..] {
                    let valid = match record.record.record_type.as_str() {
                        "REPROTECT" => record.record.object_hash == plan_hash,
                        "FINALIZE" => {
                            load_object::<Finalize>(&disk, "finalize", &record.record.object_hash)?
                                .recovery_id
                                .as_deref()
                                == Some(plan.recovery_id.as_str())
                        }
                        _ => false,
                    };
                    if !valid || record.record.candidate_hash != hash {
                        return Err(required(
                            "unexpected evidence appeared after recovery preview",
                        ));
                    }
                }
                records = expected.records.clone();
            }
            if records.iter().any(|r| {
                r.record.candidate_hash != hash
                    || r.record.record_type == "PUBLICATION"
                    || r.record.record_type == "REPROTECT"
            }) {
                return Err(required(
                    "publication or another recovery must be reconciled first",
                ));
            }
            for receipt in records.iter().filter(|r| r.record.record_type == "ACCEPT") {
                if !receipt_binds(receipt, "ACCEPT", c.sequence, &hash, &hash)? {
                    return Err(required(
                        "original ACCEPT does not bind the fixed candidate",
                    ));
                }
                accepts.push(receipt.clone());
            }
            if let Some(plan) = resume {
                witnesses.push(
                    plan.members
                        .iter()
                        .find(|m| m.disk_id == disk.disk_id)
                        .expect("member checked")
                        .clone(),
                );
            } else {
                witnesses.push(MemberWitness {
                    disk_id: disk.disk_id.clone(),
                    slot_id: disk.slot_id,
                    binding_generation: disk.binding_generation,
                    log_bytes: scanned.valid_bytes,
                    last_frame_hash: tail.1.clone(),
                    records,
                });
            }
            candidate = Some(c);
        }
        if accepts.len() < 2 || receipts_ids(&accepts).len() != accepts.len() {
            return Err(required(
                "at least two original durable ACCEPT receipts are required",
            ));
        }
        let candidate = candidate.ok_or_else(|| required("candidate missing"))?;
        let hash = object_hash("candidate", &candidate)?;
        exclude_publication(self, &candidate, &hash)?;
        let state = self.load_bound_snapshot(&candidate.snapshot_hash)?;
        Ok((candidate, state, witnesses, accepts))
    }
    pub fn prepare_degraded_recovery(
        &mut self,
        operation: &str,
        principal: &str,
        reason: &str,
    ) -> Result<RecoveryPlan> {
        if principal.is_empty() || reason.trim().is_empty() || reason.len() > 4096 {
            return Err(Error::new(
                "INVALID_REQUEST",
                "recovery requires a principal and a bounded explicit permanent-loss reason",
            ));
        }
        let (candidate, _, members, original_accepts) =
            self.pending_recovery_candidate(operation, None)?;
        let hash = object_hash("candidate", &candidate)?;
        let old = member_plan_path(&self.members()[0], &hash);
        if old.exists() {
            let plan: RecoveryPlan = read_json(&old)?;
            if plan.principal != principal || plan.digest != plan_digest(&plan)? {
                return Err(required(
                    "a unique recovery plan already exists for this candidate",
                ));
            }
            for disk in self.members() {
                write_json(&member_plan_path(&disk, &hash), &plan, false)?;
                store_object(&disk, "recovery-plan", &plan)?;
            }
            write_json(&plan_path(&self.root, &plan.recovery_id), &plan, false)?;
            return Ok(plan);
        }
        let observed = observe(self, &candidate)?;
        require_permanent_loss(self, &candidate, &observed)?;
        let (version_protection, loss_details) = freeze_observation(observed);
        if !version_protection.iter().any(|v| {
            v["slots"]
                .as_array()
                .is_some_and(|s| s.iter().any(|s| s["state"] != "COMPLETE_VERIFIED"))
        }) {
            return Err(required(
                "all original payloads remain healthy; finish the original normal publication",
            ));
        }
        let mut plan = RecoveryPlan {
            recovery_id: new_id("recovery"),
            digest: String::new(),
            operation_id: operation.into(),
            candidate_hash: hash.clone(),
            principal: principal.into(),
            reason: reason.into(),
            isolation_generation: new_id("fence"),
            sequence: candidate.sequence,
            version_protection,
            loss_details,
            members,
            original_accepts,
        };
        plan.digest = plan_digest(&plan)?;
        for disk in self.members() {
            write_json(&member_plan_path(&disk, &hash), &plan, false)?;
            store_object(&disk, "recovery-plan", &plan)?;
        }
        write_json(&plan_path(&self.root, &plan.recovery_id), &plan, false)?;
        Ok(plan)
    }
    pub fn recovery_plan(&self, id: &str) -> Result<RecoveryPlan> {
        validate_id(id, "recovery")?;
        let local = plan_path(&self.root, id);
        let plan: RecoveryPlan = if local.exists() {
            read_json(&local)?
        } else {
            let mut found: Option<RecoveryPlan> = None;
            for disk in self.members() {
                let directory = disk.root.join("metadata/recovery-plans");
                if !directory.exists() {
                    continue;
                }
                for entry in fs::read_dir(directory)? {
                    let candidate: RecoveryPlan = read_json(&entry?.path())?;
                    if candidate.recovery_id == id {
                        if found
                            .as_ref()
                            .is_some_and(|old| old.digest != candidate.digest)
                        {
                            return Err(required("member recovery plans conflict"));
                        }
                        found = Some(candidate);
                    }
                }
            }
            let recovered = found.ok_or_else(|| {
                Error::new(
                    "RECOVERY_PLAN_NOT_FOUND",
                    "no member holds this recovery plan",
                )
            })?;
            if recovered.digest != plan_digest(&recovered)? {
                return Err(required("member recovery plan digest differs"));
            }
            write_json(&local, &recovered, false)?;
            recovered
        };
        if plan.recovery_id != id || plan.digest != plan_digest(&plan)? {
            return Err(required("recovery plan digest differs"));
        }
        Ok(plan)
    }
    pub fn commit_degraded_recovery(
        &mut self,
        id: &str,
        digest: &str,
        principal: &str,
        accept_permanent_loss: bool,
    ) -> Result<OperationResult> {
        self.require_no_commit_preparation()?;
        let plan = self.recovery_plan(id)?;
        if plan.digest != digest || plan.principal != principal {
            return Err(Error::new(
                "PERMISSION_DENIED",
                "recovery plan binding differs",
            ));
        }
        if !accept_permanent_loss {
            return Err(Error::new(
                "PERMANENT_LOSS_NOT_ACKNOWLEDGED",
                "explicit permanent-loss acknowledgement is required",
            ));
        }
        if let Some(result) = self.state.operations.get(&plan.operation_id) {
            if result.result["recovery_id"].as_str() == Some(id) {
                return self.operation(&plan.operation_id);
            }
            return Err(required(
                "the original operation already has a different published result",
            ));
        }
        let (candidate, mut next, members, accepts) =
            self.pending_recovery_candidate(&plan.operation_id, Some(&plan))?;
        let (observed, loss_details) = freeze_observation(observe(self, &candidate)?);
        if canonical_json(&members)? != canonical_json(&plan.members)?
            || canonical_json(&accepts)? != canonical_json(&plan.original_accepts)?
            || canonical_json(&observed)? != canonical_json(&plan.version_protection)?
            || canonical_json(&loss_details)? != canonical_json(&plan.loss_details)?
        {
            return Err(Error::new(
                "PLAN_STALE",
                "recovery evidence changed after preview",
            ));
        }
        require_permanent_loss(self, &candidate, &plan.version_protection)?;
        let member_disks = self.members();
        // Dependences and the immutable candidate are persisted before fencing.
        // Complete the exact hash-bound snapshot, not a reserialization using
        // today's page partitioning. Older pending candidates keep their root.
        self.repair_bound_snapshot(&candidate.snapshot_hash)?;
        let manifest_hashes =
            crate::history_package::store_history_manifests(&member_disks, &next)?;
        if !candidate.history_manifest_hashes.is_empty()
            && manifest_hashes != candidate.history_manifest_hashes
        {
            return Err(required("recovery history manifests differ"));
        }
        let mut result = next
            .operations
            .get(&candidate.operation_id)
            .cloned()
            .ok_or_else(|| required("candidate result missing"))?;
        result.commit_state = "PUBLISHED_DEGRADED".into();
        result.protection_at_publish = "BELOW_TARGET".into();
        result.current_health = if plan.version_protection.iter().any(|v| {
            !v["slots"]
                .as_array()
                .expect("slots")
                .iter()
                .any(|s| s["state"] == "COMPLETE_VERIFIED")
        }) {
            "UNAVAILABLE"
        } else {
            "DEGRADED"
        }
        .into();
        result.result["recovery_id"] = json!(id);
        result.result["recovery_plan_digest"] = json!(plan.digest);
        result.result["isolation_generation"] = json!(plan.isolation_generation);
        result.result["outcome"] = json!("COMMITTED_DEGRADED");
        result.result["version_protection"] = json!(plan.version_protection);
        crate::metadata_budget::preflight_publication(
            self,
            &candidate,
            &result,
            &next,
            Some(RecoveryProof {
                plan: plan.clone(),
                reprotect: Vec::new(),
            }),
            MAX_METADATA,
        )?;
        let mut reprotect = Vec::new();
        let plan_hash = object_hash("recovery-plan", &plan)?;
        for (index, disk) in member_disks.iter().enumerate() {
            store_object(disk, "candidate", &candidate)?;
            store_object(disk, "recovery-plan", &plan)?;
            reprotect.push(append(
                disk,
                "REPROTECT",
                candidate.sequence,
                &plan.candidate_hash,
                &plan_hash,
            )?);
            self.failpoint(&format!("after_recovery_reprotect_{}", index + 1))?;
        }
        let finalize = Finalize {
            candidate_hash: plan.candidate_hash.clone(),
            accepts: plan.original_accepts.clone(),
            recovery_id: Some(id.into()),
        };
        let final_hash = object_hash("finalize", &finalize)?;
        let mut finalize_receipts = Vec::new();
        for disk in &member_disks {
            store_object(disk, "finalize", &finalize)?;
            finalize_receipts.push(append(
                disk,
                "FINALIZE",
                candidate.sequence,
                &plan.candidate_hash,
                &final_hash,
            )?);
        }
        self.failpoint("after_recovery_finalize")?;
        let mut payload_receipts = Vec::new();
        for evidence in &candidate.versions {
            let health = plan
                .version_protection
                .iter()
                .find(|v| v["version_id"] == evidence.version.version_id)
                .ok_or_else(|| required("version health missing"))?;
            for slot in &evidence.placement.route_slot_ids {
                if !health["slots"]
                    .as_array()
                    .expect("frozen slots")
                    .iter()
                    .any(|s| s["slot_id"] == *slot && s["state"] == "COMPLETE_VERIFIED")
                {
                    continue;
                }
                let disk = self.disk(slot)?;
                let dir = version_dir(
                    disk,
                    &evidence.version.history_tree_id,
                    &evidence.version.version_id,
                );
                verify_payload(&dir, &evidence.version)?;
                let proof = CommitProof {
                    candidate_hash: plan.candidate_hash.clone(),
                    finalize: finalize.clone(),
                    version_hash: object_hash("version", &evidence.version)?,
                    disk_id: disk.disk_id.clone(),
                    slot_id: slot.clone(),
                    route_slot_ids: evidence.placement.route_slot_ids.clone(),
                };
                let proof_dir = local_proof_directory(&dir, &plan.candidate_hash)?;
                write_json(&proof_dir.join("commit-proof.json"), &proof, true)?;
                payload_receipts.push(PayloadReceipt {
                    version_id: evidence.version.version_id.clone(),
                    disk_id: disk.disk_id.clone(),
                    slot_id: slot.clone(),
                    proof_hash: object_hash("commit-proof", &proof)?,
                });
            }
        }
        let publication = Publication {
            candidate_hash: plan.candidate_hash.clone(),
            finalize,
            finalize_receipts,
            payload_receipts,
            result: result.clone(),
            recovery: Some(RecoveryProof {
                plan: plan.clone(),
                reprotect,
            }),
        };
        publication
            .recovery
            .as_ref()
            .expect("recovery")
            .verify(&candidate, &publication)?;
        self.complete_candidate(&candidate, &plan.candidate_hash, &next, Some(publication))?;
        next.operations
            .insert(candidate.operation_id.clone(), result.clone());
        self.state = next;
        self.snapshot = Some(candidate.snapshot_hash);
        self.next_slot = self.next_slot.max(
            candidate
                .sequence
                .checked_add(1)
                .ok_or_else(|| required("decision sequence exhausted"))?,
        );
        // Publication is durable, but a failed local reconciliation must retain
        // the write fence, just like ordinary publication's index update. A
        // restart can rebuild the disposable index and reconcile workspaces.
        self.rebuild_index()?;
        self.load_workspaces()?;
        self.blocked = false;
        self.operation(&result.operation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(point: &str) -> (tempfile::TempDir, Engine, String, Candidate) {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("p"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&temp.path().join("state"), config).unwrap();
        let repo = engine
            .create_repository("恢复测试", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "alice")
            .unwrap();
        engine
            .stage_bytes(
                &workspace.workspace_id,
                "lost.txt",
                b"permanently lost bytes",
            )
            .unwrap();
        engine
            .stage_bytes(
                &workspace.workspace_id,
                "healthy.txt",
                b"retained full bytes",
            )
            .unwrap();
        let operation = new_id("op");
        engine.inject_fault_once(point);
        assert!(
            engine
                .commit_workspace(CommitRequest {
                    workspace_id: workspace.workspace_id,
                    operation_id: operation.clone(),
                    principal: "alice".into(),
                    expected_head: workspace.base_head,
                    fencing_token: workspace.fencing_token,
                    message: "pending".into(),
                    request_digest: None
                })
                .is_err()
        );
        let member = engine.members()[0].clone();
        let records = scan(&member).unwrap();
        let record = records.records.last().unwrap();
        let candidate = load_object(&member, "candidate", &record.0.candidate_hash).unwrap();
        (temp, engine, operation, candidate)
    }
    fn lose(engine: &Engine, candidate: &Candidate, all_copies: bool) {
        let lost = candidate
            .versions
            .iter()
            .find(|v| v.version.creation_name == "lost.txt")
            .unwrap();
        for slot in
            lost.placement
                .route_slot_ids
                .iter()
                .take(if all_copies { usize::MAX } else { 1 })
        {
            let disk = engine.disk(slot).unwrap();
            fs::remove_file(
                crate::container_storage::content_path(
                    &version_dir(
                        disk,
                        &lost.version.history_tree_id,
                        &lost.version.version_id,
                    ),
                    &lost.version,
                )
                .unwrap(),
            )
            .unwrap();
        }
        if all_copies {
            for workspace in engine.workspaces.values() {
                for source in workspace
                    .files
                    .values()
                    .filter(|f| f.content_hash == lost.version.content_hash)
                {
                    fs::remove_file(&source.source_path).unwrap();
                }
            }
        }
    }
    #[test]
    fn explicit_two_accept_recovery_preserves_candidate_and_reads_remaining_files_after_restart() {
        let (temp, engine, operation, candidate) = fixture("after_accept_2");
        lose(&engine, &candidate, true);
        drop(engine);
        let mut engine = Engine::open_for_recovery(&temp.path().join("state")).unwrap();
        assert!(engine.is_read_only());
        let plan = engine
            .prepare_degraded_recovery(
                &operation,
                "alice",
                "all registered copies and staged source were permanently lost",
            )
            .unwrap();
        assert_eq!(plan.original_accepts.len(), 2);
        assert_eq!(
            engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", false)
                .unwrap_err()
                .code(),
            "PERMANENT_LOSS_NOT_ACKNOWLEDGED"
        );
        let result = engine
            .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
            .unwrap();
        assert_eq!(result.commit_state, "PUBLISHED_DEGRADED");
        assert_eq!(result.protection_at_publish, "BELOW_TARGET");
        assert_eq!(result.current_health, "UNAVAILABLE");
        let disk = engine.members()[0].clone();
        let scanned = scan(&disk).unwrap();
        let (record, _) = scanned.records.last().unwrap();
        let publication: Publication =
            load_object(&disk, "publication", &record.object_hash).unwrap();
        let original = load_snapshot(&disk, &candidate.snapshot_hash).unwrap();
        let bound = crate::metadata_budget::preflight_publication(
            &engine,
            &candidate,
            &publication.result,
            &original,
            publication.recovery.clone(),
            MAX_METADATA,
        )
        .unwrap();
        let actual = candidate
            .versions
            .iter()
            .filter(|e| e.version.creation_name == "healthy.txt")
            .flat_map(|e| e.placement.route_slot_ids.iter().map(move |slot| (e, slot)))
            .map(|(e, slot)| {
                fs::read(
                    version_dir(
                        engine.disk(slot).unwrap(),
                        &e.version.history_tree_id,
                        &e.version.version_id,
                    )
                    .join("publication-proof.json"),
                )
                .unwrap()
                .len()
            })
            .max()
            .unwrap();
        assert_eq!(bound, actual);
        assert_eq!(
            crate::metadata_budget::preflight_publication(
                &engine,
                &candidate,
                &publication.result,
                &original,
                publication.recovery.clone(),
                bound - 1
            )
            .unwrap_err()
            .code(),
            "RESOURCE_LIMIT"
        );

        assert_eq!(
            object_hash("candidate", &candidate).unwrap(),
            plan.candidate_hash
        );
        assert_eq!(
            engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                .unwrap()
                .sequence,
            result.sequence
        );
        let healthy = candidate
            .versions
            .iter()
            .find(|v| v.version.creation_name == "healthy.txt")
            .unwrap();
        assert!(engine.open_content(&healthy.version.version_id).is_ok());
        let lost = candidate
            .versions
            .iter()
            .find(|v| v.version.creation_name == "lost.txt")
            .unwrap();
        assert!(engine.open_content(&lost.version.version_id).is_err());
        drop(engine);
        let engine = Engine::open(&temp.path().join("state")).unwrap();
        let replay = engine.operation(&operation).unwrap();
        assert_eq!(replay.commit_state, "PUBLISHED_DEGRADED");
        assert_eq!(replay.protection_at_publish, "BELOW_TARGET");
        assert!(engine.open_content(&healthy.version.version_id).is_ok());
        for disk in engine.members() {
            let same: Candidate = load_object(&disk, "candidate", &plan.candidate_hash).unwrap();
            assert_eq!(
                object_hash("candidate", &same).unwrap(),
                plan.candidate_hash
            );
        }
        // The next clean restart may skip historical snapshot decoding, but it
        // must preserve the degraded-publication result overlay exactly.
        drop(engine);
        crate::journal::SNAPSHOT_LOAD_COUNT.with(|count| count.set(0));
        let engine = Engine::open(&temp.path().join("state")).unwrap();
        assert_eq!(
            crate::journal::SNAPSHOT_LOAD_COUNT.with(|count| count.get()),
            1
        );
        let replay = engine.operation(&operation).unwrap();
        assert_eq!(replay.commit_state, "PUBLISHED_DEGRADED");
        assert_eq!(replay.protection_at_publish, "BELOW_TARGET");
        assert!(engine.open_content(&healthy.version.version_id).is_ok());
        assert!(engine.open_content(&lost.version.version_id).is_err());
    }
    #[test]
    fn degraded_recovery_uses_verified_snapshot_replica_and_repairs_missing_page() {
        for corrupt in [false, true] {
            let (temp, engine, operation, candidate) = fixture("after_accept_2");
            lose(&engine, &candidate, true);
            let members = engine.members();
            let snapshot: Snapshot =
                load_object(&members[0], "snapshot", &candidate.snapshot_hash).unwrap();
            let page = snapshot.pages.values().flatten().next().unwrap();
            let path = object_path(&members[0], "state-page", page);
            let original = fs::read(&path).unwrap();
            if corrupt {
                fs::write(&path, b"corrupt").unwrap();
            } else {
                fs::remove_file(&path).unwrap();
            }
            drop(engine);
            let root = temp.path().join("state");
            let mut engine = Engine::open_for_recovery(&root).unwrap();
            let plan = engine
                .prepare_degraded_recovery(&operation, "alice", "permanent loss")
                .unwrap();
            let result = engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                .unwrap();
            assert_eq!(result.commit_state, "PUBLISHED_DEGRADED");
            assert_eq!(fs::read(&path).unwrap(), original);
            assert_eq!(engine.snapshot.as_ref(), Some(&candidate.snapshot_hash));
            drop(engine);
            let engine = Engine::open(&root).unwrap();
            assert_eq!(
                engine.operation(&operation).unwrap().commit_state,
                "PUBLISHED_DEGRADED"
            );
        }
    }

    #[test]
    fn degraded_recovery_rejects_unverifiable_snapshot_before_reprotect() {
        let (_temp, mut engine, operation, candidate) = fixture("after_accept_2");
        lose(&engine, &candidate, true);
        let plan = engine
            .prepare_degraded_recovery(&operation, "alice", "permanent loss")
            .unwrap();
        let members = engine.members();
        let snapshot: Snapshot =
            load_object(&members[0], "snapshot", &candidate.snapshot_hash).unwrap();
        let page = snapshot.pages.values().flatten().next().unwrap();
        let logs = members
            .iter()
            .map(|d| fs::read(log_path(d)).unwrap())
            .collect::<Vec<_>>();
        for member in &members {
            fs::remove_file(object_path(member, "state-page", page)).unwrap();
        }
        assert!(
            engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                .is_err()
        );
        assert!(engine.is_read_only());
        assert!(!engine.state.operations.contains_key(&operation));
        for (member, log) in members.iter().zip(logs) {
            assert_eq!(fs::read(log_path(member)).unwrap(), log);
        }
    }

    #[test]
    fn degraded_publication_stays_read_only_when_local_reconciliation_fails() {
        for fail_index in [true, false] {
            let (temp, engine, operation, candidate) = fixture("after_accept_2");
            lose(&engine, &candidate, true);
            drop(engine);
            let root = temp.path().join("state");
            let mut engine = Engine::open_for_recovery(&root).unwrap();
            let plan = engine
                .prepare_degraded_recovery(&operation, "alice", "permanent loss")
                .unwrap();
            let obstruction = if fail_index {
                let path = root.join("index.sqlite");
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
                path
            } else {
                let path = root.join("workspaces").join("broken-workspace");
                fs::create_dir_all(&path).unwrap();
                fs::write(path.join("workspace.json"), b"{}").unwrap();
                path
            };
            let error = engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                .unwrap_err();
            if fail_index {
                assert_eq!(error.code(), "UNSAFE_PATH");
            }
            assert!(
                engine.is_read_only(),
                "failed reconciliation must retain the write fence"
            );
            assert_eq!(
                engine.operation(&operation).unwrap().commit_state,
                "PUBLISHED_DEGRADED"
            );
            assert_eq!(
                engine
                    .create_repository("must-not-write", "P01", 3, "alice", &new_id("op"))
                    .unwrap_err()
                    .code(),
                "RECOVERY_REQUIRED"
            );
            // Idempotent result lookup must not reopen writes either.
            engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                .unwrap();
            assert!(engine.is_read_only());
            let expected = engine.state.clone();
            drop(engine);
            fs::remove_dir_all(&obstruction).unwrap();
            let engine = Engine::open(&root).unwrap();
            assert!(!engine.is_read_only());
            assert_eq!(engine.state, expected);
            assert_eq!(
                engine.operation(&operation).unwrap().commit_state,
                "PUBLISHED_DEGRADED"
            );
        }
    }

    #[test]
    fn insufficient_accepts_or_repairable_bytes_never_authorize_degraded_publication() {
        let (temp, engine, operation, candidate) = fixture("after_accept_1");
        lose(&engine, &candidate, true);
        drop(engine);
        let mut engine = Engine::open_for_recovery(&temp.path().join("state")).unwrap();
        assert_eq!(
            engine
                .prepare_degraded_recovery(&operation, "alice", "loss")
                .unwrap_err()
                .code(),
            "RECOVERY_REQUIRED"
        );
        let (temp, engine, operation, candidate) = fixture("after_accept_2");
        lose(&engine, &candidate, false);
        drop(engine);
        let mut engine = Engine::open_for_recovery(&temp.path().join("state")).unwrap();
        assert_eq!(
            engine
                .prepare_degraded_recovery(&operation, "alice", "one copy lost")
                .unwrap_err()
                .code(),
            "RECOVERY_REQUIRED"
        );
    }
    #[test]
    fn original_normal_publication_is_preserved_when_content_was_lost_later() {
        let (temp, engine, operation, candidate) = fixture("after_publication_1");
        lose(&engine, &candidate, true);
        drop(engine);
        let mut engine = Engine::open_for_recovery(&temp.path().join("state")).unwrap();
        let result = engine.operation(&operation).unwrap();
        assert_eq!(result.commit_state, "PUBLISHED");
        assert_eq!(result.protection_at_publish, "TARGET_MET");
        assert_eq!(result.current_health, "UNAVAILABLE");
        assert_eq!(
            engine
                .prepare_degraded_recovery(&operation, "alice", "later loss")
                .unwrap_err()
                .code(),
            "RECOVERY_REQUIRED"
        );
    }
    #[test]
    fn degraded_recovery_resumes_the_same_plan_after_each_durable_boundary() {
        for point in [
            "after_recovery_reprotect_1",
            "after_recovery_finalize",
            "after_recovery_publication_1",
        ] {
            let (temp, engine, operation, candidate) = fixture("after_accept_2");
            lose(&engine, &candidate, true);
            drop(engine);
            let root = temp.path().join("state");
            let mut engine = Engine::open_for_recovery(&root).unwrap();
            let plan = engine
                .prepare_degraded_recovery(&operation, "alice", "permanent loss")
                .unwrap();
            engine.inject_fault_once(point);
            assert!(
                engine
                    .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                    .is_err(),
                "{point}"
            );
            drop(engine);
            if point == "after_recovery_reprotect_1" {
                fs::remove_file(plan_path(&root, &plan.recovery_id)).unwrap();
            }
            let mut engine = Engine::open_for_recovery(&root).unwrap();
            let result = engine
                .commit_degraded_recovery(&plan.recovery_id, &plan.digest, "alice", true)
                .unwrap();
            assert_eq!(result.result["recovery_id"], plan.recovery_id, "{point}");
            assert_eq!(result.commit_state, "PUBLISHED_DEGRADED", "{point}");
        }
    }
    #[test]
    fn incomplete_log_cannot_prove_absence_and_unknown_record_types_are_rejected() {
        use std::io::Write;
        let (temp, engine, operation, candidate) = fixture("after_accept_2");
        lose(&engine, &candidate, true);
        let disk = engine.members()[0].clone();
        drop(engine);
        let log = log_path(&disk);
        let mut file = std::fs::OpenOptions::new().append(true).open(&log).unwrap();
        file.write_all(b"ZXSL").unwrap();
        file.sync_all().unwrap();
        drop(file);
        let length = fs::metadata(&log).unwrap().len();
        let mut engine = Engine::open_for_recovery(&temp.path().join("state")).unwrap();
        assert_eq!(
            engine
                .prepare_degraded_recovery(&operation, "alice", "loss")
                .unwrap_err()
                .code(),
            "RECOVERY_REQUIRED"
        );
        assert_eq!(fs::metadata(log).unwrap().len(), length);
        drop(engine);
        let (temp, engine, _operation, candidate) = fixture("after_accept_2");
        let disk = engine.members()[0].clone();
        append(
            &disk,
            "UNRECOGNIZED",
            candidate.sequence,
            &object_hash("candidate", &candidate).unwrap(),
            &candidate.snapshot_hash,
        )
        .unwrap();
        drop(engine);
        assert_eq!(
            Engine::open_for_recovery(&temp.path().join("state"))
                .err()
                .unwrap()
                .code(),
            "LOG_FORMAT_UNSUPPORTED"
        );
    }

    #[test]
    fn prepare_only_failure_reserves_its_slot_and_is_not_an_indeterminate_operation() {
        let (temp, mut engine, operation, _candidate) = fixture("after_prepare_1");
        assert!(!engine.is_read_only());
        assert_eq!(
            engine.operation(&operation).unwrap_err().code(),
            "OPERATION_NOT_FOUND"
        );
        assert!(engine.operation_owner(&operation).is_none());
        let branch = engine
            .state
            .repositories
            .values()
            .next()
            .unwrap()
            .main_branch_id
            .clone();
        let workspace = engine.create_workspace(&branch, "alice").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "next.txt", b"next")
            .unwrap();
        let result = engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "after preparation failure".into(),
                request_digest: None,
            })
            .unwrap();
        assert_eq!(result.sequence, 3);
        drop(engine);
        assert!(Engine::open(&temp.path().join("state")).is_ok());
    }
}
