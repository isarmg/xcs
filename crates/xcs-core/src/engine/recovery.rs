//! Replay the verified publication prefix and reconcile pending decisions.
use super::Engine;
use crate::{disk::*, journal::*, *};
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
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
            let source = online
                .iter()
                .find(|d| object_path(d, "candidate", &hash).exists())
                .ok_or_else(|| {
                    Error::new(
                        "RECOVERY_REQUIRED",
                        "candidate dependencies are unavailable",
                    )
                })?;
            let candidate: Candidate = load_object(source, "candidate", &hash)?;
            if candidate.pool_id != self.config.pool_id
                || candidate.previous_snapshot != self.snapshot
            {
                return Err(Error::new(
                    "LOG_FORK",
                    "candidate does not extend the verified publication prefix",
                ));
            }
            let publication_record = rs.iter().find(|(_, r)| r.record_type == "PUBLICATION");
            let publication = if let Some((slot, r)) = publication_record {
                let d = self.disk(slot)?;
                Some(load_object::<Publication>(
                    d,
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
                if candidate.sequence == checkpoint.sequence {
                    if hash != checkpoint.candidate_hash
                        || candidate.snapshot_hash != checkpoint.snapshot_hash
                    {
                        return Err(Error::new(
                            "CHECKPOINT_INVALID",
                            "checkpoint boundary differs from journal",
                        ));
                    }
                    let (mut next, _) = verified_snapshot(&online, &candidate.snapshot_hash)?;
                    if publication.recovery.is_some() {
                        next.operations
                            .insert(candidate.operation_id.clone(), publication.result.clone());
                    }
                    self.state = next;
                }
                self.snapshot = Some(candidate.snapshot_hash);
                continue;
            }
            let (mut next, snapshot_source) = verified_snapshot(&online, &candidate.snapshot_hash)?;
            if publication.is_none() && !complete_unpublished {
                self.blocked = true;
                break;
            }
            if publication.is_none() && complete_unpublished && online.len() == 3 {
                self.repair_pending_payloads(&candidate)?;
            }
            if online.len() == 3 {
                // Repair only replicas of the exact hash-bound snapshot. Never
                // manufacture a replacement state from an unverifiable copy.
                repair_snapshot_copies(&snapshot_source, &members, &candidate.snapshot_hash)?;
                store_snapshot(&members, &next)?;
                let manifests = crate::history_package::store_history_manifests(&members, &next)?;
                if !candidate.history_manifest_hashes.is_empty()
                    && manifests != candidate.history_manifest_hashes
                {
                    return Err(Error::new(
                        "OBJECT_CORRUPT",
                        "history recovery manifest differs from candidate",
                    ));
                }
                for d in &members {
                    store_object(d, "candidate", &candidate)?;
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
                let sealed = online
                    .iter()
                    .filter_map(|d| {
                        read_json::<PublicationSeal>(&publication_seal_path(d, &hash)).ok()
                    })
                    .any(|seal| verify_publication_seal(&candidate, &p, &seal).is_ok());
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

// A candidate binds one snapshot root. Try only replicas of that exact root;
// never substitute a different state or advance past a failed dependency.
fn verified_snapshot(members: &[DiskConfig], hash: &str) -> Result<(State, DiskConfig)> {
    let mut first_error = None;
    for member in members {
        match load_snapshot(member, hash) {
            Ok(state) => return Ok((state, member.clone())),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }
    Err(first_error
        .unwrap_or_else(|| Error::new("RECOVERY_REQUIRED", "no trusted snapshot replica")))
}
fn repair_snapshot_copies(source: &DiskConfig, members: &[DiskConfig], hash: &str) -> Result<()> {
    fn install<T: serde::Serialize + serde::de::DeserializeOwned>(
        source: &DiskConfig,
        members: &[DiskConfig],
        kind: &str,
        hash: &str,
    ) -> Result<()> {
        let value: T = load_object(source, kind, hash)?;
        let bytes = encode_metadata(&value, MAX_METADATA)?;
        for member in members {
            let path = object_path(member, kind, hash);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                    // Require a safe regular file even when replacing corrupt bytes.
                    open_regular(&path)?;
                    if read_bytes(&path, MAX_METADATA).ok().as_deref() != Some(bytes.as_slice()) {
                        write_atomic(&path, &bytes, true)?;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    write_atomic(&path, &bytes, false)?
                }
                Err(e) => return Err(e.into()),
                Ok(_) => {
                    return Err(Error::new(
                        "UNSAFE_PATH",
                        "snapshot repair target is not a regular file",
                    ));
                }
            }
        }
        Ok(())
    }
    let snapshot: Snapshot = load_object(source, "snapshot", hash)?;
    for hash in snapshot.pages.values().flatten() {
        install::<serde_json::Map<String, serde_json::Value>>(source, members, "state-page", hash)?;
    }
    install::<Snapshot>(source, members, "snapshot", hash)
}
