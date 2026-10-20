//! Rebuildable SQLite projection; never an authority for publication.
//!
//! Startup rebuilds from recovered durable evidence, including migration of the
//! old full-snapshot projection. Publications compare typed values and serialize
//! only changed rows. Comparisons still scan state; this is not an O(delta) CPU
//! claim. No duplicate full-State snapshot is retained in SQLite.
use super::Engine;
use crate::*;
use serde::Serialize;
use sqlx::{Connection, Executor, Row};
use std::{collections::BTreeMap, path::Path};

const HISTORY_INDEX_MIN_VERSIONS: usize = 300_000;
const HISTORY_INDEX_MAX_CANDIDATES: usize = 100;

const CURRENT_STATE_TABLE: &str = "CREATE TABLE current_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),sequence TEXT NOT NULL)";
const HISTORY_TABLE: &str = "CREATE TABLE file_history_versions(version_id TEXT PRIMARY KEY COLLATE BINARY,file_id TEXT NOT NULL,history_tree_id TEXT NOT NULL,version_sequence_key BLOB NOT NULL CHECK(typeof(version_sequence_key)='blob' AND length(version_sequence_key)=8))";
const HISTORY_INDEX: &str = "CREATE INDEX file_history_lookup ON file_history_versions(file_id,version_sequence_key,version_id)";

/// Trusted completeness evidence, never loaded from the disposable database.
#[derive(Default)]
pub(super) struct QueryProof {
    sequence: u64,
    counts: BTreeMap<String, usize>,
    healthy: bool,
    #[cfg(test)]
    indexed_reads: usize,
}

const SCHEMA: &str = "CREATE TABLE current_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),sequence TEXT NOT NULL);CREATE TABLE entities(kind TEXT NOT NULL,id TEXT NOT NULL,json BLOB NOT NULL,PRIMARY KEY(kind,id));";

#[derive(Debug, Default, PartialEq)]
struct IndexWork {
    upserts: usize,
    deletes: usize,
    serialized_bytes: usize,
}

