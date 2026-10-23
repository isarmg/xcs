use super::*;
use crate::{Backend, BackendAction, engine_backend::EngineBackend};
use data_serialization::{Serializer, ser::Error};
use json_codec::json;
use std::{collections::BTreeSet, sync::Mutex};
use storage_contract::{Config, new_id};

fn fixture() -> (
    tempfile::TempDir,
    Engine,
    storage_engine::Repository,
    Identity,
) {
    let root = tempfile::tempdir().unwrap();
    let config = Config::development([
        root.path().join("d1"),
        root.path().join("d2"),
        root.path().join("d3"),
    ]);
    let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
    let repo = engine
        .create_repository("Projection", "P01", 3, "owner", &new_id("op"))
        .unwrap();
    let identity = Identity {
        admission_session: None,
        smb_binding: None,
        principal: "owner".into(),
        admin: false,
        repositories: BTreeSet::from([repo.repository_id.clone()]),
    };
    (root, engine, repo, identity)
}

#[test]
fn rendering_releases_engine_before_serializer_runs() {
    let (_root, engine, _, _) = fixture();
    let engine = Mutex::new(engine);
    struct Probe<'a>(&'a Mutex<Engine>);
    impl Serialize for Probe<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            // Runs on another thread to prove this is not reentrant access.
            let available = std::thread::scope(|scope| {
                scope.spawn(|| self.0.try_lock().is_ok()).join().unwrap()
            });
            if !available {
                return Err(S::Error::custom("serialization retained Engine mutex"));
            }
            serializer.serialize_bool(available)
        }
    }
    assert_eq!(
        render_unlocked(engine.lock().unwrap(), Probe(&engine)).unwrap(),
        json!(true)
    );
}

#[test]
fn tree_projection_is_frozen_and_preserves_authorization_and_wire_shape() {
    let (_root, mut engine, repo, identity) = fixture();
    let input = json!({"branch_id": repo.main_branch_id});
    let expected = json!({
        "branch_id": repo.main_branch_id,
        "commit_id": engine.state().branches[&repo.main_branch_id].head_commit_id,
        "entries": engine.list_tree(&repo.main_branch_id).unwrap(),
        "pending_file_versions": {},
    });
    let projection = Tree::capture(&engine, &identity, &input).unwrap();
    let mut denied = identity.clone();
    denied.repositories.clear();
    assert_eq!(
        Tree::capture(&engine, &denied, &input).err().unwrap().code,
        "FORBIDDEN"
    );
    let other = engine
        .create_repository("Other", "P01", 3, "owner", &new_id("op"))
        .unwrap();
    let foreign = &engine.state().branches[&other.main_branch_id].head_commit_id;
    assert_eq!(
        Tree::capture(
            &engine,
            &identity,
            &json!({
                "branch_id": repo.main_branch_id, "query": {"commit_id": foreign}
            })
        )
        .err()
        .unwrap()
        .code,
        "NOT_FOUND"
    );
    // The already admitted response owns its snapshot even after live changes.
    assert_eq!(encode(projection).unwrap(), expected);
    let backend = EngineBackend::new(engine);
    assert_eq!(
        backend
            .call(BackendAction::Tree, &identity, input.clone())
            .unwrap(),
        expected
    );
    assert_eq!(
        backend
            .call(BackendAction::Tree, &denied, input.clone())
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
    let frozen = {
        let mut engine = backend.engine.lock().unwrap();
        let frozen = Tree::capture(&engine, &identity, &input).unwrap();
        engine
            .set_grants(
                &repo.repository_id,
                storage_engine::Grants {
                    manage: BTreeSet::from(["replacement-owner".into()]),
                    ..Default::default()
                },
                &identity.principal,
                &new_id("op"),
            )
            .unwrap();
        frozen
    };
    assert_eq!(encode(frozen).unwrap(), expected);
    // Frozen response data never grants admission to a later request.
    assert_eq!(
        backend
            .call(BackendAction::Tree, &identity, input)
            .unwrap_err()
            .code,
        "FORBIDDEN"
    );
}

// Exact pre-optimization behavior used to compare the request-local resolver.
fn legacy_pending_entry_source<'a>(
    engine: &'a Engine,
    branch: &str,
    commit: &'a storage_contract::Commit,
    entry: &storage_contract::Entry,
) -> Option<&'a str> {
    if entry.kind != "file" {
        return None;
    }
    let state = engine.state();
    let visible = commit.entries.iter().find(|visible| {
        visible.lookup_key == entry.lookup_key && visible.file_id == entry.file_id
    })?;
    let current = state.branches.get(branch)?;
    if current.head_commit_id == commit.commit_id {
        if let Some(repo) = state
            .repositories
            .values()
            .find(|repo| repo.main_branch_id == branch)
            && let Some(context) = engine.file_context(&repo.repository_id, &entry.path)
        {
            return if context.inherited_from.is_some() {
                Some(visible.version_id.as_str())
            } else {
                state
                    .branches
                    .get(&context.branch_id)?
                    .pending_file_versions
                    .get(&entry.file_id)
                    .map(String::as_str)
            };
        }
    } else if let Some(context) = state
        .directory_trees
        .values()
        .filter(|tree| tree.commit_id == commit.commit_id)
        .find_map(|tree| tree.file_contexts.get(&entry.lookup_key))
    {
        return context
            .inherited_from
            .as_ref()
            .map(|_| visible.version_id.as_str());
    }
    commit
        .pending_file_versions
        .get(&entry.file_id)
        .filter(|version| **version == entry.version_id)
        .map(String::as_str)
        .or_else(|| {
            (current.head_commit_id == commit.commit_id)
                .then(|| engine.pending_file_source(branch, &entry.file_id))
                .flatten()
        })
}

