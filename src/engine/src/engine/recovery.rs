//! Replay the verified publication prefix and reconcile pending decisions.
use super::Engine;
use crate::publication_evidence::{
    repair_object_copies, verified_object, verified_publication_seal, verified_snapshot,
};
use crate::{disk::*, journal::*, *};
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    /// Read the complete exact candidate-bound object closure across replicas.
    /// A damaged preferred member must not hide healthy metadata dependencies.
    pub(crate) fn load_bound_snapshot(&self, hash: &str) -> Result<State> {
        verified_snapshot(&self.members(), hash)
    }

    /// Preserve the candidate's original snapshot layout when completing a
    /// persisted decision, including decisions made by an older page writer.
    pub(crate) fn repair_bound_snapshot(&self, hash: &str) -> Result<()> {
        let members = self.members();
        verified_snapshot(&members, hash)?;
        repair_snapshot_copies(&members, &members, hash, &mut BTreeSet::new())
    }

    pub(super) fn recover_mode(&mut self, complete_unpublished: bool) -> Result<()> {
        let members = self.members();
        let mut online = Vec::new();
        let mut prefixes = BTreeMap::<String, BTreeSet<String>>::new();
        let mut torn_tail = false;
        let mut records = BTreeMap::<u64, BTreeMap<String, Vec<(String, Record)>>>::new();
        for d in &members {
            if validate_seal(&self.config, d).is_err() {
                continue;
            }
            let scanned = scan(d)?;
            torn_tail |= scanned.torn_tail;
            prefixes.insert(
                d.disk_id.clone(),
                scanned
                    .records
                    .iter()
                    .map(|(_, hash)| hash.clone())
                    .collect(),
            );
            for (r, _) in scanned.records {
                self.next_slot =
                    self.next_slot.max(r.sequence.checked_add(1).ok_or_else(|| {
                        Error::new("RECOVERY_REQUIRED", "journal decision sequence exhausted")
                    })?);
                records
                    .entry(r.sequence)
                    .or_default()
                    .entry(r.candidate_hash.clone())
                    .or_default()
                    .push((d.slot_id.clone(), r));
            }
            online.push(d.clone());
        }
        if online.len() < 2 {
            return Err(Error::new(
                "METADATA_QUORUM_UNAVAILABLE",
                "online service requires at least two trusted metadata members; use offline recovery",
            ));
        }
        let checkpoint = if online.len() == 3 && !torn_tail {
            super::recovery_checkpoint::load(self, &prefixes, &records).filter(|c| {
                records
                    .get(&c.sequence)
                    .is_some_and(|candidates| candidates.contains_key(&c.candidate_hash))
            })
        } else {
            None
        };
        // Recovery holds the pool writer locks. Immutable page replicas repaired
        // in this invocation need not be decoded and compared again for each
        // historical root that references them. Never persist this evidence.
        let mut repaired_pages = BTreeSet::new();
        let mut repaired_histories = BTreeSet::new();
        for (_seq, candidates) in records {
            let active = candidates
                .into_iter()
                .filter(|(_, rs)| {
                    rs.iter()
                        .any(|(_, r)| r.record_type == "ACCEPT" || r.record_type == "PUBLICATION")
                })
                .collect::<Vec<_>>();
            if active.len() > 1 {
                return Err(Error::new(
                    "LOG_FORK",
                    "different accepted candidates occupy the same slot",
                ));
            }
            let Some((hash, rs)) = active.into_iter().next() else {
                continue;
            };
            let candidate: Candidate = verified_object(&online, "candidate", &hash)?;
            if candidate.pool_id != self.config.pool_id
                || candidate.previous_snapshot != self.snapshot
            {
                return Err(Error::new(
                    "LOG_FORK",
                    "candidate does not extend the verified publication prefix",
                ));
            }
            let publication_record = rs.iter().find(|(_, r)| r.record_type == "PUBLICATION");
            let publication = if let Some((_, r)) = publication_record {
                Some(verified_object::<Publication>(
                    &online,
                    "publication",
                    &r.object_hash,
                )?)
            } else {
                crate::degraded_recovery::discover_publication(self, &candidate, &hash)?
            };
            if let Some(checkpoint) = checkpoint
                .as_ref()
                .filter(|c| candidate.sequence <= c.sequence)
            {
                let publication = publication.as_ref().ok_or_else(|| {
                    Error::new("CHECKPOINT_INVALID", "covered publication is absent")
                })?;
                // Recheck the publication barrier independently of the checkpoint.
                // Coverage established that all historical repair artifacts remain
                // byte-for-byte present after the previous successful full replay.
                for d in &online {
                    let seal: PublicationSeal = read_json(&publication_seal_path(d, &hash))?;
                    verify_publication_seal(&candidate, publication, &seal)?;
                }
                // Checkpoint coverage never substitutes for candidate-bound graph verification.
                crate::history_package::bound_history_objects(&online, &candidate, None)?;
                if candidate.sequence == checkpoint.sequence {
                    if hash != checkpoint.candidate_hash
                        || candidate.snapshot_hash != checkpoint.snapshot_hash
                    {
                        return Err(Error::new(
                            "CHECKPOINT_INVALID",
                            "checkpoint boundary differs from journal",
                        ));
                    }
                    let mut next = verified_snapshot(&online, &candidate.snapshot_hash)?;
                    if publication.recovery.is_some() {
                        next.operations
                            .insert(candidate.operation_id.clone(), publication.result.clone());
                    }
                    self.state = next;
                }
                self.snapshot = Some(candidate.snapshot_hash);
                continue;
            }
            let mut next = verified_snapshot(&online, &candidate.snapshot_hash)?;
            let history_objects =
                crate::history_package::bound_history_objects(&online, &candidate, Some(&next))?;
            if publication.is_none() && !complete_unpublished {
                self.blocked = true;
                break;
            }
            if publication.is_none() && complete_unpublished && online.len() == 3 {
                let result = next
                    .operations
                    .get(&candidate.operation_id)
                    .ok_or_else(|| Error::new("STATE_INVALID", "candidate result is absent"))?;
                crate::metadata_budget::preflight_bound_publication(
                    self,
                    &candidate,
                    result,
                    &next,
                    None,
                    MAX_METADATA,
                )?;
                self.repair_pending_payloads(&candidate)?;
            }
            if online.len() == 3 {
                // Repair only replicas of the exact hash-bound snapshot. Never
                // manufacture a replacement state from an unverifiable copy.
                repair_snapshot_copies(
                    &online,
                    &members,
                    &candidate.snapshot_hash,
                    &mut repaired_pages,
                )?;
                // The original hash-bound root and all of its pages are now
                // installed on every member. Re-encoding State here duplicates
                // that work and can create unrelated roots when the writer's
                // page partitioning has changed since this snapshot was saved.
                crate::history_package::repair_history_objects(
                    &members,
                    &history_objects,
                    &mut repaired_histories,
                )?;
                repair_object_copies(&members, "candidate", &hash, &candidate)?;
                if let Some(publication) = &publication {
                    repair_object_copies(
                        &members,
                        "publication",
                        &object_hash("publication", publication)?,
                        publication,
                    )?;
                }
                let recovered_result = publication
                    .as_ref()
                    .filter(|p| p.recovery.is_some())
                    .map(|p| p.result.clone());
                self.complete_candidate(&candidate, &hash, &next, publication)?;
                if let Some(result) = recovered_result {
                    next.operations
                        .insert(candidate.operation_id.clone(), result);
                }
                self.state = next;
                self.snapshot = Some(candidate.snapshot_hash);
            } else if let Some(p) = publication {
                if p.candidate_hash != hash || p.result.operation_id != candidate.operation_id {
                    return Err(Error::new(
                        "PUBLICATION_INVALID",
                        "publication differs from candidate",
                    ));
                }
                // A PUBLICATION object or a single frame is not the complete
                // barrier. Preserve the old visible state until all three
                // synchronized receipts can be checked after restart.
                let sealed = verified_publication_seal(&online, &candidate, &p).is_ok();
                if !sealed {
                    self.blocked = true;
                    break;
                }
                if p.recovery.is_some() {
                    next.operations
                        .insert(candidate.operation_id.clone(), p.result.clone());
                }
                self.state = next;
                self.snapshot = Some(candidate.snapshot_hash);
                self.blocked = true;
            } else {
                self.blocked = true;
                break;
            }
        }
        if online.len() != 3 {
            self.blocked = true;
        }
        self.cleanup_purged()?;
        // Optional acceleration only. Failure to create a checkpoint must never
        // fail or roll back an otherwise verified recovery. Existing checkpoints
        // become ineligible whenever their journal/coverage hashes differ.
        if checkpoint
            .as_ref()
            .is_none_or(|c| c.sequence < self.state.sequence)
        {
            let _ = super::recovery_checkpoint::save(self);
        }
        Ok(())
    }
}

