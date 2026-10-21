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
#[cfg(test)]
pub(crate) fn preflight_publication(
    engine: &Engine,
    candidate: &Candidate,
    result: &OperationResult,
    state: &State,
    recovery: Option<RecoveryProof>,
    maximum: usize,
) -> Result<usize> {
    preflight_publication_prepared(engine, candidate, result, state, recovery, maximum, None)
}
/// Recovery sizes tombstones using the candidate's exact stored snapshot graph.
pub(crate) fn preflight_bound_publication(
    engine: &Engine,
    candidate: &Candidate,
    result: &OperationResult,
    state: &State,
    recovery: Option<RecoveryProof>,
    maximum: usize,
) -> Result<usize> {
    preflight_inner(
        engine,
        candidate,
        result,
        state,
        PreflightOptions {
            recovery,
            maximum,
            prepared_snapshot: None,
            bound_snapshot: true,
        },
    )
}
pub(crate) fn preflight_publication_prepared(
    engine: &Engine,
    candidate: &Candidate,
    result: &OperationResult,
    state: &State,
    recovery: Option<RecoveryProof>,
    maximum: usize,
    prepared_snapshot: Option<&Snapshot>,
) -> Result<usize> {
    preflight_inner(
        engine,
        candidate,
        result,
        state,
        PreflightOptions {
            recovery,
            maximum,
            prepared_snapshot,
            bound_snapshot: false,
        },
    )
}
struct PreflightOptions<'a> {
    recovery: Option<RecoveryProof>,
    maximum: usize,
    prepared_snapshot: Option<&'a Snapshot>,
    bound_snapshot: bool,
}

fn preflight_inner(
    engine: &Engine,
    candidate: &Candidate,
    result: &OperationResult,
    state: &State,
    options: PreflightOptions<'_>,
) -> Result<usize> {
    let PreflightOptions {
        mut recovery,
        maximum,
        prepared_snapshot,
        bound_snapshot,
    } = options;
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
        // Decided candidates retain their original snapshot partitioning. Read
        // each exact dependency without repair so preflight precedes mutation.
        let recovering = bound_snapshot || publication.recovery.is_some();
        let snapshot = if recovering {
            crate::publication_evidence::verified_object(
                &members,
                "snapshot",
                &candidate.snapshot_hash,
            )?
        } else if let Some(snapshot) = prepared_snapshot {
            snapshot.clone()
        } else {
            snapshot_descriptor(state, maximum)?
        };
        let states = json_codec::to_value(&state.content_states)?;
        let states = states
            .as_object()
            .ok_or_else(|| Error::new("STATE_INVALID", "content states are not a map"))?;
        let pages: Box<
            dyn Iterator<Item = Result<json_codec::Map<String, json_codec::Value>>> + '_,
        > = if recovering {
            Box::new(
                snapshot
                    .pages
                    .get("content_states")
                    .into_iter()
                    .flatten()
                    .map(|hash| {
                        crate::publication_evidence::verified_object(&members, "state-page", hash)
                    }),
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
                    &json_codec::json!({"version_id":id,"state":status}),
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
#[path = "../../../test/engine/unit/metadata_budget/tests.rs"]
mod tests;
