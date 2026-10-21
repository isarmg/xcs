use super::*;
use sqlx::Row;

fn version(id: &str, file: &str, sequence: u64) -> FileVersion {
    FileVersion {
        history_tree_id: "history".into(),
        version_id: id.into(),
        file_id: file.into(),
        creation_primary_slot_id: "P01".into(),
        logical_size: 0,
        version_sequence: sequence,
        content_hash: "hash".into(),
        checksum_chunk_bytes: 1,
        checksum_manifest_hash: "manifest".into(),
        creation_name: "name".into(),
        storage_name: "v000001_20261020T000000Z_00000000.txt".into(),
        created_at: "2026-10-20T00:00:00Z".into(),
        source_version_ids: vec![],
        created_as_full_checkpoint: true,
    }
}
fn engine(temp: &tempfile::TempDir) -> Engine {
    Engine::initialize(
        &temp.path().join("state"),
        Config::development([
            temp.path().join("p"),
            temp.path().join("b"),
            temp.path().join("t"),
        ]),
    )
    .unwrap()
}
fn scan(engine: &Engine, file: &str) -> Vec<FileVersion> {
    let mut versions: Vec<_> = engine
        .state
        .versions
        .values()
        .filter(|v| v.file_id == file)
        .cloned()
        .collect();
    versions.sort_by_key(|v| v.version_sequence);
    versions
}
fn populate(engine: &mut Engine) {
    for (id, seq) in [
        ("z", 0),
        ("b", u64::MAX),
        ("a", u64::MAX),
        ("c", 1 << 63),
        ("d", (1 << 63) - 1),
    ] {
        engine
            .state
            .versions
            .insert(id.into(), version(id, "file", seq));
    }
    for n in 0..2000 {
        let id = format!("other_{n}");
        engine
            .state
            .versions
            .insert(id.clone(), version(&id, "unrelated", n));
    }
    engine.rebuild_index().unwrap();
}

#[test]
fn indexed_history_matches_scan_with_unsigned_extremes_ties_and_scoped_query() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    assert_eq!(
        engine.indexed_file_history("file"),
        Some(scan(&engine, "file"))
    );
    assert_eq!(engine.indexed_file_history("absent"), Some(vec![]));
    let path = engine.root.join("index.sqlite");
    platform_io::sqlite::block_on_sqlite_connection(async {
        let mut c = platform_io::sqlite::open_index_read_connection(&path)
            .await
            .unwrap();
        let plans = sqlx::query("EXPLAIN QUERY PLAN SELECT version_id,file_id,history_tree_id,version_sequence_key FROM file_history_versions INDEXED BY file_history_lookup WHERE file_id=? ORDER BY version_sequence_key,version_id LIMIT ?")
                .bind("file").bind(6i64).fetch_all(&mut c).await.unwrap();
        let detail: String = plans[0].get(3);
        assert!(
            detail.contains(
                "SEARCH file_history_versions USING INDEX file_history_lookup (file_id=?)"
            ),
            "{detail}"
        );
        assert!(
            !plans
                .iter()
                .any(|r| r.get::<String, _>(3).contains("TEMP B-TREE"))
        );
        c.close().await.unwrap();
    });
}

#[test]
fn history_delta_handles_deletes_replacements_and_identity_moves() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    let previous = engine.state.clone();
    engine.state.sequence += 1;
    engine.state.versions.remove("z");
    engine
        .state
        .versions
        .insert("a".into(), version("a", "moved", 2));
    engine
        .state
        .versions
        .insert("new".into(), version("new", "file", 5));
    engine.state.versions.get_mut("b").unwrap().history_tree_id = "changed".into();
    engine.update_index(&previous).unwrap();
    for file in ["file", "moved", "unrelated", "missing"] {
        assert_eq!(engine.indexed_file_history(file), Some(scan(&engine, file)));
    }
}