#[derive(Default)]
struct Delta {
    upserts: Vec<(&'static str, String, Vec<u8>)>,
    deletes: Vec<(&'static str, String)>,
}
impl Delta {
    fn put<T: Serialize>(&mut self, kind: &'static str, id: String, value: &T) -> Result<()> {
        self.upserts.push((kind, id, canonical_json(value)?));
        Ok(())
    }
    fn map<T: Serialize + PartialEq>(
        &mut self,
        kind: &'static str,
        before: Option<&BTreeMap<String, T>>,
        after: &BTreeMap<String, T>,
    ) -> Result<()> {
        for (id, value) in after {
            if before.and_then(|m| m.get(id)) != Some(value) {
                self.put(kind, id.clone(), value)?;
            }
        }
        if let Some(before) = before {
            for id in before.keys().filter(|id| !after.contains_key(*id)) {
                self.deletes.push((kind, id.clone()));
            }
        }
        Ok(())
    }
    fn between(before: Option<&State>, after: &State) -> Result<Self> {
        let mut delta = Self::default();
        // Exhaustive destructuring makes newly added State fields a compile-time
        // projection update requirement rather than silently omitting them.
        let State {
            sequence: _,
            business_generation,
            topology_generation,
            control_auth,
            events,
            repositories,
            histories,
            branches,
            file_selections,
            directory_branches,
            directory_trees,
            file_write_origins,
            file_contexts,
            file_path_nodes,
            file_metadata_nodes,
            file_branch_merges,
            file_branch_aliases,
            file_node_parents,
            file_path_notes,
            directory_tree_selections,
            file_locations,
            file_backup_paths,
            file_backup_policies,
            directory_selections,
            directory_selection_generations,
            file_selection_generations,
            file_notes,
            version_notes,
            file_names,
            commits,
            versions,
            placements,
            merge_links,
            history_boundaries,
            content_states,
            operations,
            plans,
            placement_rules,
            protection_policies,
            operation_principals,
        } = after;
        macro_rules! maps {
            ($($field:ident),* $(,)?) => {$(
                delta.map(stringify!($field), before.map(|s| &s.$field), $field)?;
            )*};
        }
        maps!(
            repositories,
            histories,
            branches,
            file_selections,
            directory_branches,
            directory_trees,
            file_write_origins,
            file_contexts,
            file_path_nodes,
            file_metadata_nodes,
            file_branch_merges,
            file_branch_aliases,
            file_node_parents,
            file_path_notes,
            directory_tree_selections,
            file_locations,
            file_backup_paths,
            file_backup_policies,
            directory_selections,
            file_notes,
            version_notes,
            file_names,
            commits,
            versions,
            placements,
            merge_links,
            history_boundaries,
            content_states,
            operations,
            plans,
            placement_rules,
            protection_policies,
            operation_principals
        );
        // These two maps have a string-u64 wire representation.
        macro_rules! generations {
            ($($field:ident),*) => {$(
                for (id, value) in $field {
                    if before.and_then(|s| s.$field.get(id)) != Some(value) {
                        delta.put(stringify!($field), id.clone(), &value.to_string())?;
                    }
                }
                if let Some(before) = before {
                    for id in before.$field.keys().filter(|id| !$field.contains_key(*id)) {
                        delta.deletes.push((stringify!($field), id.clone()));
                    }
                }
            )*};
        }
        generations!(directory_selection_generations, file_selection_generations);
        if before.is_none_or(|s| s.business_generation != *business_generation) {
            delta.put(
                "$state",
                "business_generation".into(),
                &business_generation.to_string(),
            )?;
        }
        if before.is_none_or(|s| s.topology_generation != *topology_generation) {
            delta.put(
                "$state",
                "topology_generation".into(),
                &topology_generation.to_string(),
            )?;
        }
        if before.is_none_or(|s| s.control_auth != *control_auth) {
            delta.put("$state", "control_auth".into(), control_auth)?;
        }
        // Events normally append, but replacement/truncation is also supported.
        for (i, event) in events.iter().enumerate() {
            if before.and_then(|s| s.events.get(i)) != Some(event) {
                delta.put("$events", i.to_string(), event)?;
            }
        }
        if let Some(before) = before {
            for i in events.len()..before.events.len() {
                delta.deletes.push(("$events", i.to_string()));
            }
        }
        Ok(delta)
    }
    fn work(&self) -> IndexWork {
        IndexWork {
            upserts: self.upserts.len(),
            deletes: self.deletes.len(),
            serialized_bytes: self.upserts.iter().map(|(_, _, v)| v.len()).sum(),
        }
    }
}

#[cfg(test)]
fn project(path: &Path, before: Option<&State>, after: &State) -> Result<IndexWork> {
    let delta = Delta::between(before, after)?;
    project_delta(path, before, after, &delta)
}

fn project_delta(
    path: &Path,
    before: Option<&State>,
    after: &State,
    delta: &Delta,
) -> Result<IndexWork> {
    let work = delta.work();
    let result = xcs_platform::sqlite::block_on_sqlite_connection(async {
        let mut c = xcs_platform::sqlite::open_index_connection(path, before.is_none(), None)
            .await
            .map_err(|e| e.to_string())?;
        let mut tx = c.begin().await.map_err(|e| e.to_string())?;
        if let Some(before) = before {
            // This runs after durable publication. Never evaluate a cache-supplied
            // view while checking the marker, even if a reader already disabled
            // its candidate path; a definite error retains write blocking/recovery.
            let marker =
                sqlx::query("SELECT type,sql FROM sqlite_schema WHERE name='current_state'")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            if marker.try_get::<String, _>(0).map_err(|e| e.to_string())? != "table"
                || marker.try_get::<String, _>(1).map_err(|e| e.to_string())? != CURRENT_STATE_TABLE
            {
                return Err(
                    "projection marker schema mismatch; reopen to rebuild from durable evidence"
                        .into(),
                );
            }
            let sequence: Option<String> =
                sqlx::query_scalar("SELECT sequence FROM current_state WHERE singleton=1")
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            if sequence.as_deref() != Some(before.sequence.to_string().as_str()) {
                return Err(
                    "projection sequence mismatch; reopen to rebuild from durable evidence".into(),
                );
            }
        } else {
            // Transactional replacement also migrates the legacy snapshot table.
            // A damaged cache may have substituted views for our tables.
            for (name, drop_view) in [
                ("current_state", "DROP VIEW current_state"),
                ("entities", "DROP VIEW entities"),
                ("file_history_versions", "DROP VIEW file_history_versions"),
            ] {
                let kind: Option<String> =
                    sqlx::query_scalar("SELECT type FROM sqlite_schema WHERE name=?")
                        .bind(name)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                if kind.as_deref() == Some("view") {
                    tx.execute(drop_view).await.map_err(|e| e.to_string())?;
                }
            }
            tx.execute("DROP TABLE IF EXISTS current_state;DROP TABLE IF EXISTS entities;DROP TABLE IF EXISTS file_history_versions;")
                .await
                .map_err(|e| e.to_string())?;
            tx.execute(SCHEMA).await.map_err(|e| e.to_string())?;
            tx.execute(HISTORY_TABLE).await.map_err(|e| e.to_string())?;
            tx.execute(HISTORY_INDEX).await.map_err(|e| e.to_string())?;
        }
        for (kind, id) in &delta.deletes {
            if *kind == "versions" {
                sqlx::query("DELETE FROM file_history_versions WHERE version_id=?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            sqlx::query("DELETE FROM entities WHERE kind=? AND id=?")
                .bind(kind)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
        for (kind, id, data) in &delta.upserts {
            if *kind == "versions" {
                let v = &after.versions[id];
                sqlx::query("INSERT INTO file_history_versions VALUES(?,?,?,?) ON CONFLICT(version_id) DO UPDATE SET file_id=excluded.file_id,history_tree_id=excluded.history_tree_id,version_sequence_key=excluded.version_sequence_key")
                    .bind(id).bind(&v.file_id).bind(&v.history_tree_id).bind(v.version_sequence.to_be_bytes().as_slice())
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            }
            sqlx::query("INSERT INTO entities(kind,id,json) VALUES(?,?,?) ON CONFLICT(kind,id) DO UPDATE SET json=excluded.json")
                .bind(kind).bind(id).bind(data).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        sqlx::query("INSERT INTO current_state VALUES(1,?) ON CONFLICT(singleton) DO UPDATE SET sequence=excluded.sequence")
            .bind(after.sequence.to_string()).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        c.close().await.map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    });
    result.map_err(|e| Error::new("INDEX_REBUILD_FAILED", e))?;
    Ok(work)
}

impl Engine {
    pub(crate) fn rebuild_index(&self) -> Result<()> {
        let mut proof = self.history_query_proof.lock().unwrap();
        proof.healthy = false;
        let delta = Delta::between(None, &self.state)?;
        project_delta(&self.root.join("index.sqlite"), None, &self.state, &delta)?;
        proof.counts.clear();
        for v in self.state.versions.values() {
            *proof.counts.entry(v.file_id.clone()).or_default() += 1;
        }
        proof.sequence = self.state.sequence;
        proof.healthy = true;
        Ok(())
    }
    pub(crate) fn update_index(&self, previous: &State) -> Result<()> {
        let delta = Delta::between(Some(previous), &self.state)?;
        let mut proof = self.history_query_proof.lock().unwrap();
        if let Err(error) = project_delta(
            &self.root.join("index.sqlite"),
            Some(previous),
            &self.state,
            &delta,
        ) {
            proof.healthy = false;
            return Err(error);
        }
        if proof.healthy && proof.sequence == previous.sequence {
            // Reuse the changed-version delta, including replacements/moves.
            for id in delta
                .deletes
                .iter()
                .filter(|(kind, _)| *kind == "versions")
                .map(|(_, id)| id)
                .chain(
                    delta
                        .upserts
                        .iter()
                        .filter(|(kind, _, _)| *kind == "versions")
                        .map(|(_, id, _)| id),
                )
            {
                if let Some(old) = previous.versions.get(id) {
                    let count = proof
                        .counts
                        .get_mut(&old.file_id)
                        .expect("trusted version count");
                    *count -= 1;
                    if *count == 0 {
                        proof.counts.remove(&old.file_id);
                    }
                }
                if let Some(new) = self.state.versions.get(id) {
                    *proof.counts.entry(new.file_id.clone()).or_default() += 1;
                }
            }
            proof.sequence = self.state.sequence;
        } else {
            proof.healthy = false;
        }
        Ok(())
    }

    pub(crate) fn history_index_is_selective(&self, file_id: &str) -> bool {
        // Connection/validation costs dominate small maps and large result sets.
        // Conservative measured threshold; this is not an API response limit.
        self.state.versions.len() >= HISTORY_INDEX_MIN_VERSIONS
            && self.history_query_proof.lock().is_ok_and(|proof| {
                proof.healthy
                    && proof.counts.get(file_id).copied().unwrap_or(0)
                        <= HISTORY_INDEX_MAX_CANDIDATES
            })
    }

    /// Candidate selection only. Every field comes from State, never SQLite JSON.
    pub(crate) fn indexed_file_history(&self, file_id: &str) -> Option<Vec<FileVersion>> {
        let mut proof = self.history_query_proof.lock().ok()?;
        if !proof.healthy || proof.sequence != self.state.sequence {
            return None;
        }
        let expected = proof.counts.get(file_id).copied().unwrap_or(0);
        let limit = i64::try_from(expected).ok()?.checked_add(1)?;
        let result = xcs_platform::sqlite::block_on_sqlite_connection(async {
            let mut c =
                xcs_platform::sqlite::open_index_read_connection(&self.root.join("index.sqlite"))
                    .await
                    .map_err(|e| e.to_string())?;
            let result = async {
                let mut tx = c.begin().await.map_err(|e| e.to_string())?;
                // Attest all queried objects before evaluating their contents: in
                // particular, a cache must not substitute an expensive view.
                for (name, kind, ddl) in [
                    ("current_state", "table", CURRENT_STATE_TABLE),
                    ("file_history_versions", "table", HISTORY_TABLE),
                    ("file_history_lookup", "index", HISTORY_INDEX),
                ] {
                    let actual = sqlx::query("SELECT type,sql FROM sqlite_schema WHERE name=?")
                        .bind(name).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
                    if actual.try_get::<String, _>(0).map_err(|e| e.to_string())? != kind
                        || actual.try_get::<String, _>(1).map_err(|e| e.to_string())? != ddl {
                        return Err("query schema mismatch".into());
                    }
                }
                let sequence: String = sqlx::query_scalar("SELECT sequence FROM current_state WHERE singleton=1")
                    .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
                if sequence != self.state.sequence.to_string() { return Err("stale sequence".into()); }
                let rows = sqlx::query("SELECT version_id,file_id,history_tree_id,version_sequence_key FROM file_history_versions INDEXED BY file_history_lookup WHERE file_id=? ORDER BY version_sequence_key,version_id LIMIT ?")
                    .bind(file_id).bind(limit).fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
                if rows.len() != expected { return Err("incomplete candidate set".into()); }
                let mut versions = Vec::with_capacity(expected);
                let mut prior: Option<(u64, String)> = None;
                for row in rows {
                    let id: String = row.try_get(0).map_err(|e| e.to_string())?;
                    let file: String = row.try_get(1).map_err(|e| e.to_string())?;
                    let history: String = row.try_get(2).map_err(|e| e.to_string())?;
                    let key: Vec<u8> = row.try_get(3).map_err(|e| e.to_string())?;
                    let v = self.state.versions.get(&id).ok_or("unknown candidate")?;
                    if file != file_id || v.file_id != file || v.history_tree_id != history || v.version_id != id || key != v.version_sequence.to_be_bytes() {
                        return Err("invalid candidate".into());
                    }
                    let order = (v.version_sequence, id);
                    if prior.as_ref().is_some_and(|p| p >= &order) { return Err("invalid candidate order".into()); }
                    prior = Some(order);
                    versions.push(v.clone());
                }
                tx.commit().await.map_err(|e| e.to_string())?;
                Ok::<_, String>(versions)
            }.await;
            let closed = c.close().await.map_err(|e| e.to_string());
            match (result, closed) {
                (Ok(v), Ok(())) => Ok(v),
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        });
        match result {
            Ok(versions) => {
                #[cfg(test)]
                {
                    proof.indexed_reads += 1;
                }
                Some(versions)
            }
            Err(_) => {
                proof.healthy = false;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    fn version(id: &str, file: &str, sequence: u64) -> FileVersion {
        FileVersion {
            format_version: 2,
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
            storage_name: None,
            created_at: None,
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
        xcs_platform::sqlite::block_on_sqlite_connection(async {
            let mut c = xcs_platform::sqlite::open_index_read_connection(&path)
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
            xcs_platform::sqlite::block_on_sqlite_connection(async {
                let mut c = xcs_platform::sqlite::open_index_connection(&path, false, None)
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
        xcs_platform::sqlite::block_on_sqlite_connection(async {
            let mut c = xcs_platform::sqlite::open_index_connection(path, false, None)
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
        xcs_platform::sqlite::block_on_sqlite_connection(async {
            let mut c = xcs_platform::sqlite::open_index_connection(path, false, None)
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
        let mut value = serde_json::to_value(State::default()).unwrap();
        value["sequence"] = sequence.into();
        let mut events = BTreeMap::new();
        for (kind, id, bytes) in rows {
            let row: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
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
                        .or_insert_with(|| serde_json::json!({}));
                    map[&id] = row;
                }
            }
        }
        value["events"] = events.into_values().collect::<Vec<_>>().into();
        serde_json::from_value(value).unwrap()
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
            state.control_auth = Some(serde_json::json!({"nested": {"generation": sequence}}));
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
            let writes: i64 = xcs_platform::sqlite::block_on_sqlite_connection(async {
                let mut c = xcs_platform::sqlite::open_index_connection(&path, false, None)
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
    fn startup_migrates_legacy_snapshot_projection() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("index.sqlite");
        xcs_platform::sqlite::block_on_sqlite_connection(async {
            let mut c = xcs_platform::sqlite::open_index_connection(&path, true, None)
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
}
