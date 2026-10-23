//! Rebuildable SQLite projection; never an authority for publication.
//!
//! Startup rebuilds from recovered durable evidence, including migration of the
//! old full-snapshot projection. Publications compare typed values and serialize
//! only changed rows. Comparisons still scan state; this is not an O(delta) CPU
//! claim. No duplicate full-State snapshot is retained in SQLite.
use super::Engine;
use crate::*;
use data_serialization::Serialize;
use sqlite_access::{Connection, Executor, Row};
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
        let Some(before) = before else {
            for (id, value) in after {
                self.put(kind, id.clone(), value)?;
            }
            return Ok(());
        };
        // Merge the ordered maps once, preserving each output vector's key
        // order without two tree searches per unchanged row.
        let mut before = before.iter().peekable();
        for (id, value) in after {
            while let Some((old_id, _)) = before.next_if(|(old_id, _)| *old_id < id) {
                self.deletes.push((kind, old_id.clone()));
            }
            let previous = before
                .next_if(|(old_id, _)| *old_id == id)
                .map(|(_, value)| value);
            if previous != Some(value) {
                self.put(kind, id.clone(), value)?;
            }
        }
        self.deletes
            .extend(before.map(|(id, _)| (kind, id.clone())));
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

#[derive(Debug)]
struct ProjectionFailure {
    message: String,
    corrupt: bool,
}
impl From<sqlite_access::Error> for ProjectionFailure {
    fn from(error: sqlite_access::Error) -> Self {
        // Extended SQLite codes retain their primary error in the low byte.
        // Never turn permission, I/O, busy, or schema errors into file replacement.
        let corrupt = error
            .as_database_error()
            .and_then(|error| error.code())
            .and_then(|code| code.parse::<i32>().ok())
            .is_some_and(|code| matches!(code & 0xff, 11 | 26));
        Self {
            message: error.to_string(),
            corrupt,
        }
    }
}
impl From<platform_io::sqlite::Error> for ProjectionFailure {
    fn from(error: platform_io::sqlite::Error) -> Self {
        match error {
            platform_io::sqlite::Error::Sqlx(error) => error.into(),
            error => Self {
                message: error.to_string(),
                corrupt: false,
            },
        }
    }
}
impl From<&str> for ProjectionFailure {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
            corrupt: false,
        }
    }
}
impl From<ProjectionFailure> for Error {
    fn from(error: ProjectionFailure) -> Self {
        Self::new("INDEX_REBUILD_FAILED", error.message)
    }
}

fn project_delta(
    path: &Path,
    before: Option<&State>,
    after: &State,
    delta: &Delta,
) -> Result<IndexWork> {
    try_project_delta(path, before, after, delta).map_err(Into::into)
}

