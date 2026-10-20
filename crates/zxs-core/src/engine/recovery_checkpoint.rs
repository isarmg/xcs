//! Conservative durable replay checkpoint. Journals remain authoritative.
//!
//! Three trusted members must agree on a checkpoint whose tips occur in the
//! fully verified journal prefixes. Covered metadata/proofs and sensitive layout
//! are reverified; unrelated append-only additions permit normal suffix replay.
//! Missing or changed covered dependencies fall back to full repair. This bounds
//! historical state decoding, not verification work or total startup time.
use super::Engine;
use crate::{disk::*, journal::*, *};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    coverage: BTreeMap<String, Vec<CoverageEntry>>,
    version_ids: BTreeSet<String>,
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
fn named_container(root: &Path, relative: &Path) -> Result<bool> {
    let parts = relative_parts(relative)?;
    if parts.len() < 5 || parts[0] != "histories" || parts[2] != "file-containers" {
        return Ok(false);
    }
    // Original direct-name layout has no namespace owner marker. Namespaced
    // containers may be arbitrarily deep and must carry their exact file ID.
    if parts.len() == 5 && parts[4] != "namespace" {
        return Ok(true);
    }
    Ok(
        read_json::<String>(&root.join(relative).join(".zxs-container-owner.json"))
            .is_ok_and(|id| id == parts[3]),
    )
}
fn payload(root: &Path, relative: &Path) -> Result<bool> {
    let Some(parent) = relative.parent() else {
        return Ok(false);
    };
    if parent.file_name().is_none_or(|name| name != "versions") {
        return Ok(false);
    }
    let Some(container) = parent.parent() else {
        return Ok(false);
    };
    named_container(root, container)
}
const MAX_CHECKPOINT_BYTES: usize = 16 * 1024 * 1024;
const MAX_COVERAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_COVERAGE_ENTRIES: usize = 32_768;
pub(super) type Records = BTreeMap<u64, BTreeMap<String, Vec<(String, Record)>>>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CoverageEntry {
    path: String,
    value: Covered,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Covered {
    Directory {
        children: Option<Vec<String>>,
    },
    File {
        length: u64,
        hash: Option<String>,
        links: u64,
    },
    Absent,
}
fn relative_parts(path: &Path) -> Result<Vec<&str>> {
    path.components()
        .map(|part| match part {
            Component::Normal(name) => name.to_str().ok_or_else(invalid),
            _ => Err(invalid()),
        })
        .collect()
}
fn excluded(path: &Path) -> bool {
    path == Path::new("metadata/application-log")
        || path == Path::new("metadata/replay-checkpoint.json")
}
fn append_only(root: &Path, path: &Path) -> Result<bool> {
    let parts = relative_parts(path)?;
    if path.file_name().is_some_and(|name| name == "versions")
        && path
            .parent()
            .is_some_and(|parent| named_container(root, parent).unwrap_or(false))
    {
        return Ok(true);
    }
    Ok(
        (parts.first() == Some(&"metadata") && parts.get(1) == Some(&"objects"))
            || path == Path::new("metadata/publication-seals")
            || (parts.first() == Some(&"histories")
                && (parts.len() <= 2
                    || (parts.len() == 3 && matches!(parts[2], "versions" | "file-containers"))
                    || (parts.len() == 5
                        && parts[2] == "versions"
                        && parts[4] == "protection-proofs"))),
    )
}
fn children(root: &Path, path: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut bytes = 0;
    for entry in std::fs::read_dir(root.join(path))? {
        let name = entry?.file_name().into_string().map_err(|_| invalid())?;
        if excluded(&path.join(&name)) {
            continue;
        }
        bytes += name.len();
        if out.len() >= MAX_COVERAGE_ENTRIES || bytes > MAX_COVERAGE_BYTES {
            return Err(invalid());
        }
        out.push(name);
    }
    out.sort();
    Ok(out)
}
fn file_hash(path: &Path) -> Result<String> {
    let mut file = open_regular(path)?;
    let mut h = blake3::Hasher::new();
    let mut bytes = [0; 64 * 1024];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        h.update(&bytes[..count]);
    }
    Ok(h.finalize().to_hex().to_string())
}
fn add_entry(out: &mut Vec<CoverageEntry>, entry: CoverageEntry, budget: &mut usize) -> Result<()> {
    *budget = budget
        .checked_add(canonical_json(&entry)?.len())
        .ok_or_else(invalid)?;
    if *budget > MAX_COVERAGE_BYTES || out.len() >= MAX_COVERAGE_ENTRIES {
        return Err(invalid());
    }
    out.push(entry);
    Ok(())
}
fn inventory(
    root: &Path,
    relative: &Path,
    out: &mut Vec<CoverageEntry>,
    budget: &mut usize,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(root.join(relative))?;
    if metadata.file_type().is_symlink() || !(metadata.is_dir() || metadata.is_file()) {
        return Err(invalid());
    }
    let name = relative.to_str().ok_or_else(invalid)?.to_owned();
    if metadata.is_dir() {
        let names = children(root, relative)?;
        add_entry(
            out,
            CoverageEntry {
                path: name,
                value: Covered::Directory {
                    children: if append_only(root, relative)? {
                        None
                    } else {
                        Some(names.clone())
                    },
                },
            },
            budget,
        )?;
        for child in names {
            inventory(root, &relative.join(child), out, budget)?;
        }
    } else {
        add_entry(
            out,
            CoverageEntry {
                path: name,
                value: Covered::File {
                    length: metadata.len(),
                    links: metadata.nlink(),
                    hash: if payload(root, relative)? {
                        None
                    } else {
                        Some(file_hash(&root.join(relative))?)
                    },
                },
            },
            budget,
        )?;
    }
    Ok(())
}
fn coverage(
    engine: &Engine,
    absent: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeMap<String, Vec<CoverageEntry>>> {
    let mut out = BTreeMap::new();
    let mut budget = 0;
    for d in &engine.config.disks {
        validate_seal(&engine.config, d)?;
        let mut entries = Vec::new();
        inventory(&d.root, Path::new("metadata"), &mut entries, &mut budget)?;
        inventory(&d.root, Path::new("histories"), &mut entries, &mut budget)?;
        for path in absent.get(&d.disk_id).into_iter().flatten() {
            add_entry(
                &mut entries,
                CoverageEntry {
                    path: path.clone(),
                    value: Covered::Absent,
                },
                &mut budget,
            )?;
        }
        out.insert(d.disk_id.clone(), entries);
    }
    Ok(out)
}
fn suffix_rebuilt_containers(
    engine: &Engine,
    checkpoint: &Checkpoint,
    records: &Records,
) -> Result<BTreeSet<(String, String)>> {
    let mut out = BTreeSet::new();
    let mut seen = checkpoint.version_ids.clone();
    let mut previous = Some(checkpoint.snapshot_hash.clone());
    for (sequence, candidates) in records.range((
        std::ops::Bound::Excluded(checkpoint.sequence),
        std::ops::Bound::Unbounded,
    )) {
        let active = candidates
            .iter()
            .filter(|(_, records)| {
                records
                    .iter()
                    .any(|(_, r)| matches!(r.record_type.as_str(), "ACCEPT" | "PUBLICATION"))
            })
            .collect::<Vec<_>>();
        if active.len() > 1 {
            return Err(invalid());
        }
        let Some((hash, records)) = active.first() else {
            continue;
        };
        let Some((slot, r)) = records.iter().find(|(_, r)| r.record_type == "PUBLICATION") else {
            break;
        };
        let disk = engine.disk(slot)?;
        let c: Candidate = load_object(disk, "candidate", hash)?;
        let p: Publication = load_object(disk, "publication", &r.object_hash)?;
        if c.pool_id != engine.config.pool_id
            || c.sequence != *sequence
            || c.previous_snapshot != previous
        {
            return Err(invalid());
        }
        // Only a continuous, fully sealed suffix may waive checking a derived
        // container cache. Pending/forked decisions never authorize exceptions.
        let sealed = engine.members().iter().all(|d| {
            read_json::<PublicationSeal>(&publication_seal_path(d, hash))
                .and_then(|seal| verify_publication_seal(&c, &p, &seal))
                .is_ok()
        });
        if !sealed {
            break;
        }
        previous = Some(c.snapshot_hash.clone());
        for evidence in &c.versions {
            if seen.insert(evidence.version.version_id.clone()) && p.recovery.is_none() {
                out.insert((
                    evidence.version.history_tree_id.clone(),
                    evidence.version.file_id.clone(),
                ));
            }
        }
    }
    Ok(out)
}
fn verify_coverage(
    engine: &Engine,
    checkpoint: &Checkpoint,
    rebuilt: &BTreeSet<(String, String)>,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    if checkpoint.coverage.len() != engine.config.disks.len() {
        return Err(invalid());
    }
    for d in &engine.config.disks {
        validate_seal(&engine.config, d)?;
        let entries = checkpoint.coverage.get(&d.disk_id).ok_or_else(invalid)?;
        if entries.len() > MAX_COVERAGE_ENTRIES {
            return Err(invalid());
        }
        for entry in entries {
            let path = Path::new(&entry.path);
            let parts = relative_parts(path)?;
            if !parts
                .first()
                .is_some_and(|p| matches!(*p, "metadata" | "histories"))
            {
                return Err(invalid());
            }
            if parts.len() >= 6
                && parts[0] == "histories"
                && parts[2] == "file-containers"
                && parts.last() == Some(&"container.json")
                && path
                    .parent()
                    .is_some_and(|parent| named_container(&d.root, parent).unwrap_or(false))
                && rebuilt.contains(&(parts[1].to_owned(), parts[3].to_owned()))
            {
                continue;
            }
            let metadata = std::fs::symlink_metadata(d.root.join(path));
            match (&entry.value, metadata) {
                (Covered::Absent, Err(e)) if e.kind() == std::io::ErrorKind::NotFound => (),
                (Covered::Directory { children: expected }, Ok(m))
                    if m.is_dir() && !m.file_type().is_symlink() =>
                {
                    if let Some(expected) = expected {
                        if children(&d.root, path)? != *expected {
                            return Err(invalid());
                        }
                    } else if !append_only(&d.root, path)? {
                        return Err(invalid());
                    }
                }
                (
                    Covered::File {
                        length,
                        hash,
                        links,
                    },
                    Ok(m),
                ) if m.is_file() && !m.file_type().is_symlink() => {
                    if m.len() != *length || m.nlink() != *links {
                        return Err(invalid());
                    }
                    match hash {
                        Some(hash) if file_hash(&d.root.join(path))? == *hash => {
                            // Append-only directories may gain an alternate layout for an
                            // already covered hash. That addition must not hide conflicting
                            // evidence behind the checkpoint's historical replay shortcut.
                            if parts.first() == Some(&"metadata")
                                && parts.get(1) == Some(&"objects")
                            {
                                let kind = parts.get(2).ok_or_else(invalid)?;
                                let object_hash = parts
                                    .last()
                                    .and_then(|name| name.strip_suffix(".json"))
                                    .ok_or_else(invalid)?;
                                let paths = existing_object_paths(d, kind, object_hash)?;
                                let covered = d.root.join(path);
                                if !paths.contains(&covered) {
                                    return Err(invalid());
                                }
                                for alternate in paths {
                                    if alternate != covered && file_hash(&alternate)? != *hash {
                                        return Err(invalid());
                                    }
                                }
                            }
                        }
                        None if payload(&d.root, path)? => (),
                        _ => return Err(invalid()),
                    }
                }
                _ => return Err(invalid()),
            }
        }
    }
    Ok(())
}
pub(super) fn load(
    engine: &Engine,
    prefixes: &BTreeMap<String, BTreeSet<String>>,
    records: &Records,
) -> Option<Checkpoint> {
    let members = engine.members();
    let mut chosen = None;
    for d in &members {
        let envelope: Envelope =
            decode_strict(&read_bytes(&path(d), MAX_CHECKPOINT_BYTES).ok()?).ok()?;
        let checkpoint = envelope.checkpoint;
        if checkpoint.format_version != 2
            || checkpoint.config_hash != object_hash("replay-config", &engine.config).ok()?
            || checkpoint.tips.len() != members.len()
            || !checkpoint
                .tips
                .iter()
                .all(|(id, tip)| prefixes.get(id).is_some_and(|hashes| hashes.contains(tip)))
            || envelope.hash != object_hash("replay-checkpoint", &checkpoint).ok()?
            || chosen.as_ref().is_some_and(|c| c != &checkpoint)
        {
            return None;
        }
        chosen = Some(checkpoint);
    }
    let checkpoint = chosen?;
    let rebuilt = suffix_rebuilt_containers(engine, &checkpoint, records).ok()?;
    verify_coverage(engine, &checkpoint, &rebuilt).ok()?;
    Some(checkpoint)
}

pub(super) fn save(engine: &Engine) -> Result<()> {
    if engine.blocked || engine.snapshot.is_none() {
        return Ok(());
    }
    let mut tips = BTreeMap::new();
    let mut candidate = None;
    let mut absent = BTreeMap::<String, BTreeSet<String>>::new();
    let mut version_ids = BTreeSet::new();
    for d in engine.members() {
        validate_seal(&engine.config, &d)?;
        let scanned = scan(&d)?;
        if scanned.torn_tail {
            return Ok(());
        }
        // Preserve absences that made historical recovery skip local proof or
        // container work. Restoring such a prefix path requires full repair.
        for (record, _) in scanned
            .records
            .iter()
            .filter(|(r, _)| r.record_type == "PUBLICATION")
        {
            let c: Candidate = load_object(&d, "candidate", &record.candidate_hash)?;
            for evidence in &c.versions {
                version_ids.insert(evidence.version.version_id.clone());
                if version_ids.len() > MAX_COVERAGE_ENTRIES {
                    return Err(invalid());
                }
                for slot in &evidence.placement.route_slot_ids {
                    let target = engine.disk(slot)?;
                    let history = Path::new("histories").join(&evidence.version.history_tree_id);
                    for relative in [
                        history.clone(),
                        history.join("versions").join(&evidence.version.version_id),
                        history
                            .join("file-containers")
                            .join(&evidence.version.file_id),
                    ] {
                        match std::fs::symlink_metadata(target.root.join(&relative)) {
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                let paths = absent.entry(target.disk_id.clone()).or_default();
                                paths.insert(relative.to_str().ok_or_else(invalid)?.into());
                                if paths.len() > MAX_COVERAGE_ENTRIES {
                                    return Err(invalid());
                                }
                            }
                            Err(e) => return Err(e.into()),
                            Ok(_) => (),
                        }
                    }
                }
            }
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
        format_version: 2,
        config_hash: object_hash("replay-config", &engine.config)?,
        sequence,
        candidate_hash,
        snapshot_hash: engine.snapshot.clone().ok_or_else(invalid)?,
        tips,
        coverage: coverage(engine, &absent)?,
        version_ids,
    };
    let envelope = Envelope {
        hash: object_hash("replay-checkpoint", &checkpoint)?,
        checkpoint,
    };
    let bytes = encode_metadata(&envelope, MAX_CHECKPOINT_BYTES)?;
    for d in engine.members() {
        write_atomic(&path(&d), &bytes, true)?;
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
    fn missing_or_corrupt_selected_snapshot_source_repairs_from_verified_replica() {
        for corrupt in [false, true] {
            let (_temp, root, config) = setup();
            let (engine, _) = reopen(&root);
            let expected = canonical_json(&engine.state).unwrap();
            drop(engine);
            let source = config.disks.iter().min_by_key(|d| &d.disk_id).unwrap();
            let first = scan(source)
                .unwrap()
                .records
                .into_iter()
                .find(|(r, _)| r.record_type == "PREPARE")
                .unwrap()
                .0;
            let candidate: Candidate =
                load_object(source, "candidate", &first.candidate_hash).unwrap();
            let snapshot: Snapshot =
                load_object(source, "snapshot", &candidate.snapshot_hash).unwrap();
            let page = snapshot
                .pages
                .values()
                .find_map(|pages| pages.first())
                .unwrap();
            let path = object_path(source, "state-page", page);
            let original = std::fs::read(&path).unwrap();
            if corrupt {
                std::fs::write(&path, b"corrupt").unwrap();
            } else {
                std::fs::remove_file(&path).unwrap();
            }
            let (engine, _) = reopen(&root);
            assert_eq!(canonical_json(&engine.state).unwrap(), expected);
            assert_eq!(std::fs::read(&path).unwrap(), original);
            drop(engine);
            assert_eq!(reopen(&root).1, 1);
        }
    }
    #[test]
    fn all_snapshot_replicas_invalid_fail_closed() {
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
        let source = &config.disks[0];
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
        for disk in &config.disks {
            std::fs::remove_file(object_path(disk, "state-page", page)).unwrap();
        }
        assert!(Engine::open(&root).is_err());
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
        assert!(
            !payload(
                &config.disks[0].root,
                Path::new("histories/history_test/versions/ver_test/content/body")
            )
            .unwrap()
        );
        assert!(
            !payload(
                &config.disks[0].root,
                Path::new(
                    "histories/history_test/file-containers/file_test/name/legacy/ver_test/body"
                )
            )
            .unwrap()
        );
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
    fn added_conflicting_layout_cannot_hide_behind_checkpoint_coverage() {
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
        let source = &config.disks[0];
        let first = scan(source)
            .unwrap()
            .records
            .into_iter()
            .find(|(r, _)| r.record_type == "PUBLICATION")
            .unwrap()
            .0;
        for disk in &config.disks {
            let flat = disk
                .root
                .join("metadata/objects/candidate")
                .join(format!("{}.json", first.candidate_hash));
            std::fs::write(flat, b"{}").unwrap();
        }
        assert!(Engine::open(&root).is_err());
    }

    #[test]
    fn missing_historical_page_falls_back_and_repairs_replica() {
        let (_temp, root, config) = setup();
        drop(reopen(&root).0);
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
    fn save_file(engine: &mut Engine, branch: &str, name: &str, bytes: &[u8]) {
        let workspace = engine.create_workspace(branch, "admin").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, name, bytes)
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "checkpoint suffix fixture".into(),
                request_digest: None,
            })
            .unwrap();
    }
    #[test]
    fn restored_absent_historical_version_directory_requires_full_proof_repair() {
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
        save_file(&mut engine, &branch, "lost-proof.txt", b"body");
        let version = engine.state.versions.values().next().unwrap().clone();
        drop(engine);
        let directory = version_dir(
            &config.disks[0],
            &version.history_tree_id,
            &version.version_id,
        );
        std::fs::remove_dir_all(&directory).unwrap();
        drop(reopen(&root).0);
        // Recovery legitimately skipped the absent local directory; making it
        // exist again must not bypass reconstruction of its historical proof.
        private_dir(&directory).unwrap();
        let (_, count) = reopen(&root);
        assert!(count >= 7);
        assert!(directory.join("publication-proof.json").exists());
    }

    #[test]
    fn forked_suffix_cannot_authorize_derived_cache_exception() {
        let (_temp, root, config) = setup();
        let engine = reopen(&root).0;
        let envelope: Envelope = read_json(&path(&config.disks[0])).unwrap();
        let sequence = envelope.checkpoint.sequence + 1;
        let mut candidates = BTreeMap::new();
        for hash in ["a".repeat(64), "b".repeat(64)] {
            candidates.insert(
                hash.clone(),
                vec![(
                    config.disks[0].slot_id.clone(),
                    Record {
                        record_type: "ACCEPT".into(),
                        sequence,
                        candidate_hash: hash.clone(),
                        object_hash: hash,
                        previous_frame_hash: "0".repeat(64),
                    },
                )],
            );
        }
        assert!(
            suffix_rebuilt_containers(
                &engine,
                &envelope.checkpoint,
                &BTreeMap::from([(sequence, candidates)])
            )
            .is_err()
        );
    }

    #[test]
    fn repeated_same_file_saves_replay_only_checkpoint_and_suffix() {
        let (_temp, root, _config) = setup();
        let mut engine = Engine::open(&root).unwrap();
        let branch = engine
            .state
            .repositories
            .values()
            .next()
            .unwrap()
            .main_branch_id
            .clone();
        save_file(&mut engine, &branch, "same.txt", b"first");
        drop(engine);
        let (mut engine, _) = reopen(&root);
        for bytes in [
            b"second".as_slice(),
            b"third".as_slice(),
            b"fourth".as_slice(),
        ] {
            save_file(&mut engine, &branch, "same.txt", bytes);
        }
        let expected = canonical_json(&engine.state).unwrap();
        drop(engine);
        let (engine, count) = reopen(&root);
        assert_eq!(
            count, 4,
            "one checkpoint snapshot plus three suffix decisions"
        );
        assert_eq!(canonical_json(&engine.state).unwrap(), expected);
        drop(engine);
        assert_eq!(reopen(&root).1, 1);
    }
    #[test]
    fn corrupted_container_outside_suffix_forces_full_repair() {
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
        save_file(&mut engine, &branch, "first.txt", b"first");
        save_file(&mut engine, &branch, "other.txt", b"other");
        let other = engine
            .state
            .versions
            .values()
            .find(|v| v.creation_name == "other.txt")
            .unwrap()
            .clone();
        drop(engine);
        let (mut engine, _) = reopen(&root);
        save_file(&mut engine, &branch, "first.txt", b"new first");
        drop(engine);
        let body = crate::container_storage::content_path(
            &version_dir(&config.disks[0], &other.history_tree_id, &other.version_id),
            &other,
        )
        .unwrap();
        let cache = body
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("container.json");
        std::fs::write(&cache, b"{}").unwrap();
        let (_, count) = reopen(&root);
        assert!(
            count >= 9,
            "unrelated suffix evidence must not waive this cache"
        );
        assert!(std::fs::read(&cache).unwrap().len() > 2);
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
                if path.is_dir()
                    && let Some(proof) = find_proof(&path)
                {
                    return Some(proof);
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
    fn sealed_suffix_replays_only_new_decision_and_refreshes_checkpoint() {
        let (_temp, root, _config) = setup();
        let (mut engine, _) = reopen(&root);
        engine
            .create_repository("newer", "P01", 3, "admin", &new_id("op"))
            .unwrap();
        let expected = canonical_json(&engine.state).unwrap();
        drop(engine);
        let (engine, count) = reopen(&root);
        assert_eq!(count, 2);
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
        file.write_all(b"ZXSLO").unwrap();
        let (_, count) = reopen(&root);
        assert!(count >= 6);
    }
}