fn synthetic_commit(
    engine: &Engine,
    repo: &storage_engine::Repository,
    count: usize,
) -> storage_contract::Commit {
    let head = &engine.state().branches[&repo.main_branch_id].head_commit_id;
    let mut commit = engine.state().commits[head].clone();
    commit.commit_id = "historical-projection-fixture".into();
    commit.entries = (0..count)
        .map(|index| {
            let path = format!("file-{index:05}.txt");
            storage_contract::Entry {
                entry_id: format!("entry-{index}"),
                name: path.clone(),
                lookup_key: path.clone(),
                path,
                kind: "file".into(),
                file_id: format!("file-{index}"),
                version_id: format!("version-{index}"),
                size: index as u64,
            }
        })
        .collect();
    commit.pending_file_versions = commit
        .entries
        .iter()
        .map(|entry| (entry.file_id.clone(), entry.version_id.clone()))
        .collect();
    commit
}

#[test]
fn canonical_tree_resolution_eliminates_repeated_entry_matching_at_scale() {
    let (_root, engine, repo, _) = fixture();
    for count in [100, 1_000, 10_000] {
        let commit = synthetic_commit(&engine, &repo, count);
        let resolver = PendingSources::new(&engine, &repo.main_branch_id, &commit);
        let optimized = resolver.pending_versions();
        assert_eq!(
            resolver.matching_visits(),
            0,
            "known entries need no search"
        );
        let checked: BTreeMap<_, _> = commit
            .entries
            .iter()
            .filter_map(|entry| {
                resolver
                    .checked(entry)
                    .map(|source| (entry.file_id.clone(), source.to_owned()))
            })
            .collect();
        assert_eq!(resolver.matching_visits(), count * (count + 1) / 2);
        assert_eq!(optimized, checked);
        assert_eq!(optimized, commit.pending_file_versions);
        // The exact original implementation remains the semantic oracle, not a
        // timing threshold. Its loop is also exercised at every requested size.
        let legacy: BTreeMap<_, _> = commit
            .entries
            .iter()
            .filter_map(|entry| {
                legacy_pending_entry_source(&engine, &repo.main_branch_id, &commit, entry)
                    .map(|source| (entry.file_id.clone(), source.to_owned()))
            })
            .collect();
        assert_eq!(optimized, legacy);
    }
}

