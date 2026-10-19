//! Replay the verified publication prefix and reconcile pending decisions.
use super::Engine;
use crate::{disk::*, journal::*, *};
use std::collections::BTreeMap;

impl Engine {
    pub(super) fn recover_mode(&mut self, complete_unpublished: bool) -> Result<()> {
        let members = self.members();
        let mut online = Vec::new();
        let mut records = BTreeMap::<u64, BTreeMap<String, Vec<(String, Record)>>>::new();
        for d in &members {
            if validate_seal(&self.config, d).is_err() {
                continue;
            }
            let scanned = scan(d)?;
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
            let mut next = load_snapshot(source, &candidate.snapshot_hash)?;
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
            if publication.is_none() && !complete_unpublished {
                self.blocked = true;
                break;
            }
            if publication.is_none() && complete_unpublished && online.len() == 3 {
                self.repair_pending_payloads(&candidate)?;
            }
            if online.len() == 3 {
                // Copy verified immutable dependencies, never manufacture a new candidate.
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
        Ok(())
    }
}
