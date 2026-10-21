use super::*;

#[test]
fn recovery_repairs_exact_candidate_and_publication_replicas() {
    for interrupted in [false, true] {
        for kind in ["candidate", "publication"] {
            for missing in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let root = temp.path().join("system");
                let config = Config::development([
                    temp.path().join("a"),
                    temp.path().join("b"),
                    temp.path().join("c"),
                ]);
                let mut engine = Engine::initialize(&root, config).unwrap();
                if interrupted {
                    engine.inject_fault_once("after_publication_3");
                }
                let result = engine.create_repository("repo", "P01", 3, "admin", &new_id("op"));
                assert_eq!(result.is_err(), interrupted);
                let members = engine.members();
                let record = scan(&members[0])
                    .unwrap()
                    .records
                    .into_iter()
                    .find(|(r, _)| r.record_type == "PUBLICATION")
                    .unwrap()
                    .0;
                let candidate: Candidate =
                    load_object(&members[0], "candidate", &record.candidate_hash).unwrap();
                let expected =
                    canonical_json(&load_snapshot(&members[0], &candidate.snapshot_hash).unwrap())
                        .unwrap();
                let hash = if kind == "candidate" {
                    record.candidate_hash
                } else {
                    record.object_hash
                };
                let target = object_path(&members[0], kind, &hash);
                let original = std::fs::read(&target).unwrap();
                drop(engine);
                if missing {
                    std::fs::remove_file(&target).unwrap();
                } else {
                    std::fs::write(&target, b"corrupt").unwrap();
                }
                let recovered = Engine::open(&root).unwrap();
                assert_eq!(canonical_json(&recovered.state).unwrap(), expected);
                assert_eq!(std::fs::read(&target).unwrap(), original);
            }
        }
    }
}

#[test]
fn pending_candidate_recovers_from_healthy_exact_replica() {
    for fault in ["after_accept_1", "after_accept_2"] {
        for missing in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("system");
            let config = Config::development([
                temp.path().join("a"),
                temp.path().join("b"),
                temp.path().join("c"),
            ]);
            let mut engine = Engine::initialize(&root, config).unwrap();
            engine.inject_fault_once(fault);
            let operation = new_id("op");
            assert!(
                engine
                    .create_repository("repo", "P01", 3, "admin", &operation)
                    .is_err()
            );
            let members = engine.members();
            let record = scan(&members[0])
                .unwrap()
                .records
                .into_iter()
                .find(|(r, _)| r.record_type == "ACCEPT")
                .unwrap()
                .0;
            let target = object_path(&members[0], "candidate", &record.candidate_hash);
            let original = std::fs::read(&target).unwrap();
            drop(engine);
            if missing {
                std::fs::remove_file(&target).unwrap();
            } else {
                std::fs::write(&target, b"corrupt").unwrap();
            }
            let recovered = Engine::open(&root).unwrap();
            assert_eq!(
                recovered.operation(&operation).unwrap().commit_state,
                storage_contract::CommitState::Published
            );
            assert_eq!(recovered.state.repositories.len(), 1);
            assert_eq!(std::fs::read(&target).unwrap(), original);
            drop(recovered);
            assert_eq!(Engine::open(&root).unwrap().state.repositories.len(), 1);
        }
    }
}