#[test]
fn checked_entries_and_noncanonical_snapshots_preserve_original_first_match() {
    let (_root, engine, repo, _) = fixture();
    let mut commit = synthetic_commit(&engine, &repo, 3);
    let mut duplicate = commit.entries[0].clone();
    duplicate.version_id = "different-version".into();
    commit.entries.insert(1, duplicate.clone());
    commit.entries.swap(2, 3);
    let resolver = PendingSources::new(&engine, &repo.main_branch_id, &commit);
    let expected: BTreeMap<_, _> = commit
        .entries
        .iter()
        .filter_map(|entry| {
            legacy_pending_entry_source(&engine, &repo.main_branch_id, &commit, entry)
                .map(|source| (entry.file_id.clone(), source.to_owned()))
        })
        .collect();
    assert_eq!(resolver.pending_versions(), expected);
    for entry in &commit.entries {
        assert_eq!(
            resolver.checked(entry),
            legacy_pending_entry_source(&engine, &repo.main_branch_id, &commit, entry)
        );
    }
    duplicate.file_id = "absent-file".into();
    assert_eq!(resolver.checked(&duplicate), None);
    duplicate.kind = "directory".into();
    assert_eq!(resolver.checked(&duplicate), None);
    assert!(
        resolver.matching_visits() > 0,
        "noncanonical input keeps checked lookup"
    );
}

fn save_projection_file(engine: &mut Engine, branch: &str, path: &str, body: &[u8]) {
    let workspace = engine.create_workspace(branch, "owner").unwrap();
    engine
        .stage_bytes(&workspace.workspace_id, path, body)
        .unwrap();
    engine
        .commit_workspace(storage_engine::CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "owner".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: String::new(),
            request_digest: None,
        })
        .unwrap();
}

#[test]
fn real_current_and_historical_tree_sources_match_previous_wire_output() {
    let (_root, mut engine, repo, identity) = fixture();
    save_projection_file(
        &mut engine,
        &repo.main_branch_id,
        "folder/data.txt",
        b"first",
    );
    let first = engine.state().branches[&repo.main_branch_id]
        .head_commit_id
        .clone();
    save_projection_file(
        &mut engine,
        &repo.main_branch_id,
        "folder/data.txt",
        b"second",
    );
    let current = engine.state().branches[&repo.main_branch_id]
        .head_commit_id
        .clone();
    for commit_id in [&first, &current] {
        let commit = &engine.state().commits[commit_id];
        let expected: BTreeMap<_, _> = commit
            .entries
            .iter()
            .filter_map(|entry| {
                legacy_pending_entry_source(&engine, &repo.main_branch_id, commit, entry)
                    .map(|source| (entry.file_id.clone(), source.to_owned()))
            })
            .collect();
        let projection = Tree::capture(
            &engine,
            &identity,
            &json!({
                "branch_id": repo.main_branch_id, "query": {"commit_id": commit_id}
            }),
        )
        .unwrap();
        assert_eq!(projection.pending_file_versions, expected);
        assert_eq!(projection.entries, commit.entries);
        assert_eq!(
            encode(projection).unwrap(),
            json!({
                "branch_id": repo.main_branch_id,
                "commit_id": commit_id,
                "entries": commit.entries,
                "pending_file_versions": expected,
            })
        );
        for entry in &commit.entries {
            assert_eq!(
                PendingSources::new(&engine, &repo.main_branch_id, commit).checked(entry),
                legacy_pending_entry_source(&engine, &repo.main_branch_id, commit, entry)
            );
            if commit_id == &current {
                assert_eq!(
                    engine
                        .file_context_for_current_entry(&repo.repository_id, entry)
                        .map(|context| context.into_owned()),
                    engine.file_context(&repo.repository_id, &entry.path)
                );
            }
        }
    }
}