#[test]
fn malformed_or_incomplete_index_falls_back_and_latches_until_rebuild() {
    for statement in [
        "DELETE FROM file_history_versions WHERE version_id='a'",
        "UPDATE current_state SET sequence='999'",
        "DROP TABLE current_state;CREATE VIEW current_state AS WITH RECURSIVE bounded(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM bounded WHERE n<10000) SELECT 1 AS singleton, CAST(sum(n)*0 AS TEXT) AS sequence FROM bounded",
        "UPDATE file_history_versions SET file_id='file' WHERE version_id='other_0'",
        "UPDATE file_history_versions SET history_tree_id='forged' WHERE version_id='a'",
        "UPDATE file_history_versions SET version_sequence_key=X'0000000000000001' WHERE version_id='a'",
        "PRAGMA ignore_check_constraints=ON;UPDATE file_history_versions SET version_sequence_key=X'01' WHERE version_id='a'",
        "UPDATE file_history_versions SET history_tree_id=hex(zeroblob(100000)) WHERE version_id='a'",
        "DROP TABLE file_history_versions",
        "DROP INDEX file_history_lookup",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = engine(&temp);
        populate(&mut engine);
        sql(&engine.root.join("index.sqlite"), statement);
        assert!(engine.indexed_file_history("file").is_none(), "{statement}");
        assert_eq!(
            engine.file_history("file").unwrap(),
            scan(&engine, "file"),
            "{statement}"
        );
        assert!(
            !engine.history_query_proof.lock().unwrap().healthy,
            "{statement}"
        );
        assert!(engine.indexed_file_history("file").is_none());
        engine.rebuild_index().unwrap();
        assert_eq!(
            engine.indexed_file_history("file"),
            Some(scan(&engine, "file"))
        );
    }
}

#[test]
fn history_missing_database_and_untrusted_snapshot_fall_back() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    std::fs::remove_file(engine.root.join("index.sqlite")).unwrap();
    assert!(engine.indexed_file_history("file").is_none());
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
    assert!(!engine.root.join("index.sqlite").exists());
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
    *engine.history_query_proof.lock().unwrap() = QueryProof::default();
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
}

#[test]
fn typed_history_write_failure_rolls_back_all_projection_rows_and_proof() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    let path = engine.root.join("index.sqlite");
    let baseline = rows(&path);
    sql(
        &path,
        "CREATE TRIGGER fail_history BEFORE INSERT ON file_history_versions BEGIN SELECT RAISE(ABORT,'fail');END;",
    );
    let previous = engine.state.clone();
    engine.state.sequence += 1;
    engine
        .state
        .versions
        .insert("new".into(), version("new", "file", 1));
    assert!(engine.update_index(&previous).is_err());
    assert_eq!(rows(&path), baseline);
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
}

#[test]
fn busy_history_read_has_short_budget_and_latches() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    let path = engine.root.join("index.sqlite");
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        platform_io::sqlite::block_on_sqlite_connection(async {
            let mut c = platform_io::sqlite::open_index_connection(&path, false, None)
                .await
                .unwrap();
            c.execute("PRAGMA journal_mode=DELETE;BEGIN EXCLUSIVE")
                .await
                .unwrap();
            ready_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            c.execute("ROLLBACK").await.unwrap();
            c.close().await.unwrap();
        });
    });
    ready_rx.recv().unwrap();
    let start = std::time::Instant::now();
    assert!(engine.indexed_file_history("file").is_none());
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
    let start = std::time::Instant::now();
    for _ in 0..100 {
        assert!(engine.indexed_file_history("file").is_none());
    }
    assert!(start.elapsed() < std::time::Duration::from_millis(100));
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
}

#[test]
fn successful_delta_does_not_clear_unhealthy_latch() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    sql(
        &engine.root.join("index.sqlite"),
        "DELETE FROM file_history_versions WHERE version_id='a'",
    );
    assert!(engine.indexed_file_history("file").is_none());
    let previous = engine.state.clone();
    engine.state.sequence += 1;
    engine
        .state
        .file_notes
        .insert("file".into(), "changed".into());
    engine.update_index(&previous).unwrap();
    assert!(engine.indexed_file_history("file").is_none());
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
    engine.rebuild_index().unwrap();
    assert_eq!(
        engine.indexed_file_history("file"),
        Some(scan(&engine, "file"))
    );
}

#[test]
fn actual_history_route_uses_index_only_for_large_selective_stores() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    populate(&mut engine);
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
    assert_eq!(engine.history_query_proof.lock().unwrap().indexed_reads, 0);
    for n in engine.state.versions.len()..HISTORY_INDEX_MIN_VERSIONS {
        let id = format!("large_{n}");
        engine
            .state
            .versions
            .insert(id.clone(), version(&id, "unrelated", n as u64));
    }
    engine.rebuild_index().unwrap();
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
    assert_eq!(engine.history_query_proof.lock().unwrap().indexed_reads, 1);
    assert!(!engine.history_index_is_selective("unrelated"));
    sql(
        &engine.root.join("index.sqlite"),
        "DELETE FROM file_history_versions WHERE version_id='a'",
    );
    assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
}

