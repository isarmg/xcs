//! Durable preparation, decision and publication of one frozen transaction.
use super::{Engine, Prepared};
use crate::{disk::*, journal::*, *};
use serde_json::json;
use std::collections::BTreeMap;
#[path = "verified_payloads.rs"]
mod verified_payloads;
use verified_payloads::{PayloadPreparation, VerifiedPayloadSet};

/// An owned preparation, never serializable or constructible by callers.
pub(crate) struct DetachedPublication {
    next: State,
    candidate: Candidate,
    result: OperationResult,
    verified: VerifiedPayloadSet,
    base_root: std::path::PathBuf,
    base_snapshot: Option<String>,
    base_sequence: u64,
    base_next_slot: u64,
    config_hash: String,
}

impl Engine {
    pub(crate) fn persist(
        &mut self,
        next: State,
        prepared: Vec<Prepared>,
        result: OperationResult,
    ) -> Result<()> {
        self.persist_with_metadata_limit(next, prepared, result, MAX_METADATA)
    }
    pub(crate) fn persist_with_metadata_limit(
        &mut self,
        next: State,
        prepared: Vec<Prepared>,
        result: OperationResult,
        maximum: usize,
    ) -> Result<()> {
        let detached = self.prepare_publication(next, prepared, result, maximum)?;
        self.decide_publication(detached).map(|_| ())
    }
    pub(crate) fn prepare_publication(
        &self,
        next: State,
        prepared: Vec<Prepared>,
        result: OperationResult,
        maximum: usize,
    ) -> Result<DetachedPublication> {
        let members = self.members();
        let mut required = BTreeMap::<String, u64>::new();
        for p in &prepared {
            for slot in &p.placement.route_slot_ids {
                let disk = self.disk(slot)?;
                if verify_payload(
                    &version_dir(disk, &p.version.history_tree_id, &p.version.version_id),
                    &p.version,
                )
                .is_ok()
                {
                    continue;
                }
                *required.entry(slot.clone()).or_default() = required
                    .get(slot)
                    .copied()
                    .unwrap_or(0)
                    .checked_add(p.version.logical_size)
                    .ok_or_else(|| Error::new("RESOURCE_LIMIT", "admission bytes overflow"))?;
            }
        }
        for (slot, bytes) in &required {
            let d = self.disk(slot)?;
            validate_seal(&self.config, d)?;
            let free = available(&d.root)?;
            let used = self.used_bytes(slot);
            let budget = d
                .capacity_bytes
                .map(|limit| limit.saturating_sub(used))
                .unwrap_or(u64::MAX);
            if bytes.saturating_add(self.config.metadata_reserve_bytes) > free || *bytes > budget {
                return Err(Error::new(
                    "NO_SPACE",
                    format!("fixed route slot {slot} cannot reserve the complete payload"),
                ));
            }
        }
        // Resolve real page hashes and enforce every metadata bound without
        // publishing files or journal records. No estimate enters the decision.
        let snapshot_hash = store_snapshot_bounded(&[], &next, maximum)?;
        let history_manifest_hashes =
            crate::history_package::store_history_manifests_bounded(&[], &next, maximum)?;
        let candidate = Candidate {
            format_version: 2,
            pool_id: self.config.pool_id.clone(),
            sequence: next.sequence,
            transaction_id: new_id("tx"),
            operation_id: result.operation_id.clone(),
            request_digest: result.request_digest.clone(),
            previous_snapshot: self.snapshot.clone(),
            snapshot_hash,
            versions: prepared
                .iter()
                .map(|p| VersionEvidence {
                    version: p.version.clone(),
                    placement: p.placement.clone(),
                })
                .collect(),
            metadata_disk_ids: members.iter().map(|d| d.disk_id.clone()).collect(),
            history_manifest_hashes,
        };
        crate::metadata_budget::preflight_publication(
            self, &candidate, &result, &next, None, maximum,
        )?;
        let preparation = PayloadPreparation::new(&self.config, &candidate)?;
        self.prepare_payloads(&prepared)?;
        let verified = preparation.verify(&self.config, &candidate)?;
        if store_snapshot(&members, &next)? != candidate.snapshot_hash
            || crate::history_package::store_history_manifests(&members, &next)?
                != candidate.history_manifest_hashes
        {
            return Err(Error::new(
                "STATE_INVALID",
                "metadata changed after size preflight",
            ));
        }
        Ok(DetachedPublication {
            next,
            candidate,
            result,
            verified,
            base_root: self.root.clone(),
            base_snapshot: self.snapshot.clone(),
            base_sequence: self.state.sequence,
            base_next_slot: self.next_slot,
            config_hash: object_hash("publication-config", &self.config)?,
        })
    }
    pub(crate) fn decide_publication(
        &mut self,
        detached: DetachedPublication,
    ) -> Result<OperationResult> {
        if self.blocked
            || self.root != detached.base_root
            || self.snapshot != detached.base_snapshot
            || self.state.sequence != detached.base_sequence
            || self.next_slot != detached.base_next_slot
            || object_hash("publication-config", &self.config)? != detached.config_hash
        {
            return Err(Error::new(
                "PREPARATION_STALE",
                "engine changed during detached publication preparation",
            )
            .retryable());
        }
        let DetachedPublication {
            next,
            candidate,
            result,
            verified,
            ..
        } = detached;
        verified
            .validate(&self.config, &candidate)
            .map_err(|error| {
                Error::new(
                    "PREPARATION_STALE",
                    format!("prepared payload identity changed: {error}"),
                )
                .retryable()
            })?;
        // Preparation consumed payload capacity, but concurrent staged uploads
        // may have consumed the metadata reserve without changing State.
        for disk in &self.config.disks {
            if disk.metadata_member
                || candidate
                    .versions
                    .iter()
                    .any(|evidence| evidence.placement.route_slot_ids.contains(&disk.slot_id))
            {
                if available(&disk.root)? < self.config.metadata_reserve_bytes {
                    return Err(Error::new(
                        "NO_SPACE",
                        "metadata reserve was consumed during payload preparation",
                    ));
                }
            }
        }
        let members = self.members();
        for disk in &members {
            validate_seal(&self.config, disk)?;
        }
        self.failpoint("after_payloads")?;
        let hash = object_hash("candidate", &candidate)?;
        self.next_slot = candidate
            .sequence
            .checked_add(1)
            .ok_or_else(|| Error::new("RESOURCE_LIMIT", "decision sequence exhausted"))?;
        for (index, d) in members.iter().enumerate() {
            store_object(d, "candidate", &candidate)?;
            append(
                d,
                "PREPARE",
                candidate.sequence,
                &hash,
                &candidate.snapshot_hash,
            )?;
            self.failpoint(&format!("after_prepare_{}", index + 1))?;
        }
        // Once the first ACCEPT may have reached disk, failure is indeterminate.
        self.blocked = true;
        let finish =
            self.complete_candidate_verified(&candidate, &hash, &next, None, Some(&verified));
        if let Err(e) = finish {
            return Err(Error::new(
                "IN_DOUBT",
                format!("{}; query operation {} and recover", e, result.operation_id),
            )
            .retryable());
        }
        let previous = std::mem::replace(&mut self.state, next);
        self.snapshot = Some(candidate.snapshot_hash);
        self.update_index(&previous)?;
        self.blocked = false;
        Ok(result)
    }
    fn prepare_payloads(&self, prepared: &[Prepared]) -> Result<()> {
        // Versions of the same file can share a named container. Preserve their
        // order, and each version's route chain, while independent files copy
        // in bounded parallel batches. No decision is written until all join.
        let mut by_file = BTreeMap::<&str, Vec<&Prepared>>::new();
        for payload in prepared {
            by_file
                .entry(&payload.version.file_id)
                .or_default()
                .push(payload);
        }
        let groups: Vec<_> = by_file.into_values().collect();
        run_payload_groups(&groups, |payload| self.prepare_payload(payload))
    }
    pub(crate) fn prepare_payload(&self, p: &Prepared) -> Result<()> {
        let mut source = p.source.clone();
        for slot in &p.placement.route_slot_ids {
            let disk = self.disk(slot)?;
            validate_seal(&self.config, disk)?;
            let directory = version_dir(disk, &p.version.history_tree_id, &p.version.version_id);
            if directory.join("version.json").exists() {
                let existing: FileVersion = read_json(&directory.join("version.json"))?;
                if object_hash("version", &existing)? != object_hash("version", &p.version)? {
                    return Err(Error::new(
                        "OBJECT_COLLISION",
                        "version identity already exists with different bytes",
                    ));
                }
                verify_payload(&directory, &p.version)?;
            } else {
                let staging = disk.root.join("staging").join(new_id("op"));
                private_dir(&staging.join("content"))?;
                let target = staging.join("content").join(&p.version.creation_name);
                let (size, hash, checks) = full_copy(
                    &source,
                    &target,
                    self.config.checksum_chunk_bytes,
                    self.config.max_file_bytes,
                )?;
                if size != p.version.logical_size
                    || hash != p.version.content_hash
                    || blake3::hash(&checks.encode()?).to_hex().as_str()
                        != p.version.checksum_manifest_hash
                {
                    return Err(Error::new(
                        "SOURCE_CHANGED",
                        "prepared full file differs from the sealed input",
                    ));
                }
                write_json(&staging.join("version.json"), &p.version, false)?;
                write_atomic(&staging.join("checksums.bin"), &checks.encode()?, false)?;
                write_json(
                    &staging.join("payload.json"),
                    &json!({"format_version":2,"payload_id":format!("payload_{}",&p.version.version_id[4..]),"version_id":p.version.version_id,"kind":"FULL","size":size.to_string(),"content_hash":hash}),
                    false,
                )?;
                sync_dir(&staging)?;
                private_dir(directory.parent().expect("version parent"))?;
                let staging_parent = zxs_platform::fs_safety::linux::OpenAt2Root::open(
                    staging.parent().expect("staging parent"),
                    zxs_platform::fs_safety::linux::MountPolicy::SameMount,
                )
                .map_err(|error| Error::new("UNSAFE_PATH", error.to_string()))?;
                let version_parent = zxs_platform::fs_safety::linux::OpenAt2Root::open(
                    directory.parent().expect("version parent"),
                    zxs_platform::fs_safety::linux::MountPolicy::SameMount,
                )
                .map_err(|error| Error::new("UNSAFE_PATH", error.to_string()))?;
                rustix::fs::renameat_with(
                    staging_parent.directory(),
                    staging.file_name().expect("staging name"),
                    version_parent.directory(),
                    directory.file_name().expect("version name"),
                    rustix::fs::RenameFlags::NOREPLACE,
                )
                .map_err(std::io::Error::from)?;
                sync_dir(directory.parent().expect("version parent"))?;
            }
            // Install a real version file in this file's named container before
            // any protection receipt is issued. The next level reads this copy.
            source = crate::container_storage::install_version_content(&directory, &p.version)?;
        }
        Ok(())
    }
    pub(crate) fn complete_candidate(
        &mut self,
        c: &Candidate,
        hash: &str,
        next: &State,
        existing: Option<Publication>,
    ) -> Result<()> {
        self.complete_candidate_verified(c, hash, next, existing, None)
    }
    fn complete_candidate_verified(
        &mut self,
        c: &Candidate,
        hash: &str,
        next: &State,
        existing: Option<Publication>,
        verified: Option<&VerifiedPayloadSet>,
    ) -> Result<()> {
        if let Some(publication) = existing {
            if publication.candidate_hash != hash
                || publication.result.request_digest != c.request_digest
                || (publication.recovery.is_none() && publication.finalize.accepts.len() != 3)
                || publication.finalize_receipts.len() != 3
            {
                return Err(Error::new(
                    "PUBLICATION_INVALID",
                    "stored publication does not bind this candidate",
                ));
            }
            crate::container_storage::synchronize_names(&self.config, &self.state, next)?;
            let pub_hash = object_hash("publication", &publication)?;
            let mut publication_receipts = Vec::new();
            for (index, d) in self.members().into_iter().enumerate() {
                store_object(&d, "publication", &publication)?;
                publication_receipts.push(append(&d, "PUBLICATION", c.sequence, hash, &pub_hash)?);
                if publication.recovery.is_some() {
                    self.failpoint(&format!("after_recovery_publication_{}", index + 1))?;
                }
            }
            let seal = PublicationSeal {
                candidate_hash: hash.into(),
                publication_hash: pub_hash,
                receipts: publication_receipts.clone(),
            };
            verify_publication_seal(c, &publication, &seal)?;
            for d in self.members() {
                write_json(&publication_seal_path(&d, hash), &seal, false)?;
            }
            // Historical success is preserved even if bytes were later purged or lost.
            // Reconstruct only its original proof, never claim a newly verified payload.
            for evidence in &c.versions {
                for slot in &evidence.placement.route_slot_ids {
                    let d = self.disk(slot)?;
                    if validate_seal(&self.config, d).is_err() {
                        continue;
                    }
                    let dir = version_dir(
                        d,
                        &evidence.version.history_tree_id,
                        &evidence.version.version_id,
                    );
                    if !dir.exists() {
                        continue;
                    }
                    let proof = CommitProof {
                        candidate_hash: hash.into(),
                        finalize: publication.finalize.clone(),
                        version_hash: object_hash("version", &evidence.version)?,
                        disk_id: d.disk_id.clone(),
                        slot_id: slot.clone(),
                        route_slot_ids: evidence.placement.route_slot_ids.clone(),
                    };
                    let expected = publication.payload_receipts.iter().find(|r| {
                        r.version_id == evidence.version.version_id && r.slot_id == *slot
                    });
                    let Some(expected) = expected else {
                        if publication.recovery.is_some() {
                            continue;
                        }
                        return Err(Error::new(
                            "PUBLICATION_INVALID",
                            "stored local receipt missing",
                        ));
                    };
                    if expected.proof_hash != object_hash("commit-proof", &proof)? {
                        return Err(Error::new(
                            "PUBLICATION_INVALID",
                            "stored local receipt differs",
                        ));
                    }
                    let proof_dir = local_proof_directory(&dir, hash)?;
                    write_json(&proof_dir.join("commit-proof.json"), &proof, false)?;
                    write_json(
                        &proof_dir.join("publication-proof.json"),
                        &LocalPublication {
                            candidate: c.clone(),
                            publication: publication.clone(),
                            commit_proof: proof,
                            publication_receipts: publication_receipts.clone(),
                        },
                        false,
                    )?;
                }
            }
            return Ok(());
        }
        let members = self.members();
        let mut accepts = Vec::new();
        for (i, d) in members.iter().enumerate() {
            validate_seal(&self.config, d)?;
            accepts.push(append(d, "ACCEPT", c.sequence, hash, hash)?);
            self.failpoint(&format!("after_accept_{}", i + 1))?;
        }
        let finalize = Finalize {
            candidate_hash: hash.into(),
            accepts,
            recovery_id: None,
        };
        let final_hash = object_hash("finalize", &finalize)?;
        let mut final_receipts = Vec::new();
        for d in &members {
            store_object(d, "finalize", &finalize)?;
            final_receipts.push(append(d, "FINALIZE", c.sequence, hash, &final_hash)?);
        }
        self.failpoint("after_finalize")?;
        // Only a decided candidate may rename a published file container.
        // A crash leaves the operation indeterminate and recovery replays it.
        crate::container_storage::synchronize_names(&self.config, &self.state, next)?;
        self.failpoint("after_container_names")?;
        let mut payload_receipts = Vec::new();
        for evidence in &c.versions {
            for slot in &evidence.placement.route_slot_ids {
                let d = self.disk(slot)?;
                validate_seal(&self.config, d)?;
                let directory = version_dir(
                    d,
                    &evidence.version.history_tree_id,
                    &evidence.version.version_id,
                );
                if let Some(verified) = verified {
                    verified.validate_payload(&self.config, &evidence.version, slot)?;
                } else {
                    // Recovery has no live held-inode proof and must re-read all bytes.
                    verify_payload(&directory, &evidence.version)?;
                }
                let proof = CommitProof {
                    candidate_hash: hash.into(),
                    finalize: finalize.clone(),
                    version_hash: object_hash("version", &evidence.version)?,
                    disk_id: d.disk_id.clone(),
                    slot_id: d.slot_id.clone(),
                    route_slot_ids: evidence.placement.route_slot_ids.clone(),
                };
                let proof_dir = local_proof_directory(&directory, hash)?;
                write_json(&proof_dir.join("commit-proof.json"), &proof, false)?;
                payload_receipts.push(PayloadReceipt {
                    version_id: evidence.version.version_id.clone(),
                    disk_id: d.disk_id.clone(),
                    slot_id: d.slot_id.clone(),
                    proof_hash: object_hash("commit-proof", &proof)?,
                });
            }
        }
        payload_receipts
            .sort_by(|a, b| (&a.version_id, &a.disk_id).cmp(&(&b.version_id, &b.disk_id)));
        self.failpoint("after_commit_proofs")?;
        let publication = Publication {
            recovery: None,
            candidate_hash: hash.into(),
            finalize,
            finalize_receipts: final_receipts,
            payload_receipts,
            result: next
                .operations
                .get(&c.operation_id)
                .cloned()
                .ok_or_else(|| Error::new("STATE_INVALID", "candidate result is absent"))?,
        };
        let pub_hash = object_hash("publication", &publication)?;
        let mut publication_receipts = Vec::new();
        for (i, d) in members.iter().enumerate() {
            store_object(d, "publication", &publication)?;
            publication_receipts.push(append(d, "PUBLICATION", c.sequence, hash, &pub_hash)?);
            self.failpoint(&format!("after_publication_{}", i + 1))?;
        }
        let seal = PublicationSeal {
            candidate_hash: hash.into(),
            publication_hash: pub_hash,
            receipts: publication_receipts.clone(),
        };
        for d in &members {
            write_json(&publication_seal_path(d, hash), &seal, false)?;
        }
        for evidence in &c.versions {
            for slot in &evidence.placement.route_slot_ids {
                let d = self.disk(slot)?;
                let directory = version_dir(
                    d,
                    &evidence.version.history_tree_id,
                    &evidence.version.version_id,
                );
                let proof_dir = local_proof_directory(&directory, hash)?;
                let proof = read_json(&proof_dir.join("commit-proof.json"))?;
                write_json(
                    &proof_dir.join("publication-proof.json"),
                    &LocalPublication {
                        candidate: c.clone(),
                        publication: publication.clone(),
                        commit_proof: proof,
                        publication_receipts: publication_receipts.clone(),
                    },
                    false,
                )?;
            }
        }
        self.failpoint("after_local_publications")?;
        Ok(())
    }
}