#[test]
fn pending_main_file_and_historical_branch_sources_match_previous_response() {
    let (_root, mut engine, repo, identity) = fixture();
    save_projection_file(
        &mut engine,
        &repo.main_branch_id,
        "data.txt",
        b"parent bytes",
    );
    let original = engine
        .list_tree(&repo.main_branch_id)
        .unwrap()
        .into_iter()
        .find(|entry| entry.kind == "file")
        .unwrap();
    let created = engine
        .create_file_branch(
            &original.file_id,
            &original.version_id,
            "pending-view",
            "owner",
            &new_id("op"),
        )
        .unwrap();
    let pending = created.result["branch_id"].as_str().unwrap().to_owned();
    engine
        .select_file_branch(&original.file_id, &pending, "owner", &new_id("op"))
        .unwrap();
    let old_pending = engine.state().branches[&pending].head_commit_id.clone();
    let assert_sources = |engine: &Engine, branch: &str, commit_id: &str| {
        let commit = &engine.state().commits[commit_id];
        let legacy: BTreeMap<_, _> = commit
            .entries
            .iter()
            .filter_map(|entry| {
                legacy_pending_entry_source(engine, branch, commit, entry)
                    .map(|source| (entry.file_id.clone(), source.to_owned()))
            })
            .collect();
        let tree = Tree::capture(
            engine,
            &identity,
            &json!({
                "branch_id": branch, "query": {"commit_id": commit_id}
            }),
        )
        .unwrap();
        assert_eq!(tree.pending_file_versions, legacy);
        assert_eq!(tree.entries, commit.entries);
        assert_eq!(
            encode(tree).unwrap(),
            json!({
                "branch_id": branch,
                "commit_id": commit_id,
                "entries": commit.entries,
                "pending_file_versions": legacy,
            })
        );
        legacy
    };
    for branch in [&repo.main_branch_id, &pending] {
        let commit_id = &engine.state().branches[branch].head_commit_id;
        let sources = assert_sources(&engine, branch, commit_id);
        assert_eq!(sources.get(&original.file_id), Some(&original.version_id));
    }
    save_projection_file(
        &mut engine,
        &repo.main_branch_id,
        "data.txt",
        b"written branch bytes",
    );
    let sources = assert_sources(&engine, &pending, &old_pending);
    assert_eq!(sources.get(&original.file_id), Some(&original.version_id));
    let current = &engine.state().branches[&repo.main_branch_id].head_commit_id;
    assert!(assert_sources(&engine, &repo.main_branch_id, current).is_empty());
}

#[test]
fn inherited_directory_context_keeps_first_match_and_historical_sources() {
    let (_root, mut engine, repo, identity) = fixture();
    save_projection_file(
        &mut engine,
        &repo.main_branch_id,
        "folder/data.txt",
        b"inherited bytes",
    );
    engine
        .create_directory_branches(
            &repo.repository_id,
            "folder",
            "inherited-view",
            "owner",
            &new_id("op"),
        )
        .unwrap();
    engine
        .select_directory_branches(
            &repo.repository_id,
            "folder",
            "inherited-view",
            "owner",
            &new_id("op"),
        )
        .unwrap();
    let head = engine.state().branches[&repo.main_branch_id]
        .head_commit_id
        .clone();
    let commit = &engine.state().commits[&head];
    let original = commit
        .entries
        .iter()
        .find(|entry| entry.kind == "file")
        .unwrap();
    assert!(
        engine
            .file_context(&repo.repository_id, &original.path)
            .unwrap()
            .inherited_from
            .is_some()
    );
    assert!(matches!(
        engine.file_context_for_current_entry(&repo.repository_id, original),
        Some(std::borrow::Cow::Borrowed(_))
    ));
    let resolver = PendingSources::new(&engine, &repo.main_branch_id, commit);
    assert_eq!(
        resolver.checked(original),
        Some(original.version_id.as_str())
    );
    let mut external = original.clone();
    external.version_id = "not-the-first-visible-version".into();
    assert_eq!(
        resolver.checked(&external),
        Some(original.version_id.as_str())
    );
    assert_eq!(
        resolver.checked(&external),
        legacy_pending_entry_source(&engine, &repo.main_branch_id, commit, &external)
    );
    let mut duplicate_snapshot = commit.clone();
    duplicate_snapshot.entries.push(external.clone());
    let duplicate = PendingSources::new(&engine, &repo.main_branch_id, &duplicate_snapshot);
    assert_eq!(
        duplicate.pending_versions().get(&original.file_id),
        Some(&original.version_id)
    );
    let expected: BTreeMap<_, _> = commit
        .entries
        .iter()
        .filter_map(|entry| {
            legacy_pending_entry_source(&engine, &repo.main_branch_id, commit, entry)
                .map(|source| (entry.file_id.clone(), source.to_owned()))
        })
        .collect();
    assert_eq!(resolver.pending_versions(), expected);
    save_projection_file(
        &mut engine,
        &repo.main_branch_id,
        "folder/data.txt",
        b"materialized",
    );
    let historical = &engine.state().commits[&head];
    let expected: BTreeMap<_, _> = historical
        .entries
        .iter()
        .filter_map(|entry| {
            legacy_pending_entry_source(&engine, &repo.main_branch_id, historical, entry)
                .map(|source| (entry.file_id.clone(), source.to_owned()))
        })
        .collect();
    let projection = Tree::capture(
        &engine,
        &identity,
        &json!({
            "branch_id": repo.main_branch_id, "query": {"commit_id": head}
        }),
    )
    .unwrap();
    assert_eq!(projection.pending_file_versions, expected);
}