#[test]
#[ignore = "manual latency measurement"]
fn measure_history_candidate_latency() {
    for (count, selected) in [
        (10, 5),
        (1000, 5),
        (10_000, 5),
        (100_000, 5),
        (100_000, 100),
        (100_000, 1000),
        (300_000, 5),
        (300_000, 100),
        (300_000, 1000),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = engine(&temp);
        for n in 0..count {
            let id = format!("v_{n:08}");
            engine.state.versions.insert(
                id.clone(),
                version(&id, if n < selected { "file" } else { "other" }, n),
            );
        }
        engine.rebuild_index().unwrap();
        let start = std::time::Instant::now();
        for _ in 0..20 {
            std::hint::black_box(scan(&engine, "file"));
        }
        let scan_time = start.elapsed() / 20;
        let start = std::time::Instant::now();
        for _ in 0..20 {
            assert!(std::hint::black_box(engine.indexed_file_history("file")).is_some());
        }
        eprintln!(
            "versions={count} selected={selected} scan={scan_time:?} indexed={:?}",
            start.elapsed() / 20
        );
    }
}

fn rows(path: &Path) -> (String, Vec<(String, String, Vec<u8>)>) {
    platform_io::sqlite::block_on_sqlite_connection(async {
        let mut c = platform_io::sqlite::open_index_connection(path, false, None)
            .await
            .unwrap();
        let sequence = sqlx::query_scalar("SELECT sequence FROM current_state")
            .fetch_one(&mut c)
            .await
            .unwrap();
        let rows = sqlx::query("SELECT kind,id,json FROM entities ORDER BY kind,id")
            .fetch_all(&mut c)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.get(0), r.get(1), r.get(2)))
            .collect();
        c.close().await.unwrap();
        (sequence, rows)
    })
}
fn sql(path: &Path, statement: &'static str) {
    platform_io::sqlite::block_on_sqlite_connection(async {
        let mut c = platform_io::sqlite::open_index_connection(path, false, None)
            .await
            .unwrap();
        c.execute(statement).await.unwrap();
        c.close().await.unwrap();
    });
}
fn event(sequence: u64) -> Event {
    Event {
        sequence,
        operation_id: format!("op_{sequence}"),
        kind: "test".into(),
        object_id: "object".into(),
    }
}

fn projected_state(path: &Path) -> State {
    let (sequence, rows) = rows(path);
    let mut value = json_codec::to_value(State::default()).unwrap();
    value["sequence"] = sequence.into();
    let mut events = BTreeMap::new();
    for (kind, id, bytes) in rows {
        let row: json_codec::Value = json_codec::from_slice(&bytes).unwrap();
        match kind.as_str() {
            "$state" => value[&id] = row,
            "$events" => {
                events.insert(id.parse::<usize>().unwrap(), row);
            }
            _ => {
                let map = value
                    .as_object_mut()
                    .unwrap()
                    .entry(kind)
                    .or_insert_with(|| json_codec::json!({}));
                map[&id] = row;
            }
        }
    }
    value["events"] = events.into_values().collect::<Vec<_>>().into();
    json_codec::from_value(value).unwrap()
}

#[test]
fn incremental_matches_full_projection_after_insert_update_delete_and_truncation() {
    let temp = tempfile::tempdir().unwrap();
    let incremental = temp.path().join("incremental.sqlite");
    let full = temp.path().join("full.sqlite");
    let mut state = State::default();
    project(&incremental, None, &state).unwrap();
    for sequence in 1..=4 {
        let previous = state.clone();
        state.sequence = sequence;
        state.business_generation = sequence;
        state
            .file_notes
            .insert("file".into(), format!("note {sequence}"));
        state
            .file_selection_generations
            .insert("file".into(), u64::MAX);
        state.events.push(event(sequence));
        state.control_auth = Some(json_codec::json!({"nested": {"generation": sequence}}));
        if sequence == 2 {
            state
                .file_notes
                .insert("deleted".into(), "temporary".into());
        }
        if sequence == 3 {
            state.file_notes.remove("deleted");
            state.file_selection_generations.clear();
            state.control_auth = None;
            state.events[0] = event(99);
        }
        if sequence == 4 {
            state.events.truncate(1);
        }
        project(&incremental, Some(&previous), &state).unwrap();
        project(&full, None, &state).unwrap();
        assert_eq!(rows(&incremental), rows(&full));
        assert_eq!(projected_state(&incremental), state);
    }
    let (_, projected) = rows(&incremental);
    assert!(
        projected
            .iter()
            .any(|(kind, _, json)| kind == "file_selection_generations"
                && json == b"\"18446744073709551615\"")
    );
}

