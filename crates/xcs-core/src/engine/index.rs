//! Rebuildable SQLite projection; never an authority for publication.
//!
//! Startup rebuilds from recovered durable evidence, including migration of the
//! old full-snapshot projection. Publications compare typed values and serialize
//! only changed rows. Comparisons still scan state; this is not an O(delta) CPU
//! claim. No duplicate full-State snapshot is retained in SQLite.
use super::Engine;
use crate::*;
use serde::Serialize;
use sqlx::{Connection, Executor};
use std::{collections::BTreeMap, path::Path};

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

fn project(path: &Path, before: Option<&State>, after: &State) -> Result<IndexWork> {
    let delta = Delta::between(before, after)?;
    let work = delta.work();
    let result = xcs_platform::sqlite::block_on_sqlite_connection(async {
        let mut c = xcs_platform::sqlite::open_index_connection(path, before.is_none(), None)
            .await
            .map_err(|e| e.to_string())?;
        let mut tx = c.begin().await.map_err(|e| e.to_string())?;
        if let Some(before) = before {
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
            tx.execute("DROP TABLE IF EXISTS current_state;DROP TABLE IF EXISTS entities;")
                .await
                .map_err(|e| e.to_string())?;
            tx.execute(SCHEMA).await.map_err(|e| e.to_string())?;
        }
        for (kind, id) in &delta.deletes {
            sqlx::query("DELETE FROM entities WHERE kind=? AND id=?")
                .bind(kind)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
        for (kind, id, data) in &delta.upserts {
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
        project(&self.root.join("index.sqlite"), None, &self.state).map(|_| ())
    }
    pub(crate) fn update_index(&self, previous: &State) -> Result<()> {
        project(&self.root.join("index.sqlite"), Some(previous), &self.state).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

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