fn directory_entry(index: usize, path: String) -> storage_contract::Entry {
    storage_contract::Entry {
        entry_id: format!("entry_{index:032x}"),
        name: path.rsplit('/').next().unwrap().into(),
        lookup_key: name_policy::path_key(&path),
        path,
        kind: "file".into(),
        file_id: format!("file_{index:032x}"),
        version_id: format!("version_{index:032x}"),
        size: 0,
    }
}
#[test]
fn large_directory_is_paginated_by_actual_bytes_and_count_without_full_tree_copies() {
    use smb_protocol::protocol::{
        MAX_DIRECTORY_PAGE_BYTES, MAX_DIRECTORY_PAGE_ENTRIES, MAX_FRAME_BYTES, Response,
    };
    let mut entries: Vec<_> = (0..12_000)
        .map(|index| directory_entry(index, format!("f{index:05}-{}", "x".repeat(180))))
        .collect();
    for entry in &entries {
        name_policy::validate_path(&entry.path).unwrap();
    }
    entries.sort_by(|left, right| left.lookup_key.cmp(&right.lookup_key));
    assert!(
        json_codec::to_vec(&json!({"entries":entries}))
            .unwrap()
            .len()
            > MAX_FRAME_BYTES
    );
    let mut after = None;
    let mut seen = Vec::new();
    loop {
        let page = SmbDirectory::page(
            "commit_00000000000000000000000000000000",
            &entries,
            "",
            after.as_deref(),
        )
        .unwrap();
        assert!(page.entries.len() <= MAX_DIRECTORY_PAGE_ENTRIES);
        assert!(
            json_codec::to_vec(&Response::Ok {
                result: json_codec::to_value(&page).unwrap()
            })
            .unwrap()
            .len()
                <= MAX_DIRECTORY_PAGE_BYTES
        );
        assert!(!page.more || !page.entries.is_empty());
        after = page.entries.last().map(|entry| entry.lookup_key.clone());
        seen.extend(page.entries.iter().map(|entry| entry.entry_id.clone()));
        if !page.more {
            break;
        }
    }
    assert_eq!(
        seen,
        entries
            .iter()
            .map(|entry| entry.entry_id.clone())
            .collect::<Vec<_>>()
    );
}
#[test]
fn directory_byte_bound_applies_to_utf8_and_long_paths_before_the_count_limit() {
    use smb_protocol::protocol::{MAX_DIRECTORY_PAGE_BYTES, MAX_DIRECTORY_PAGE_ENTRIES, Response};
    // Valid paths have up to 200 UTF-16 units, so non-ASCII names can consume
    // considerably more bytes than the entry-count budget alone predicts.
    let parent = vec!["界".repeat(50); 3].join("/");
    let entries: Vec<_> = (0..300)
        .map(|index| directory_entry(index, format!("{parent}/f{index:05}")))
        .collect();
    for entry in &entries {
        name_policy::validate_path(&entry.path).unwrap();
    }
    let page = SmbDirectory::page(
        "commit_00000000000000000000000000000000",
        &entries,
        &parent,
        None,
    )
    .unwrap();
    assert!(page.more);
    assert!(page.entries.len() < MAX_DIRECTORY_PAGE_ENTRIES);
    assert!(
        json_codec::to_vec(&Response::Ok {
            result: json_codec::to_value(page).unwrap()
        })
        .unwrap()
        .len()
            <= MAX_DIRECTORY_PAGE_BYTES
    );
}