#[test]
fn per_change_writes_and_serialization_do_not_grow_with_unchanged_entities() {
    let temp = tempfile::tempdir().unwrap();
    for count in [1, 1000] {
        let path = temp.path().join(format!("{count}.sqlite"));
        let mut previous = State::default();
        for i in 0..count {
            previous
                .file_notes
                .insert(format!("file_{i}"), "unchanged".into());
        }
        project(&path, None, &previous).unwrap();
        // The audit sees actual SQLite writes, not just the delta planner.
        sql(
            &path,
            "CREATE TABLE writes(n INTEGER);INSERT INTO writes VALUES(0);CREATE TRIGGER changed_insert AFTER INSERT ON entities BEGIN UPDATE writes SET n=n+1;END;CREATE TRIGGER changed_update AFTER UPDATE ON entities BEGIN UPDATE writes SET n=n+1;END;CREATE TRIGGER changed_delete AFTER DELETE ON entities BEGIN UPDATE writes SET n=n+1;END;",
        );
        let mut next = previous.clone();
        next.sequence = 1;
        next.file_notes.insert("file_0".into(), "changed".into());
        let work = project(&path, Some(&previous), &next).unwrap();
        assert_eq!(
            work,
            IndexWork {
                upserts: 1,
                deletes: 0,
                serialized_bytes: 9
            }
        );
        let writes: i64 = platform_io::sqlite::block_on_sqlite_connection(async {
            let mut c = platform_io::sqlite::open_index_connection(&path, false, None)
                .await
                .unwrap();
            sqlx::query_scalar("SELECT n FROM writes")
                .fetch_one(&mut c)
                .await
                .unwrap()
        });
        assert_eq!(writes, 1);
        assert_eq!(
            project(&path, Some(&next), &next).unwrap(),
            IndexWork::default()
        );
        let mut removed = next.clone();
        removed.sequence = 2;
        removed.file_notes.remove("file_0");
        assert_eq!(
            project(&path, Some(&next), &removed).unwrap(),
            IndexWork {
                upserts: 0,
                deletes: 1,
                serialized_bytes: 0
            }
        );
    }
}

#[test]
fn failed_delta_rolls_back_rows_and_sequence_then_rebuilds_from_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("index.sqlite");
    let previous = State::default();
    project(&path, None, &previous).unwrap();
    let baseline = rows(&path);
    let mut next = previous.clone();
    next.sequence = 7;
    next.file_notes.insert("first".into(), "changed".into());
    next.file_notes.insert("second".into(), "fail".into());
    sql(
        &path,
        "CREATE TRIGGER fail_second BEFORE INSERT ON entities WHEN NEW.id='second' BEGIN SELECT RAISE(ABORT,'injected failure');END;",
    );
    assert!(project(&path, Some(&previous), &next).is_err());
    // The failed transaction's native worker has closed before returning.
    assert!(!path.with_file_name("index.sqlite-wal").exists());
    assert!(!path.with_file_name("index.sqlite-shm").exists());
    assert_eq!(rows(&path), baseline);
    // Full rebuild discards failed/stale projection and its triggers.
    project(&path, None, &next).unwrap();
    let rebuilt = rows(&path);
    assert_eq!(rebuilt.0, "7");
    assert!(rebuilt.1.iter().any(|(_, id, _)| id == "second"));
    assert!(project(&path, Some(&previous), &next).is_err());
    assert_eq!(rows(&path), rebuilt);
}