/// Bounded scoped workers are fully joined before returning, even on failure.
/// Detached preparation may run without the engine mutex; decision follows all joins.
fn run_payload_groups<T: Sync>(
    groups: &[Vec<&T>],
    work: impl Fn(&T) -> Result<()> + Sync,
) -> Result<()> {
    const WORKERS: usize = 4;
    for batch in groups.chunks(WORKERS) {
        std::thread::scope(|scope| {
            let work = &work;
            let workers: Vec<_> = batch
                .iter()
                .map(|group| {
                    scope.spawn(move || {
                        for payload in group {
                            work(payload)?;
                        }
                        Ok(())
                    })
                })
                .collect();
            let mut first_error = None;
            for worker in workers {
                let result = worker.join().unwrap_or_else(|_| {
                    Err(Error::new(
                        "INTERNAL_ERROR",
                        "payload preparation worker panicked",
                    ))
                });
                if first_error.is_none() {
                    first_error = result.err();
                }
            }
            first_error.map_or(Ok(()), Err)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod parallel_preparation_tests {
    use super::*;
    use std::sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn independent_payload_workers_are_bounded_and_file_order_is_serial() {
        let jobs: Vec<_> = (0..8).flat_map(|file| [(file, 0), (file, 1)]).collect();
        let groups: Vec<_> = jobs.chunks(2).map(|jobs| jobs.iter().collect()).collect();
        let progress: Vec<_> = (0..8).map(|_| AtomicUsize::new(0)).collect();
        let active = AtomicUsize::new(0);
        let maximum = AtomicUsize::new(0);
        let barrier = Barrier::new(4);
        run_payload_groups(&groups, |&(file, step)| {
            assert_eq!(progress[file].fetch_add(1, Ordering::SeqCst), step);
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(now, Ordering::SeqCst);
            barrier.wait();
            active.fetch_sub(1, Ordering::SeqCst);
            barrier.wait();
            Ok(())
        })
        .unwrap();
        assert_eq!(maximum.load(Ordering::SeqCst), 4);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert!(
            progress
                .iter()
                .all(|count| count.load(Ordering::SeqCst) == 2)
        );
    }
}