#[test]
fn recovery_rejects_all_digest_mismatched_object_replicas_without_rewriting_them() {
    for kind in ["candidate", "publication"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("system");
        let config = Config::development([
            temp.path().join("a"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&root, config).unwrap();
        engine
            .create_repository("repo", "P01", 3, "admin", &new_id("op"))
            .unwrap();
        let members = engine.members();
        let record = scan(&members[0])
            .unwrap()
            .records
            .into_iter()
            .find(|(r, _)| r.record_type == "PUBLICATION")
            .unwrap()
            .0;
        let hash = if kind == "candidate" {
            record.candidate_hash
        } else {
            record.object_hash
        };
        let mut wrong: serde_json::Value =
            read_json(&object_path(&members[0], kind, &hash)).unwrap();
        if kind == "candidate" {
            wrong["operation_id"] = serde_json::json!(new_id("op"));
        } else {
            wrong["candidate_hash"] = serde_json::json!("0".repeat(64));
        }
        let wrong = canonical_json(&wrong).unwrap();
        let journals = members
            .iter()
            .map(|d| std::fs::read(log_path(d)).unwrap())
            .collect::<Vec<_>>();
        drop(engine);
        for member in &members {
            std::fs::write(object_path(member, kind, &hash), &wrong).unwrap();
        }
        assert_eq!(Engine::open(&root).err().unwrap().code, "OBJECT_CORRUPT");
        for (member, journal) in members.iter().zip(journals) {
            assert_eq!(
                std::fs::read(object_path(member, kind, &hash)).unwrap(),
                wrong
            );
            assert_eq!(std::fs::read(log_path(member)).unwrap(), journal);
        }
    }
}

#[test]
fn recovery_rejects_flat_objects_without_repair_or_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("system");
    let config = Config::development([
        temp.path().join("a"),
        temp.path().join("b"),
        temp.path().join("c"),
    ]);
    let mut engine = Engine::initialize(&root, config).unwrap();
    engine
        .create_repository("repo", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let members = engine.members();
    let record = scan(&members[0])
        .unwrap()
        .records
        .into_iter()
        .find(|(r, _)| r.record_type == "PUBLICATION")
        .unwrap()
        .0;
    let hash = record.candidate_hash;
    let expected = std::fs::read(object_path(&members[0], "candidate", &hash)).unwrap();
    let flat = members[0]
        .root
        .join("metadata/objects/candidate")
        .join(format!("{hash}.json"));
    std::fs::write(&flat, b"{}").unwrap();
    assert!(load_object::<Candidate>(&members[0], "candidate", &hash).is_err());
    drop(engine);
    assert_eq!(
        Engine::open(&root).err().unwrap().code(),
        "FORMAT_UNSUPPORTED"
    );
    assert_eq!(std::fs::read(flat).unwrap(), b"{}");
    assert_eq!(
        std::fs::read(object_path(&members[0], "candidate", &hash)).unwrap(),
        expected
    );
}

#[test]
fn object_repair_does_not_replace_symlink_or_hardlink() {
    use std::os::unix::fs::symlink;
    for hardlink in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("a"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let engine = Engine::initialize(&temp.path().join("system"), config).unwrap();
        let members = engine.members();
        let value = serde_json::json!({"data": "verified"});
        let hash = store_object(&members[0], "candidate", &value).unwrap();
        let target = object_path(&members[0], "candidate", &hash);
        std::fs::remove_file(&target).unwrap();
        let external = temp.path().join("external");
        std::fs::write(&external, b"untouched").unwrap();
        if hardlink {
            std::fs::hard_link(&external, &target).unwrap();
        } else {
            symlink(&external, &target).unwrap();
        }
        assert!(repair_object_copies(&members, "candidate", &hash, &value).is_err());
        assert_eq!(std::fs::read(&external).unwrap(), b"untouched");
        assert_eq!(std::fs::read(&target).unwrap(), b"untouched");
    }
}

#[test]
fn bound_snapshot_repair_preserves_alternate_page_partition() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::development([
        temp.path().join("a"),
        temp.path().join("b"),
        temp.path().join("c"),
    ]);
    let engine = Engine::initialize(&temp.path().join("system"), config).unwrap();
    let members = engine.members();
    let mut state = State::default();
    for n in 0..300 {
        state
            .file_notes
            .insert(format!("file_{n:04}"), format!("note {n}"));
    }
    let value = serde_json::to_value(&state).unwrap();
    let mut pages = BTreeMap::new();
    let mut scalars = serde_json::Map::new();
    // Page partitioning is not a version discriminator. Repair must retain
    // the bound root rather than regenerate content-defined boundaries.
    for (field, value) in value.as_object().unwrap() {
        if let Some(map) = value.as_object() {
            let entries = map.iter().collect::<Vec<_>>();
            let mut hashes = Vec::new();
            for chunk in entries.chunks(128) {
                let page: serde_json::Map<_, _> = chunk
                    .iter()
                    .map(|(key, value)| ((*key).clone(), (*value).clone()))
                    .collect();
                hashes.push(store_object(&members[0], "state-page", &page).unwrap());
            }
            pages.insert(field.clone(), hashes);
        } else {
            scalars.insert(field.clone(), value.clone());
        }
    }
    scalars.remove("events");
    pages.insert("events".into(), Vec::new());
    let alternate = Snapshot {
        sequence: state.sequence,
        pages,
        scalars,
    };
    let hash = store_object(&members[0], "snapshot", &alternate).unwrap();
    let modern_hash = store_snapshot(&[], &state).unwrap();
    assert_ne!(
        hash, modern_hash,
        "fixture must distinguish alternate/current page partitions"
    );
    engine.repair_bound_snapshot(&hash).unwrap();
    for member in &members {
        assert_eq!(
            canonical_json(&load_snapshot(member, &hash).unwrap()).unwrap(),
            canonical_json(&state).unwrap()
        );
        assert!(!object_path(member, "snapshot", &modern_hash).exists());
        assert_eq!(
            std::fs::read(object_path(member, "snapshot", &hash)).unwrap(),
            std::fs::read(object_path(&members[0], "snapshot", &hash)).unwrap()
        );
    }
}

#[test]
fn repair_cache_is_bounded_and_overflow_remains_uncached() {
    let mut repaired = BTreeSet::new();
    for n in 0..MAX_REPAIRED_PAGES {
        remember_repaired_page(&mut repaired, &format!("{n:064x}"));
    }
    let overflow = format!("{:064x}", MAX_REPAIRED_PAGES);
    remember_repaired_page(&mut repaired, &overflow);
    assert_eq!(repaired.len(), MAX_REPAIRED_PAGES);
    assert!(!repaired.contains(&overflow));
    assert!(repaired.contains(&format!("{:064x}", 0)));
}

#[test]
fn failed_replica_repair_does_not_cache_page_success() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::development([
        temp.path().join("a"),
        temp.path().join("b"),
        temp.path().join("c"),
    ]);
    let engine = Engine::initialize(&temp.path().join("system"), config).unwrap();
    let members = engine.members();
    let mut state = State::default();
    state
        .file_notes
        .insert("file_test".into(), "unchanged".into());
    let hash = store_snapshot(&members, &state).unwrap();
    let snapshot: Snapshot = load_object(&members[0], "snapshot", &hash).unwrap();
    let page = snapshot.pages.values().flatten().next().unwrap();
    let target = object_path(&members[2], "state-page", page);
    let original = std::fs::read(&target).unwrap();
    std::fs::remove_file(&target).unwrap();
    std::fs::create_dir(&target).unwrap();
    let mut repaired = BTreeSet::new();
    assert_eq!(
        repair_snapshot_copies(&members, &members, &hash, &mut repaired)
            .unwrap_err()
            .code,
        "UNSAFE_PATH"
    );
    assert!(!repaired.contains(page));
    std::fs::remove_dir(&target).unwrap();
    repair_snapshot_copies(&members, &members, &hash, &mut repaired).unwrap();
    assert!(repaired.contains(page));
    assert_eq!(std::fs::read(&target).unwrap(), original);
}