#[test]
fn publication_rejects_substituted_marker_view_and_reopen_recovers() {
    let temp = tempfile::tempdir().unwrap();
    let mut engine = engine(&temp);
    let root = engine.root.clone();
    // The view returns the correct old marker, so value-only checking would
    // accept it. Keep evaluation finite even if schema validation regresses.
    sql(
        &root.join("index.sqlite"),
        "DROP TABLE current_state;CREATE VIEW current_state AS WITH RECURSIVE bounded(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM bounded WHERE n<10000) SELECT 1 AS singleton, CAST(sum(n)*0 AS TEXT) AS sequence FROM bounded",
    );
    assert!(engine.indexed_file_history("absent").is_none());
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
    let operation = new_id("op");
    let error = engine
        .create_repository("published", "P01", 1, "admin", &operation)
        .unwrap_err();
    assert_eq!(error.code, "INDEX_REBUILD_FAILED");
    assert!(
        error
            .to_string()
            .contains("projection marker schema mismatch")
    );
    assert!(engine.is_read_only());
    assert!(engine.state.operations.contains_key(&operation));
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
    let published = engine.state.clone();
    drop(engine);
    let engine = Engine::open(&root).unwrap();
    assert!(!engine.is_read_only());
    assert_eq!(engine.state, published);
    assert_eq!(projected_state(&root.join("index.sqlite")), published);
    assert!(engine.history_query_proof.lock().unwrap().healthy);
}

#[test]
fn published_evidence_survives_index_failure_and_reopen_rebuilds() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("state");
    let config = Config::development([
        temp.path().join("primary"),
        temp.path().join("backup"),
        temp.path().join("tertiary"),
    ]);
    let mut engine = Engine::initialize(&root, config).unwrap();
    engine
        .create_repository("first", "P01", 1, "admin", &new_id("op"))
        .unwrap();
    let path = root.join("index.sqlite");
    assert_eq!(projected_state(&path), engine.state);
    let baseline = rows(&path);
    sql(
        &path,
        "CREATE TRIGGER fail_write BEFORE INSERT ON entities BEGIN SELECT RAISE(ABORT,'injected projection failure');END;",
    );
    let operation = new_id("op");
    let error = engine
        .create_repository("second", "P01", 1, "admin", &operation)
        .unwrap_err();
    assert_eq!(error.code, "INDEX_REBUILD_FAILED");
    assert!(engine.is_read_only());
    assert!(engine.state.operations.contains_key(&operation));
    let published = engine.state.clone();
    assert_eq!(rows(&path), baseline);
    drop(engine);
    let engine = Engine::open(&root).unwrap();
    assert!(!engine.is_read_only());
    assert_eq!(engine.state, published);
    assert_eq!(projected_state(&path), published);
    drop(engine);
    std::fs::remove_file(&path).unwrap();
    let engine = Engine::open(&root).unwrap();
    assert_eq!(engine.state, published);
    assert_eq!(projected_state(&path), published);
}

#[test]
fn startup_rebuilds_a_corrupt_disposable_database_from_durable_evidence() {
    for corrupt_page in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let mut engine = engine(&temp);
        engine
            .create_repository("published", "P01", 1, "admin", &new_id("op"))
            .unwrap();
        let root = engine.root.clone();
        let published = engine.state.clone();
        drop(engine);
        let corrupt_bytes = if corrupt_page {
            let mut bytes = std::fs::read(root.join("index.sqlite")).unwrap();
            bytes[100] = 0xff; // Invalid b-tree page type: SQLITE_CORRUPT, not NOTADB.
            bytes
        } else {
            b"not a SQLite database".to_vec()
        };
        std::fs::write(root.join("index.sqlite"), &corrupt_bytes).unwrap();
        let recovered = Engine::open(&root).unwrap();
        assert_eq!(recovered.state, published);
        assert_eq!(projected_state(&root.join("index.sqlite")), published);
        assert!(recovered.history_query_proof.lock().unwrap().healthy);
        assert!(!recovered.is_read_only());
        let quarantines: Vec<_> = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("index-corrupt-")
            })
            .collect();
        assert_eq!(quarantines.len(), 1);
        assert_eq!(
            std::fs::read(quarantines[0].join("index.sqlite")).unwrap(),
            corrupt_bytes
        );
    }
}

#[test]
fn corrupt_reader_closes_before_immediate_rebuild() {
    let temp = tempfile::tempdir().unwrap();
    let engine = engine(&temp);
    let path = engine.root.join("index.sqlite");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[100] = 0xff;
    std::fs::write(&path, &bytes).unwrap();
    assert!(engine.indexed_file_history("missing").is_none());
    assert!(!engine.history_query_proof.lock().unwrap().healthy);
    engine.rebuild_index().unwrap();
    assert_eq!(engine.indexed_file_history("missing"), Some(vec![]));
    assert_eq!(projected_state(&path), engine.state);
}

