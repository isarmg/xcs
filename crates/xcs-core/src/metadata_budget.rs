//! Preflight the exact JSON shapes that publication will persist. Hash values
//! occupy exactly 64 ASCII bytes; every other value comes from the real candidate.
use crate::{degraded_recovery::RecoveryProof, disk::*, journal::*, *};

fn receipt(
    disk: &DiskConfig,
    kind: &str,
    candidate: &Candidate,
    hash: &str,
    object: &str,
) -> Receipt {
    Receipt {
        disk_id: disk.disk_id.clone(),
        slot_id: disk.slot_id.clone(),
        frame_hash: "0".repeat(64),
        record: Record {
            record_type: kind.into(),
            sequence: candidate.sequence,
            candidate_hash: hash.into(),
            object_hash: object.into(),
            previous_frame_hash: "0".repeat(64),
        },
    }
}
/// No future variable-size data is guessed: receipt hashes have fixed hex width,
/// sequence numbers are this candidate's actual decimal strings, and member/slot
/// identifiers, page references, result fields and recovery records are real.
/// The returned bound is exact for the persisted proof JSON shapes.
pub(crate) fn preflight_publication(
    engine: &Engine,
    candidate: &Candidate,
    result: &OperationResult,
    state: &State,
    mut recovery: Option<RecoveryProof>,
    maximum: usize,
) -> Result<usize> {
    let mut largest = 0usize;
    let mut check = |bytes: Vec<u8>| {
        largest = largest.max(bytes.len());
    };
    check(encode_metadata(candidate, maximum)?);
    let hash = object_hash("candidate", candidate)?;
    let members = engine.members();
    let accepts = if let Some(recovery) = &mut recovery {
        check(encode_metadata(&recovery.plan, maximum)?);
        let plan_hash = object_hash("recovery-plan", &recovery.plan)?;
        recovery.reprotect = members
            .iter()
            .map(|d| receipt(d, "REPROTECT", candidate, &hash, &plan_hash))
            .collect();
        recovery.plan.accepts().to_vec()
    } else {
        members
            .iter()
            .map(|d| receipt(d, "ACCEPT", candidate, &hash, &hash))
            .collect()
    };
    let finalize = Finalize {
        candidate_hash: hash.clone(),
        accepts,
        recovery_id: recovery.as_ref().map(|r| r.plan.recovery_id.clone()),
    };
    check(encode_metadata(&finalize, maximum)?);
    let finalize_hash = object_hash("finalize", &finalize)?;
    let finalize_receipts = members
        .iter()
        .map(|d| receipt(d, "FINALIZE", candidate, &hash, &finalize_hash))
        .collect();
    let mut proofs = Vec::new();
    let mut payload_receipts = Vec::new();
    for evidence in &candidate.versions {
        for slot in &evidence.placement.route_slot_ids {
            if let Some(recovery) = &recovery {
                let healthy = recovery
                    .plan
                    .version_protection
                    .iter()
                    .find(|v| v["version_id"] == evidence.version.version_id)
                    .and_then(|v| v["slots"].as_array())
                    .is_some_and(|slots| {
                        slots
                            .iter()
                            .any(|s| s["slot_id"] == *slot && s["state"] == "COMPLETE_VERIFIED")
                    });
                if !healthy {
                    continue;
                }
            }
            let disk = engine.disk(slot)?;
            let proof = CommitProof {
                candidate_hash: hash.clone(),
                finalize: finalize.clone(),
                version_hash: object_hash("version", &evidence.version)?,
                disk_id: disk.disk_id.clone(),
                slot_id: slot.clone(),
                route_slot_ids: evidence.placement.route_slot_ids.clone(),
            };
            check(encode_metadata(&proof, maximum)?);
            payload_receipts.push(PayloadReceipt {
                version_id: evidence.version.version_id.clone(),
                disk_id: disk.disk_id.clone(),
                slot_id: slot.clone(),
                proof_hash: object_hash("commit-proof", &proof)?,
            });
            proofs.push(proof);
        }
    }
    let publication = Publication {
        candidate_hash: hash.clone(),
        finalize,
        finalize_receipts,
        payload_receipts,
        result: result.clone(),
        recovery,
    };
    check(encode_metadata(&publication, maximum)?);
    let publication_hash = object_hash("publication", &publication)?;
    let publication_receipts = members
        .iter()
        .map(|d| receipt(d, "PUBLICATION", candidate, &hash, &publication_hash))
        .collect::<Vec<_>>();
    let seal = PublicationSeal {
        candidate_hash: hash.clone(),
        publication_hash,
        receipts: publication_receipts.clone(),
    };
    check(encode_metadata(&seal, maximum)?);
    let package_size =
        crate::history_package::package_proof_bytes(candidate, &publication, &seal, maximum)?;
    if state
        .content_states
        .values()
        .any(|status| matches!(status.as_str(), "PURGED" | "MIGRATED"))
    {
        // A decided pre-upgrade candidate may bind legacy fixed-count pages.
        // Degraded recovery repairs its exact dependencies before preflight;
        // never re-partition that root when sizing its offline tombstones.
        let recovery_disk = if publication.recovery.is_some() {
            Some(members.first().ok_or_else(|| {
                Error::new("RECOVERY_REQUIRED", "recovery metadata member missing")
            })?)
        } else {
            None
        };
        let snapshot = if let Some(disk) = recovery_disk {
            load_object(disk, "snapshot", &candidate.snapshot_hash)?
        } else {
            snapshot_descriptor(state, maximum)?
        };
        let states = serde_json::to_value(&state.content_states)?;
        let states = states
            .as_object()
            .ok_or_else(|| Error::new("STATE_INVALID", "content states are not a map"))?;
        let pages: Box<
            dyn Iterator<Item = Result<serde_json::Map<String, serde_json::Value>>> + '_,
        > = if let Some(disk) = recovery_disk {
            Box::new(
                snapshot
                    .pages
                    .get("content_states")
                    .into_iter()
                    .flatten()
                    .map(|hash| load_object(disk, "state-page", hash)),
            )
        } else {
            Box::new(state_pages(states).map(Ok))
        };
        for page in pages {
            let page = page?;
            // Only pages containing a tombstone can be embedded by cleanup.
            let mut longest = None;
            for (id, status) in page.iter().filter(|(_, status)| {
                status
                    .as_str()
                    .is_some_and(|s| matches!(s, "PURGED" | "MIGRATED"))
            }) {
                // These are the only two fields that differ between tombstones
                // embedding this exact page. Compare their encoded lengths.
                let size = encode_metadata(
                    &serde_json::json!({"version_id":id,"state":status}),
                    maximum,
                )?
                .len();
                if longest
                    .as_ref()
                    .is_none_or(|(previous, _, _)| size > *previous)
                {
                    longest = Some((size, id, status));
                }
            }
            let Some((_, id, status)) = longest else {
                continue;
            };
            check(encode_metadata(
                &ContentTombstone {
                    version_id: id.clone(),
                    state: status.as_str().expect("tombstone checked").into(),
                    candidate: candidate.clone(),
                    publication: publication.clone(),
                    publication_seal: seal.clone(),
                    snapshot: snapshot.clone(),
                    content_state_page: page,
                },
                maximum,
            )?);
        }
    }
    for proof in proofs {
        check(encode_metadata(
            &LocalPublication {
                candidate: candidate.clone(),
                publication: publication.clone(),
                commit_proof: proof,
                publication_receipts: publication_receipts.clone(),
            },
            maximum,
        )?);
    }
    Ok(largest.max(package_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Engine, Repository) {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("p"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&temp.path().join("state"), config).unwrap();
        let repo = engine
            .create_repository("容量边界", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        (temp, engine, repo)
    }
    #[test]
    fn manifest_limit_rejects_before_any_new_payload_or_decision_and_keeps_published_state() {
        let (_temp, mut engine, repo) = fixture();
        let before = canonical_json(&engine.state).unwrap();
        let snapshot = engine.snapshot.clone();
        let next_slot = engine.next_slot;
        let mut next = engine.state.clone();
        next.sequence = next_slot;
        // A realistic next tree grows the history while preserving the original snapshot.
        let original = next.commits[&next.branches[&repo.main_branch_id].head_commit_id].clone();
        let source = engine.root.join("unpublished-full-source");
        std::fs::write(&source, b"prepared complete bytes").unwrap();
        let mut pending = Vec::new();
        let version = engine
            .build_version(
                &mut next,
                &mut pending,
                &repo.history_tree_id,
                &new_id("file"),
                "pending.txt",
                &source,
                "P01",
                3,
                Vec::new(),
                false,
            )
            .unwrap();
        let mut commit = original.clone();
        commit.commit_id = new_id("commit");
        commit.parent_commit_ids = vec![original.commit_id];
        commit.entries.push(Entry {
            entry_id: new_id("entry"),
            path: "pending.txt".into(),
            name: "pending.txt".into(),
            lookup_key: "pending.txt".into(),
            kind: "file".into(),
            file_id: version.file_id.clone(),
            version_id: version.version_id.clone(),
            size: version.logical_size,
        });
        commit.kind = "content".into();
        commit.sequence = next_slot;
        next.branches
            .get_mut(&repo.main_branch_id)
            .unwrap()
            .head_commit_id = commit.commit_id.clone();
        next.commits.insert(commit.commit_id.clone(), commit);
        let mut result = next.operations.values().next().unwrap().clone();
        result.operation_id = new_id("op");
        result.sequence = next_slot;
        next.operations
            .insert(result.operation_id.clone(), result.clone());
        // Derive the exact successful boundary using the same serializer.
        let mut high = 1usize;
        while crate::history_package::store_history_manifests_bounded(&[], &next, high).is_err() {
            high *= 2;
        }
        let mut low = 0usize;
        while low + 1 < high {
            let mid = low + (high - low) / 2;
            if crate::history_package::store_history_manifests_bounded(&[], &next, mid).is_ok() {
                high = mid;
            } else {
                low = mid;
            }
        }
        assert!(crate::history_package::store_history_manifests_bounded(&[], &next, high).is_ok());
        assert!(
            store_snapshot_bounded(&[], &next, high - 1).is_ok(),
            "the manifest itself must be the limiting object"
        );
        let logs = engine
            .members()
            .iter()
            .map(|d| fs_bytes(&log_path(d)))
            .collect::<Vec<_>>();
        let error = engine
            .persist_with_metadata_limit(next, pending, result.clone(), high - 1)
            .unwrap_err();
        assert_eq!(error.code(), "RESOURCE_LIMIT");
        assert_eq!(canonical_json(&engine.state).unwrap(), before);
        assert_eq!(engine.snapshot, snapshot);
        assert_eq!(engine.next_slot, next_slot);
        assert!(!engine.blocked);
        for disk in &engine.config.disks {
            assert!(!version_dir(disk, &repo.history_tree_id, &version.version_id).exists());
        }
        assert_eq!(
            engine.operation(&result.operation_id).unwrap_err().code(),
            "OPERATION_NOT_FOUND"
        );
        assert_eq!(
            engine
                .members()
                .iter()
                .map(|d| fs_bytes(&log_path(d)))
                .collect::<Vec<_>>(),
            logs
        );
    }
    fn fs_bytes(path: &std::path::Path) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }
    #[test]
    fn actual_publication_and_local_proofs_fit_the_exact_preflight_boundary() {
        let (_temp, mut engine, repo) = fixture();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "alice")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "file.txt", b"whole bytes")
            .unwrap();
        let result = engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "save".into(),
                request_digest: None,
            })
            .unwrap();
        let disk = engine.members()[0].clone();
        let scanned = scan(&disk).unwrap();
        let (record, _) = scanned.records.last().unwrap();
        let candidate: Candidate = load_object(&disk, "candidate", &record.candidate_hash).unwrap();
        let publication: Publication =
            load_object(&disk, "publication", &record.object_hash).unwrap();
        let largest = preflight_publication(
            &engine,
            &candidate,
            &result,
            &engine.state,
            None,
            MAX_METADATA,
        )
        .unwrap();
        let mut actual = vec![
            canonical_json(&candidate).unwrap().len(),
            canonical_json(&publication).unwrap().len(),
            canonical_json(&publication.finalize).unwrap().len(),
            fs_bytes(&publication_seal_path(&disk, &record.candidate_hash)).len(),
        ];
        for version in &candidate.versions {
            for slot in &version.placement.route_slot_ids {
                let dir = version_dir(
                    engine.disk(slot).unwrap(),
                    &version.version.history_tree_id,
                    &version.version.version_id,
                );
                actual.push(fs_bytes(&dir.join("commit-proof.json")).len());
                actual.push(fs_bytes(&dir.join("publication-proof.json")).len());
            }
        }
        assert_eq!(largest, *actual.iter().max().unwrap());
        assert_eq!(
            preflight_publication(&engine, &candidate, &result, &engine.state, None, largest)
                .unwrap(),
            largest
        );
        assert_eq!(
            preflight_publication(
                &engine,
                &candidate,
                &result,
                &engine.state,
                None,
                largest - 1
            )
            .unwrap_err()
            .code(),
            "RESOURCE_LIMIT"
        );
        // The actual final sequence occupies all twenty decimal digits.
        let mut wide = candidate;
        wide.sequence = u64::MAX;
        let mut result = result;
        result.sequence = u64::MAX;
        let largest =
            preflight_publication(&engine, &wide, &result, &engine.state, None, MAX_METADATA)
                .unwrap();
        assert!(
            preflight_publication(&engine, &wide, &result, &engine.state, None, largest).is_ok()
        );
        assert!(
            preflight_publication(&engine, &wide, &result, &engine.state, None, largest - 1)
                .is_err()
        );
    }
    fn save(engine: &mut Engine, branch: &str, bytes: &[u8]) -> String {
        let workspace = engine.create_workspace(branch, "alice").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "file.txt", bytes)
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "save".into(),
                request_digest: None,
            })
            .unwrap();
        engine.list_tree(branch).unwrap()[0].version_id.clone()
    }
    #[test]
    fn metadata_only_package_and_purge_tombstone_combinations_are_preflighted() {
        let (_temp, mut engine, repo) = fixture();
        let old = save(&mut engine, &repo.main_branch_id, b"old");
        save(&mut engine, &repo.main_branch_id, b"new");
        let plan = engine
            .create_purge_plan(&repo.history_tree_id, vec![old.clone()], "alice")
            .unwrap();
        engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        let disk = engine.members()[0].clone();
        let scanned = scan(&disk).unwrap();
        let (record, _) = scanned.records.last().unwrap();
        let candidate: Candidate = load_object(&disk, "candidate", &record.candidate_hash).unwrap();
        assert!(candidate.versions.is_empty());
        let publication: Publication =
            load_object(&disk, "publication", &record.object_hash).unwrap();
        let parent = engine.root.join("package-output");
        private_dir(&parent).unwrap();
        let package = parent.join("tree");
        engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        let package_size = fs_bytes(&package.join("publication.json")).len();
        let seal: PublicationSeal =
            read_json(&publication_seal_path(&disk, &record.candidate_hash)).unwrap();
        assert_eq!(
            crate::history_package::package_proof_bytes(
                &candidate,
                &publication,
                &seal,
                MAX_METADATA
            )
            .unwrap(),
            package_size
        );
        let tombstone = version_dir(&disk, &repo.history_tree_id, &old).join("purged.json");
        let tombstone_size = fs_bytes(&tombstone).len();
        let largest = preflight_publication(
            &engine,
            &candidate,
            &publication.result,
            &engine.state,
            None,
            MAX_METADATA,
        )
        .unwrap();
        assert_eq!(largest, package_size.max(tombstone_size));
        assert!(
            preflight_publication(
                &engine,
                &candidate,
                &publication.result,
                &engine.state,
                None,
                largest
            )
            .is_ok()
        );
        assert_eq!(
            preflight_publication(
                &engine,
                &candidate,
                &publication.result,
                &engine.state,
                None,
                largest - 1
            )
            .unwrap_err()
            .code(),
            "RESOURCE_LIMIT"
        );
        assert!(package_size > canonical_json(&publication).unwrap().len());

        // Exercise multiple content-defined pages without hundreds of disk
        // publications. Only the state shape varies here; proof hashes have
        // fixed width, so the existing real publication supplies exact sizes.
        let mut wide_state = engine.state.clone();
        for n in 0u32..600 {
            wide_state.content_states.insert(
                blake3::hash(&n.to_le_bytes()).to_hex().to_string(),
                if n % 3 == 0 { "MIGRATED" } else { "PURGED" }.into(),
            );
        }
        // Keep each modern page below half the legacy cap so this fixture
        // deterministically distinguishes legacy and current tombstone sizes.
        let values = serde_json::to_value(&wide_state.content_states).unwrap();
        let mut in_page = 0;
        for key in values.as_object().unwrap().keys() {
            if blake3::hash(key.as_bytes()).as_bytes()[0] & 31 == 0 {
                in_page = 0;
            }
            in_page += 1;
            if in_page > 64 {
                wide_state.content_states.remove(key);
            }
        }
        let snapshot_hash = store_snapshot(std::slice::from_ref(&disk), &wide_state).unwrap();
        let snapshot: Snapshot = load_object(&disk, "snapshot", &snapshot_hash).unwrap();
        assert!(snapshot.pages["content_states"].len() > 5);
        let mut actual_largest = package_size;
        for hash in &snapshot.pages["content_states"] {
            let page: serde_json::Map<String, serde_json::Value> =
                load_object(&disk, "state-page", hash).unwrap();
            for (id, status) in &page {
                if !matches!(status.as_str(), Some("PURGED" | "MIGRATED")) {
                    continue;
                }
                actual_largest = actual_largest.max(
                    canonical_json(&ContentTombstone {
                        version_id: id.clone(),
                        state: status.as_str().unwrap().into(),
                        candidate: candidate.clone(),
                        publication: publication.clone(),
                        publication_seal: seal.clone(),
                        snapshot: snapshot.clone(),
                        content_state_page: page.clone(),
                    })
                    .unwrap()
                    .len(),
                );
            }
        }
        let bounded = preflight_publication(
            &engine,
            &candidate,
            &publication.result,
            &wide_state,
            None,
            MAX_METADATA,
        )
        .unwrap();
        assert_eq!(bounded, actual_largest);
        assert!(
            preflight_publication(
                &engine,
                &candidate,
                &publication.result,
                &wide_state,
                None,
                bounded,
            )
            .is_ok()
        );
        assert_eq!(
            preflight_publication(
                &engine,
                &candidate,
                &publication.result,
                &wide_state,
                None,
                bounded - 1,
            )
            .unwrap_err()
            .code(),
            "RESOURCE_LIMIT"
        );

        // A pre-upgrade decided candidate must keep its old, larger pages for
        // exact preflight, even though the current writer makes smaller ones.
        let mut legacy = snapshot.clone();
        let states = serde_json::to_value(&wide_state.content_states).unwrap();
        let entries = states.as_object().unwrap().iter().collect::<Vec<_>>();
        let legacy_pages = entries
            .chunks(STATE_PAGE_ENTRIES)
            .map(|chunk| {
                let page = chunk
                    .iter()
                    .map(|(key, value)| ((*key).clone(), (*value).clone()))
                    .collect::<serde_json::Map<_, _>>();
                store_object(&disk, "state-page", &page).unwrap()
            })
            .collect::<Vec<_>>();
        legacy
            .pages
            .insert("content_states".into(), legacy_pages.clone());
        let legacy_hash = store_object(&disk, "snapshot", &legacy).unwrap();
        let mut legacy_candidate = candidate.clone();
        legacy_candidate.snapshot_hash = legacy_hash;
        // Preflight sizes proofs but does not authorize or publish recovery.
        let recovery = RecoveryProof {
            plan: serde_json::from_value(serde_json::json!({
                "recovery_id": "test-recovery", "digest": "0".repeat(64),
                "operation_id": candidate.operation_id, "candidate_hash": "0".repeat(64),
                "principal": "alice", "reason": "size regression", "isolation_generation": "1",
                "sequence": candidate.sequence.to_string(), "version_protection": [],
                "loss_details": [], "members": [], "original_accepts": publication.finalize.accepts,
            }))
            .unwrap(),
            reprotect: Vec::new(),
        };
        let old_bound = preflight_publication(
            &engine,
            &legacy_candidate,
            &publication.result,
            &wide_state,
            Some(recovery.clone()),
            MAX_METADATA,
        )
        .unwrap();
        let mut modern_candidate = candidate.clone();
        modern_candidate.snapshot_hash = snapshot_hash;
        let modern_bound = preflight_publication(
            &engine,
            &modern_candidate,
            &publication.result,
            &wide_state,
            Some(recovery.clone()),
            MAX_METADATA,
        )
        .unwrap();
        assert!(
            old_bound > modern_bound,
            "legacy 128-entry tombstones need the larger bound: old={old_bound}, modern={modern_bound}, pages={:?}",
            state_pages(states.as_object().unwrap())
                .map(|p| p.len())
                .collect::<Vec<_>>()
        );
        for limit in [modern_bound, old_bound - 1] {
            assert_eq!(
                preflight_publication(
                    &engine,
                    &legacy_candidate,
                    &publication.result,
                    &wide_state,
                    Some(recovery.clone()),
                    limit,
                )
                .unwrap_err()
                .code(),
                "RESOURCE_LIMIT"
            );
        }
        assert_eq!(
            preflight_publication(
                &engine,
                &legacy_candidate,
                &publication.result,
                &wide_state,
                Some(recovery.clone()),
                old_bound,
            )
            .unwrap(),
            old_bound
        );
        std::fs::remove_file(object_path(&disk, "state-page", &legacy_pages[0])).unwrap();
        assert!(
            preflight_publication(
                &engine,
                &legacy_candidate,
                &publication.result,
                &wide_state,
                Some(recovery),
                MAX_METADATA,
            )
            .is_err(),
            "missing legacy pages must fail closed, not use a new partition"
        );
    }

    #[test]
    fn snapshot_page_and_common_metadata_encoder_reject_one_byte_over_boundary() {
        let (_temp, engine, _repo) = fixture();
        let state = engine.state.clone();
        let mut high = 1usize;
        while store_snapshot_bounded(&[], &state, high).is_err() {
            high *= 2;
        }
        let mut low = 0usize;
        while low + 1 < high {
            let mid = low + (high - low) / 2;
            if store_snapshot_bounded(&[], &state, mid).is_ok() {
                high = mid
            } else {
                low = mid
            }
        }
        assert!(store_snapshot_bounded(&[], &state, high).is_ok());
        assert_eq!(
            store_snapshot_bounded(&[], &state, high - 1)
                .unwrap_err()
                .code(),
            "RESOURCE_LIMIT"
        );
        let object = serde_json::json!({"bytes":"full"});
        let size = canonical_json(&object).unwrap().len();
        assert_eq!(encode_metadata(&object, size).unwrap().len(), size);
        assert_eq!(
            encode_metadata(&object, size - 1).unwrap_err().code(),
            "RESOURCE_LIMIT"
        );
    }
}