#[test]
fn shared_pages_are_repaired_once_per_recovery_and_rechecked_next_time() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("system");
    let config = Config::development([
        temp.path().join("a"),
        temp.path().join("b"),
        temp.path().join("c"),
    ]);
    let mut engine = Engine::initialize(&root, config).unwrap();
    engine
        .create_repository("files", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let branch = engine
        .state
        .repositories
        .values()
        .next()
        .unwrap()
        .main_branch_id
        .clone();
    let workspace = engine.create_workspace(&branch, "admin").unwrap();
    engine
        .stage_bytes(
            &workspace.workspace_id,
            "shared.txt",
            b"retained historical content",
        )
        .unwrap();
    engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "admin".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "shared page fixture".into(),
            request_digest: None,
        })
        .unwrap();
    for n in 0..6 {
        engine
            .create_repository(&format!("repo{n}"), "P01", 3, "admin", &new_id("op"))
            .unwrap();
    }
    let members = engine.members();
    let expected = canonical_json(&engine.state).unwrap();
    drop(engine);

    let mut references = 0;
    let mut pages = BTreeSet::new();
    let mut shared = None;
    for (record, _) in scan(&members[0]).unwrap().records {
        if record.record_type != "PUBLICATION" {
            continue;
        }
        let candidate: Candidate =
            load_object(&members[0], "candidate", &record.candidate_hash).unwrap();
        let snapshot: Snapshot =
            load_object(&members[0], "snapshot", &candidate.snapshot_hash).unwrap();
        for hash in snapshot.pages.values().flatten() {
            references += 1;
            if !pages.insert(hash.clone()) {
                shared = Some(hash.clone());
            }
        }
    }
    let shared = shared.expect("fixture must reference an unchanged historical page");
    let path = object_path(&members[0], "state-page", &shared);
    let original = std::fs::read(&path).unwrap();
    assert!(references > pages.len());
    // Both recovery invocations must independently verify the damaged page:
    // an in-memory cache may not turn into persisted trust after restart.
    for corrupt in [false, true] {
        for member in &members {
            let checkpoint = member.root.join("metadata/replay-checkpoint.json");
            if checkpoint.exists() {
                std::fs::remove_file(checkpoint).unwrap();
            }
        }
        if corrupt {
            std::fs::write(&path, b"corrupt").unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
        PAGE_REPAIR_COUNT.with(|count| count.set(0));
        let engine = Engine::open(&root).unwrap();
        assert_eq!(canonical_json(&engine.state).unwrap(), expected);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let repairs = PAGE_REPAIR_COUNT.with(|count| count.get());
        assert_eq!(repairs, pages.len());
        assert!(repairs < references);
        eprintln!("repaired {repairs} unique pages instead of {references} references");
        drop(engine);
    }
}