#[test]
fn corrupt_index_quarantine_preserves_surviving_sidecars() {
    let temp = tempfile::tempdir().unwrap();
    for name in INDEX_FILES {
        std::fs::write(temp.path().join(name), name.as_bytes()).unwrap();
    }
    quarantine_corrupt_index(temp.path()).unwrap();
    let quarantine = std::fs::read_dir(temp.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    for name in INDEX_FILES {
        assert!(!temp.path().join(name).exists());
        assert_eq!(
            std::fs::read(quarantine.join(name)).unwrap(),
            name.as_bytes()
        );
    }
}

#[test]
fn rebuild_refuses_linked_index_or_sidecar_without_touching_target() {
    for name in INDEX_FILES {
        for hard_link in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let engine = engine(&temp);
            let path = engine.root.join(name);
            if path.exists() {
                std::fs::remove_file(&path).unwrap();
            }
            let target = temp.path().join("unrelated");
            std::fs::write(&target, b"do not change").unwrap();
            if hard_link {
                std::fs::hard_link(&target, &path).unwrap();
            } else {
                std::os::unix::fs::symlink(&target, &path).unwrap();
            }
            let original_index = std::fs::read(engine.root.join("index.sqlite")).unwrap();
            let error = engine.rebuild_index().unwrap_err();
            assert_eq!(error.code, "UNSAFE_PATH", "{name} hard_link={hard_link}");
            assert_eq!(std::fs::read(&target).unwrap(), b"do not change");
            assert_eq!(
                std::fs::read(engine.root.join("index.sqlite")).unwrap(),
                original_index
            );
            assert!(!engine.history_query_proof.lock().unwrap().healthy);
            assert!(!std::fs::read_dir(&engine.root).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("index-corrupt-")
            }));
        }
    }
}

#[test]
fn runtime_linked_cache_falls_back_and_blocks_publication_without_touching_target() {
    for name in INDEX_FILES {
        for hard_link in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let mut engine = engine(&temp);
            populate(&mut engine);
            let path = engine.root.join(name);
            if path.exists() {
                std::fs::remove_file(&path).unwrap();
            }
            let target = temp.path().join("unrelated");
            std::fs::write(&target, b"do not change").unwrap();
            if hard_link {
                std::fs::hard_link(&target, &path).unwrap();
            } else {
                std::os::unix::fs::symlink(&target, &path).unwrap();
            }
            assert!(engine.indexed_file_history("file").is_none());
            assert_eq!(engine.file_history("file").unwrap(), scan(&engine, "file"));
            assert!(!engine.history_query_proof.lock().unwrap().healthy);
            let error = engine
                .create_repository("published", "P01", 1, "admin", &new_id("op"))
                .unwrap_err();
            assert_eq!(error.code, "UNSAFE_PATH");
            assert!(engine.is_read_only());
            assert_eq!(std::fs::read(&target).unwrap(), b"do not change");
        }
    }
}

#[test]
fn non_corruption_errors_cannot_authorize_quarantine() {
    for error in [
        sqlx::Error::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
        sqlx::Error::Protocol("file is not a database".into()),
        sqlx::Error::RowNotFound,
    ] {
        assert!(!ProjectionFailure::from(error).corrupt);
    }
}

#[test]
fn startup_migrates_legacy_snapshot_projection() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("index.sqlite");
    platform_io::sqlite::block_on_sqlite_connection(async {
        let mut c = platform_io::sqlite::open_index_connection(&path, true, None)
            .await
            .unwrap();
        c.execute("CREATE TABLE current_state(singleton INTEGER PRIMARY KEY,sequence TEXT,snapshot BLOB);INSERT INTO current_state VALUES(1,'999',X'7b7d');CREATE TABLE entities(kind TEXT,id TEXT,json BLOB,PRIMARY KEY(kind,id));INSERT INTO entities VALUES('stale','stale',X'7b7d');").await.unwrap();
        c.close().await.unwrap();
    });
    project(&path, None, &State::default()).unwrap();
    let (sequence, entities) = rows(&path);
    assert_eq!(sequence, "0");
    assert_eq!(entities.len(), 3);
    assert!(entities.iter().all(|(kind, _, _)| kind == "$state"));
}