fn try_project_delta(
    path: &Path,
    before: Option<&State>,
    after: &State,
    delta: &Delta,
) -> std::result::Result<IndexWork, ProjectionFailure> {
    let work = delta.work();
    let result = platform_io::sqlite::block_on_sqlite_connection(async {
        let mut c = platform_io::sqlite::open_index_connection(path, before.is_none(), None)
            .await
            .map_err(ProjectionFailure::from)?;
        let result = async {
        let mut tx = c.begin().await.map_err(ProjectionFailure::from)?;
        if let Some(before) = before {
            // This runs after durable publication. Never evaluate a cache-supplied
            // view while checking the marker, even if a reader already disabled
            // its candidate path; a definite error retains write blocking/recovery.
            let marker =
                sqlite_access::query("SELECT type,sql FROM sqlite_schema WHERE name='current_state'")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(ProjectionFailure::from)?;
            if marker.try_get::<String, _>(0).map_err(ProjectionFailure::from)? != "table"
                || marker.try_get::<String, _>(1).map_err(ProjectionFailure::from)? != CURRENT_STATE_TABLE
            {
                return Err(
                    "projection marker schema mismatch; reopen to rebuild from durable evidence"
                        .into(),
                );
            }
            let sequence: Option<String> =
                sqlite_access::query_scalar("SELECT sequence FROM current_state WHERE singleton=1")
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(ProjectionFailure::from)?;
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
                    sqlite_access::query_scalar("SELECT type FROM sqlite_schema WHERE name=?")
                        .bind(name)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(ProjectionFailure::from)?;
                if kind.as_deref() == Some("view") {
                    tx.execute(drop_view).await.map_err(ProjectionFailure::from)?;
                }
            }
            tx.execute("DROP TABLE IF EXISTS current_state;DROP TABLE IF EXISTS entities;DROP TABLE IF EXISTS file_history_versions;")
                .await
                .map_err(ProjectionFailure::from)?;
            tx.execute(SCHEMA).await.map_err(ProjectionFailure::from)?;
            tx.execute(HISTORY_TABLE).await.map_err(ProjectionFailure::from)?;
            tx.execute(HISTORY_INDEX).await.map_err(ProjectionFailure::from)?;
        }
        for (kind, id) in &delta.deletes {
            if *kind == "versions" {
                sqlite_access::query("DELETE FROM file_history_versions WHERE version_id=?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(ProjectionFailure::from)?;
            }
            sqlite_access::query("DELETE FROM entities WHERE kind=? AND id=?")
                .bind(kind)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(ProjectionFailure::from)?;
        }
        for (kind, id, data) in &delta.upserts {
            if *kind == "versions" {
                let v = &after.versions[id];
                sqlite_access::query("INSERT INTO file_history_versions VALUES(?,?,?,?) ON CONFLICT(version_id) DO UPDATE SET file_id=excluded.file_id,history_tree_id=excluded.history_tree_id,version_sequence_key=excluded.version_sequence_key")
                    .bind(id).bind(&v.file_id).bind(&v.history_tree_id).bind(v.version_sequence.to_be_bytes().as_slice())
                    .execute(&mut *tx).await.map_err(ProjectionFailure::from)?;
            }
            sqlite_access::query("INSERT INTO entities(kind,id,json) VALUES(?,?,?) ON CONFLICT(kind,id) DO UPDATE SET json=excluded.json")
                .bind(kind).bind(id).bind(data).execute(&mut *tx).await.map_err(ProjectionFailure::from)?;
        }
        sqlite_access::query("INSERT INTO current_state VALUES(1,?) ON CONFLICT(singleton) DO UPDATE SET sequence=excluded.sequence")
            .bind(after.sequence.to_string()).execute(&mut *tx).await.map_err(ProjectionFailure::from)?;
        tx.commit().await.map_err(ProjectionFailure::from)?;
        Ok::<(), ProjectionFailure>(())
        }.await;
        // Do not replace files if closing the native worker is uncertain.
        if let Err(error) = c.close().await {
            return Err(ProjectionFailure {
                message: error.to_string(),
                corrupt: false,
            });
        }
        result
    });
    result?;
    Ok(work)
}

// Move every journal before the main database. A partial quarantine must never
// leave old WAL bytes next to a missing database that a later open could create.
const INDEX_FILES: [&str; 4] = [
    "index.sqlite-wal",
    "index.sqlite-shm",
    "index.sqlite-journal",
    "index.sqlite",
];

