use super::*;
fn fixture() -> (tempfile::TempDir, Engine, Candidate) {
    fixture_with_files(1)
}
fn fixture_with_files(count: usize) -> (tempfile::TempDir, Engine, Candidate) {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::development([
        temp.path().join("p"),
        temp.path().join("b"),
        temp.path().join("t"),
    ]);
    let mut engine = Engine::initialize(&temp.path().join("state"), config).unwrap();
    let repo = engine
        .create_repository("proof", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repo.main_branch_id, "admin")
        .unwrap();
    for index in 0..count {
        engine
            .stage_bytes(
                &workspace.workspace_id,
                &format!("body-{index}.txt"),
                b"original",
            )
            .unwrap();
    }
    let result = engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "admin".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "proof".into(),
            request_digest: None,
        })
        .unwrap();
    let candidate = Candidate {
        pool_id: engine.config.pool_id.clone(),
        sequence: engine.state.sequence,
        transaction_id: new_id("tx"),
        operation_id: result.operation_id,
        request_digest: result.request_digest,
        previous_snapshot: None,
        snapshot_hash: engine.snapshot.clone().unwrap(),
        versions: engine
            .state
            .versions
            .values()
            .map(|version| VersionEvidence {
                version: version.clone(),
                placement: engine.state.placements[&version.version_id].clone(),
            })
            .collect(),
        metadata_disk_ids: engine
            .members()
            .iter()
            .map(|disk| disk.disk_id.clone())
            .collect(),
        history_manifest_hashes: Default::default(),
    };
    (temp, engine, candidate)
}
#[test]
fn large_candidate_uses_bounded_full_revalidation_and_rejects_changes() {
    let (_temp, mut engine, candidate) = fixture_with_files(MAX_HELD_PAYLOAD_FILES / 12 + 1);
    let proof = VerifiedPayloadSet::prepare(&engine.config, &candidate).unwrap();
    assert!(proof.full_revalidation);
    assert!(proof.entries.is_empty());
    assert_eq!(proof.disks.len(), 3);
    proof.validate(&engine.config, &candidate).unwrap();
    let mut next = engine.state.clone();
    next.sequence = engine.next_slot;
    let prepared = candidate
        .versions
        .iter()
        .map(|evidence| {
            let disk = engine.disk(&evidence.placement.route_slot_ids[0]).unwrap();
            let directory = version_dir(
                disk,
                &evidence.version.history_tree_id,
                &evidence.version.version_id,
            );
            crate::engine::Prepared {
                version: evidence.version.clone(),
                placement: evidence.placement.clone(),
                source: crate::container_storage::content_path(&directory, &evidence.version)
                    .unwrap(),
            }
        })
        .collect();
    let result = engine.state.operations[&candidate.operation_id].clone();
    let detached = engine
        .prepare_publication(next, prepared, result, MAX_METADATA)
        .unwrap();
    let evidence = &candidate.versions[0];
    let slot = &evidence.placement.route_slot_ids[0];
    let disk = engine.disk(slot).unwrap();
    let directory = version_dir(
        disk,
        &evidence.version.history_tree_id,
        &evidence.version.version_id,
    );
    let body = crate::container_storage::content_path(&directory, &evidence.version).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&body, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&body, b"tampered").unwrap();
    assert!(proof.validate(&engine.config, &candidate).is_err());
    assert!(
        proof
            .validate_payload(&engine.config, &evidence.version, slot)
            .is_err()
    );
    let sequence = engine.state.sequence;
    let next_slot = engine.next_slot;
    assert_eq!(
        engine.decide_publication(detached).unwrap_err().code(),
        "PREPARATION_STALE"
    );
    assert_eq!(engine.state.sequence, sequence);
    assert_eq!(engine.next_slot, next_slot);
    assert!(!engine.blocked);
}
#[test]
fn held_proof_rejects_in_place_content_changes() {
    let (_temp, engine, candidate) = fixture();
    let proof = VerifiedPayloadSet::prepare(&engine.config, &candidate).unwrap();
    proof.validate(&engine.config, &candidate).unwrap();
    let evidence = &candidate.versions[0];
    let disk = engine.disk(&evidence.placement.route_slot_ids[0]).unwrap();
    let dir = version_dir(
        disk,
        &evidence.version.history_tree_id,
        &evidence.version.version_id,
    );
    let body = crate::container_storage::content_path(&dir, &evidence.version).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&body, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(body, b"tampered").unwrap();
    assert_eq!(
        proof
            .validate(&engine.config, &candidate)
            .unwrap_err()
            .code(),
        "PREPARATION_STALE"
    );
}
#[test]
fn held_proof_rejects_replaced_checksum_inode() {
    let (_temp, engine, candidate) = fixture();
    let proof = VerifiedPayloadSet::prepare(&engine.config, &candidate).unwrap();
    let entry = &proof.entries[0];
    let path = entry.directory.join("checksums.bin");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::rename(&path, path.with_extension("old")).unwrap();
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        proof
            .validate(&engine.config, &candidate)
            .unwrap_err()
            .code(),
        "PREPARATION_STALE"
    );
}
#[test]
fn held_proof_rejects_disk_seal_replacement_and_candidate_changes() {
    let (_temp, engine, candidate) = fixture();
    let proof = VerifiedPayloadSet::prepare(&engine.config, &candidate).unwrap();
    let mut changed = candidate.clone();
    changed.versions[0].version.content_hash = "0".repeat(64);
    assert_eq!(
        proof.validate(&engine.config, &changed).unwrap_err().code(),
        "PREPARATION_STALE"
    );
    let path = proof.disks[0].root.join("disk.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::rename(&path, path.with_extension("old")).unwrap();
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        proof
            .validate(&engine.config, &candidate)
            .unwrap_err()
            .code(),
        "PREPARATION_STALE"
    );
}
#[test]
fn detached_decision_rejects_changed_topology_before_journaling() {
    let (_temp, mut engine, candidate) = fixture();
    let mut next = engine.state.clone();
    next.sequence = engine.next_slot;
    let result = engine.state.operations[&candidate.operation_id].clone();
    let detached = engine
        .prepare_publication(next, Vec::new(), result, MAX_METADATA)
        .unwrap();
    let sequence = engine.state.sequence;
    let slot = engine.next_slot;
    engine.config.metadata_reserve_bytes += 1;
    assert_eq!(
        engine.decide_publication(detached).unwrap_err().code(),
        "PREPARATION_STALE"
    );
    assert_eq!(engine.state.sequence, sequence);
    assert_eq!(engine.next_slot, slot);
    assert!(!engine.blocked);
}
#[test]
fn preparation_pins_disk_identity_before_payload_work() {
    let (_temp, engine, candidate) = fixture();
    let preparation = PayloadPreparation::new(&engine.config, &candidate).unwrap();
    let path = preparation.disks[0].root.join("disk.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::rename(&path, path.with_extension("old")).unwrap();
    std::fs::write(&path, bytes).unwrap();
    match preparation.verify(&engine.config, &candidate) {
        Err(error) => assert_eq!(error.code(), "PREPARATION_STALE"),
        Ok(_) => panic!("replaced disk seal was accepted"),
    }
}