// Optimization only: bound hash storage independently of retained history.
// Once full, uncached pages still receive the complete verification/repair.
const MAX_REPAIRED_PAGES: usize = 16_384;
fn remember_repaired_page(repaired: &mut BTreeSet<String>, hash: &str) {
    if repaired.len() < MAX_REPAIRED_PAGES {
        repaired.insert(hash.to_owned());
    }
}

fn repair_snapshot_copies(
    sources: &[DiskConfig],
    members: &[DiskConfig],
    hash: &str,
    repaired_pages: &mut BTreeSet<String>,
) -> Result<()> {
    fn install<T: data_serialization::Serialize + data_serialization::de::DeserializeOwned>(
        sources: &[DiskConfig],
        members: &[DiskConfig],
        kind: &str,
        hash: &str,
    ) -> Result<()> {
        let value: T = verified_object(sources, kind, hash)?;
        repair_object_copies(members, kind, hash, &value)
    }
    let snapshot: Snapshot = verified_object(sources, "snapshot", hash)?;
    for hash in snapshot.pages.values().flatten() {
        if !repaired_pages.contains(hash) {
            install::<json_codec::Map<String, json_codec::Value>>(
                sources,
                members,
                "state-page",
                hash,
            )?;
            // Record only after every replica was successfully checked/repaired.
            remember_repaired_page(repaired_pages, hash);
            #[cfg(test)]
            PAGE_REPAIR_COUNT.with(|count| count.set(count.get() + 1));
        }
    }
    install::<Snapshot>(sources, members, "snapshot", hash)
}

#[cfg(test)]
thread_local! {
    static PAGE_REPAIR_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "../../../../test/engine/unit/engine/recovery/tests.rs"]
mod tests;