fn checked_index_files(root: &Path) -> Result<Vec<(&'static str, std::fs::File)>> {
    let mut files = Vec::new();
    for name in INDEX_FILES {
        match std::fs::symlink_metadata(root.join(name)) {
            Ok(_) => files.push((name, crate::disk::open_regular(&root.join(name))?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(files)
}

/// Preserve the corrupt cache and any journals remaining after SQLite closes.
/// This is recoverable diagnostic material, not a forensic pre-open snapshot:
/// SQLite may already have recovered, checkpointed or removed sidecars during
/// cleanup. These files are never recovery authority; durable evidence is untouched.
/// The engine holds its exclusive root lock and no connection may remain open.
/// Reject special files/links instead of following them or overwriting targets.
fn quarantine_corrupt_index(root: &Path) -> Result<()> {
    use platform_io::fs_safety::linux::{FileIdentity, MountPolicy, OpenAt2Root};
    let files = checked_index_files(root)?;
    let source = OpenAt2Root::open(root, MountPolicy::SameMount)
        .map_err(|error| Error::new("UNSAFE_PATH", error.to_string()))?;
    // Keep from the outset: even a partial move must never be auto-deleted.
    let quarantine = tempfile::Builder::new()
        .prefix("index-corrupt-")
        .tempdir_in(root)?
        .keep();
    let destination = OpenAt2Root::open(&quarantine, MountPolicy::SameMount)
        .map_err(|error| Error::new("UNSAFE_PATH", error.to_string()))?;
    for (name, file) in files {
        let current = crate::disk::open_regular(&root.join(name))?;
        if FileIdentity::from_fd(&file)? != FileIdentity::from_fd(&current)? {
            return Err(Error::new(
                "UNSAFE_PATH",
                "index identity changed before quarantine",
            ));
        }
        rustix::fs::renameat_with(
            source.directory(),
            name,
            destination.directory(),
            name,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(std::io::Error::from)?;
        let moved = crate::disk::open_regular(&quarantine.join(name))?;
        if FileIdentity::from_fd(&file)? != FileIdentity::from_fd(&moved)? {
            return Err(Error::new(
                "UNSAFE_PATH",
                "index identity changed during quarantine",
            ));
        }
    }
    destination.directory().sync_all()?;
    source.directory().sync_all()?;
    Ok(())
}

impl Engine {
    pub(crate) fn rebuild_index(&self) -> Result<()> {
        let mut proof = self.history_query_proof.lock().unwrap();
        proof.healthy = false;
        let delta = Delta::between(None, &self.state)?;
        checked_index_files(&self.root)?;
        let path = self.root.join("index.sqlite");
        if let Err(error) = try_project_delta(&path, None, &self.state, &delta) {
            if !error.corrupt {
                return Err(error.into());
            }
            quarantine_corrupt_index(&self.root)?;
            // Only recovered durable State supplies the replacement projection.
            project_delta(&path, None, &self.state, &delta)?;
        }
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
        if let Err(error) = checked_index_files(&self.root) {
            proof.healthy = false;
            return Err(error);
        }
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
        if checked_index_files(&self.root).is_err() {
            proof.healthy = false;
            return None;
        }
        let expected = proof.counts.get(file_id).copied().unwrap_or(0);
        let limit = i64::try_from(expected).ok()?.checked_add(1)?;
        let result = platform_io::sqlite::block_on_sqlite_connection(async {
            let mut c =
                platform_io::sqlite::open_index_read_connection(&self.root.join("index.sqlite"))
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
                    let actual = sqlite_access::query("SELECT type,sql FROM sqlite_schema WHERE name=?")
                        .bind(name).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
                    if actual.try_get::<String, _>(0).map_err(|e| e.to_string())? != kind
                        || actual.try_get::<String, _>(1).map_err(|e| e.to_string())? != ddl {
                        return Err("query schema mismatch".into());
                    }
                }
                let sequence: String = sqlite_access::query_scalar("SELECT sequence FROM current_state WHERE singleton=1")
                    .fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
                if sequence != self.state.sequence.to_string() { return Err("stale sequence".into()); }
                let rows = sqlite_access::query("SELECT version_id,file_id,history_tree_id,version_sequence_key FROM file_history_versions INDEXED BY file_history_lookup WHERE file_id=? ORDER BY version_sequence_key,version_id LIMIT ?")
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
#[path = "../../../../test/engine/unit/engine/index/tests.rs"]
mod tests;
