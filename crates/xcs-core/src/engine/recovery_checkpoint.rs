//! Conservative durable replay checkpoint. Journals remain authoritative.
//!
//! A checkpoint is used only with three trusted members and an exact match of
//! journal tips and the recovery-relevant disk inventory. Any change falls back
//! to ordinary replay/repair. This bounds historical state decoding, not the
//! journal/inventory verification work or total startup time.
use super::Engine;
use crate::{disk::*, journal::*, *};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Component, Path},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Checkpoint {
    format_version: u32,
    config_hash: String,
    pub sequence: u64,
    pub candidate_hash: String,
    pub snapshot_hash: String,
    tips: BTreeMap<String, String>,
    coverage_hash: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    checkpoint: Checkpoint,
    hash: String,
}
fn path(d: &DiskConfig) -> std::path::PathBuf {
    d.root.join("metadata/replay-checkpoint.json")
}
fn invalid() -> Error {
    Error::new("CHECKPOINT_INVALID", "replay checkpoint coverage differs")
}

// Read every repair-relevant byte, not metadata timestamps. Published payload
// modern named-version bytes are excluded: historical recovery reconstructs
// their proof but does not reverify them. Legacy/content bytes ARE hashed because
// synchronize_names revalidates them during legacy migration, even after a move.
fn payload(relative: &Path) -> bool {
    let parts = relative
        .components()
        .filter_map(|c| match c {
            Component::Normal(p) => p.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    if parts.first() != Some(&"histories") {
        return false;
    }
    parts.get(2) == Some(&"file-containers") && parts.get(5).is_some_and(|p| *p == "versions")
}
fn inventory(root: &Path, relative: &Path, h: &mut blake3::Hasher) -> Result<()> {
    let absolute = root.join(relative);
    let metadata = std::fs::symlink_metadata(&absolute)?;
    if metadata.file_type().is_symlink() || !(metadata.is_dir() || metadata.is_file()) {
        return Err(invalid());
    }
    let name = relative.to_str().ok_or_else(invalid)?;
    h.update(&(name.len() as u64).to_le_bytes());
    h.update(name.as_bytes());
    if metadata.is_dir() {
        h.update(b"directory");
        let mut entries = std::fs::read_dir(&absolute)?
            .map(|entry| entry.map(|e| e.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort();
        for entry in entries {
            let child = relative.join(entry);
            if child == Path::new("metadata/application-log")
                || child == Path::new("metadata/replay-checkpoint.json")
            {
                continue;
            }
            inventory(root, &child, h)?;
        }
    } else {
        h.update(b"file");
        h.update(&metadata.len().to_le_bytes());
        if !payload(relative) {
            let mut file = open_regular(&absolute)?;
            let mut bytes = [0; 64 * 1024];
            loop {
                let count = file.read(&mut bytes)?;
                if count == 0 {
                    break;
                }
                h.update(&bytes[..count]);
            }
        }
    }
    Ok(())
}
fn coverage(engine: &Engine) -> Result<String> {
    let mut h = blake3::Hasher::new();
    h.update(b"xcs-replay-coverage-v1\0");
    for d in &engine.config.disks {
        validate_seal(&engine.config, d)?;
        h.update(&(d.disk_id.len() as u64).to_le_bytes());
        h.update(d.disk_id.as_bytes());
        inventory(&d.root, Path::new("metadata"), &mut h)?;
        inventory(&d.root, Path::new("histories"), &mut h)?;
    }
    Ok(h.finalize().to_hex().to_string())
}

pub(super) fn load(engine: &Engine, tips: &BTreeMap<String, String>) -> Option<Checkpoint> {
    let members = engine.members();
    let mut chosen = None;
    for d in &members {
        let envelope: Envelope = read_json(&path(d)).ok()?;
        let checkpoint = envelope.checkpoint;
        if checkpoint.format_version != 1
            || checkpoint.config_hash != object_hash("replay-config", &engine.config).ok()?
            || checkpoint.tips != *tips
            || envelope.hash != object_hash("replay-checkpoint", &checkpoint).ok()?
            || chosen.as_ref().is_some_and(|c| c != &checkpoint)
        {
            return None;
        }
        chosen = Some(checkpoint);
    }
    let checkpoint = chosen?;
    if coverage(engine).ok()? != checkpoint.coverage_hash {
        return None;
    }
    Some(checkpoint)
}

pub(super) fn save(engine: &Engine) -> Result<()> {
    if engine.blocked || engine.snapshot.is_none() {
        return Ok(());
    }
    let mut tips = BTreeMap::new();
    let mut candidate = None;
    for d in engine.members() {
        validate_seal(&engine.config, &d)?;
        let scanned = scan(&d)?;
        if scanned.torn_tail {
            return Ok(());
        }
        let Some((last, hash)) = scanned.records.last() else {
            return Ok(());
        };
        tips.insert(d.disk_id.clone(), hash.clone());
        let published = scanned
            .records
            .iter()
            .rev()
            .find(|(r, _)| r.record_type == "PUBLICATION")
            .map(|(r, _)| (r.sequence, r.candidate_hash.clone()));
        if candidate
            .as_ref()
            .is_some_and(|c| Some(c) != published.as_ref())
        {
            return Ok(());
        }
        candidate = published;
        if last.sequence > engine.state.sequence {
            return Ok(());
        }
    }
    let Some((sequence, candidate_hash)) = candidate else {
        return Ok(());
    };
    let checkpoint = Checkpoint {
        format_version: 1,
        config_hash: object_hash("replay-config", &engine.config)?,
        sequence,
        candidate_hash,
        snapshot_hash: engine.snapshot.clone().ok_or_else(invalid)?,
        tips,
        coverage_hash: coverage(engine)?,
    };
    let envelope = Envelope {
        hash: object_hash("replay-checkpoint", &checkpoint)?,
        checkpoint,
    };
    for d in engine.members() {
        write_json(&path(&d), &envelope, true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn setup() -> (tempfile::TempDir, std::path::PathBuf, Config) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("system");
        let config = Config::development([
            temp.path().join("a"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&root, config.clone()).unwrap();
        for n in 0..6 {
            engine
                .create_repository(&format!("repo{n}"), "P01", 3, "admin", &new_id("op"))
                .unwrap();
        }
        drop(engine);
        (temp, root, config)
    }
    fn reopen(root: &Path) -> (Engine, usize) {
        SNAPSHOT_LOAD_COUNT.with(|count| count.set(0));
        let engine = Engine::open(root).unwrap();
        let count = SNAPSHOT_LOAD_COUNT.with(|count| count.get());
        (engine, count)
    }
    #[test]
    fn same_length_legacy_payload_changes_invalidate_coverage() {
        let (_temp, root, config) = setup();
        let legacy = config.disks[0]
            .root
            .join("histories/history_test/file-containers/file_test/name/legacy/ver_test/body");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"good").unwrap();
        drop(reopen(&root).0);
        std::fs::write(&legacy, b"evil").unwrap();
        assert!(
            reopen(&root).1 >= 6,
            "legacy bytes must invalidate the skip, even at equal length"
        );
        assert!(!payload(Path::new(
            "histories/history_test/versions/ver_test/content/body"
        )));
        assert!(!payload(Path::new(
            "histories/history_test/file-containers/file_test/name/legacy/ver_test/body"
        )));
    }

    #[test]
    fn clean_checkpoint_reopens_decode_one_snapshot_and_preserve_state() {
        let (_temp, root, config) = setup();
        let (engine, full) = reopen(&root);
        let expected = canonical_json(&engine.state).unwrap();
        assert!(full >= 6);
        assert!(config.disks.iter().all(|d| path(d).exists()));
        drop(engine);
        let (engine, fast) = reopen(&root);
        assert_eq!(fast, 1);
        assert_eq!(canonical_json(&engine.state).unwrap(), expected);
    }
    #[test]
    fn missing_historical_page_falls_back_and_repairs_replica() {
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
        // Recovery selects the first member in disk-id order, not config order.
        // Remove a replica dependency, leaving the selected source intact.
        let source = config.disks.iter().max_by_key(|d| &d.disk_id).unwrap();
        let first = scan(source)
            .unwrap()
            .records
            .into_iter()
            .find(|(r, _)| r.record_type == "PREPARE")
            .unwrap()
            .0;
        let candidate: Candidate = load_object(source, "candidate", &first.candidate_hash).unwrap();
        let snapshot: Snapshot = load_object(source, "snapshot", &candidate.snapshot_hash).unwrap();
        let page = snapshot
            .pages
            .values()
            .find_map(|pages| pages.first())
            .unwrap();
        let missing = object_path(source, "state-page", page);
        assert!(missing.exists());
        std::fs::remove_file(&missing).unwrap();
        let (engine, count) = reopen(&root);
        assert!(count >= 6);
        assert!(missing.exists());
        drop(engine);
        assert_eq!(reopen(&root).1, 1);
    }
    #[test]
    fn missing_local_publication_proof_falls_back_and_repairs() {
        let (_temp, root, config) = setup();
        let mut engine = Engine::open(&root).unwrap();
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
                "example.txt",
                b"checkpoint payload",
            )
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "checkpoint proof fixture".into(),
                request_digest: None,
            })
            .unwrap();
        drop(engine);
        drop(reopen(&root).0);
        fn find_proof(root: &Path) -> Option<std::path::PathBuf> {
            for entry in std::fs::read_dir(root).ok()? {
                let path = entry.ok()?.path();
                if path
                    .file_name()
                    .is_some_and(|n| n == "publication-proof.json")
                {
                    return Some(path);
                }
                if path.is_dir() {
                    if let Some(proof) = find_proof(&path) {
                        return Some(proof);
                    }
                }
            }
            None
        }
        let proof = find_proof(&config.disks[1].root.join("histories")).unwrap();
        std::fs::remove_file(&proof).unwrap();
        let (engine, count) = reopen(&root);
        assert!(count >= 7);
        assert!(proof.exists());
        drop(engine);
        assert_eq!(reopen(&root).1, 1);
    }

    #[test]
    fn malformed_or_partially_replicated_checkpoint_falls_back() {
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
        std::fs::write(path(&config.disks[0]), b"not a checkpoint").unwrap();
        let (engine, count) = reopen(&root);
        assert!(count >= 6);
        drop(engine);
        assert_eq!(reopen(&root).1, 1);
    }
    #[test]
    fn checkpoint_never_hides_corrupt_journal_prefix() {
        use std::io::{Seek, SeekFrom};
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(log_path(&config.disks[0]))
            .unwrap();
        file.seek(SeekFrom::Start(20)).unwrap();
        file.write_all(b"!").unwrap();
        let error = Engine::open(&root)
            .err()
            .expect("corrupt authoritative journal must fail");
        assert_eq!(error.code, "LOG_CORRUPT");
    }
    #[test]
    fn checkpoint_survives_system_disk_loss_without_becoming_authority() {
        let (_temp, root, config) = setup();
        let (engine, _) = reopen(&root);
        let expected = canonical_json(&engine.state).unwrap();
        drop(engine);
        std::fs::remove_dir_all(&root).unwrap();
        let disks = config
            .disks
            .iter()
            .map(|d| d.root.clone())
            .collect::<Vec<_>>();
        let engine = Engine::restore_from_disks(&root, &disks).unwrap();
        assert_eq!(canonical_json(&engine.state).unwrap(), expected);
    }

    #[test]
    fn degraded_open_keeps_full_replay_and_visibility_rules() {
        let (_temp, root, config) = setup();
        let (engine, _) = reopen(&root);
        let expected = canonical_json(&engine.state).unwrap();
        drop(engine);
        std::fs::rename(
            &config.disks[2].root,
            config.disks[2].root.with_extension("offline"),
        )
        .unwrap();
        let (engine, count) = reopen(&root);
        assert!(count >= 6);
        assert!(engine.blocked);
        assert_eq!(canonical_json(&engine.state).unwrap(), expected);
    }

    #[test]
    fn new_decision_invalidates_checkpoint_and_then_refreshes_it() {
        let (_temp, root, _config) = setup();
        let (mut engine, _) = reopen(&root);
        engine
            .create_repository("newer", "P01", 3, "admin", &new_id("op"))
            .unwrap();
        let expected = canonical_json(&engine.state).unwrap();
        drop(engine);
        let (engine, count) = reopen(&root);
        assert!(count >= 7);
        assert_eq!(canonical_json(&engine.state).unwrap(), expected);
        drop(engine);
        assert_eq!(reopen(&root).1, 1);
    }

    #[test]
    fn torn_journal_tail_disables_checkpoint_replay_skip() {
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(log_path(&config.disks[0]))
            .unwrap();
        file.write_all(b"XCSLO").unwrap();
        let (_, count) = reopen(&root);
        assert!(count >= 6);
    }
}
