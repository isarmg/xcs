//! Independent file branches: metadata first, complete content on first save.
use super::*;
use serde_json::json;

pub(crate) fn selected_file_branch<'a>(
    state: &'a State,
    branch_id: &str,
    file_id: &str,
) -> Option<&'a Branch> {
    let branch = state.branches.get(branch_id)?;
    if state
        .repositories
        .values()
        .any(|repo| repo.main_branch_id == branch_id)
        && let Some(selected) = state.file_selections.get(file_id)
    {
        return state.branches.get(selected);
    }
    Some(branch)
}

pub(crate) fn pending_file_source<'a>(
    state: &'a State,
    branch_id: &str,
    file_id: &str,
) -> Option<&'a str> {
    selected_file_branch(state, branch_id, file_id)?
        .pending_file_versions
        .get(file_id)
        .map(String::as_str)
}

fn file_snapshot(entries: &[Entry], file: &Entry) -> Vec<Entry> {
    entries
        .iter()
        .filter(|entry| {
            (entry.kind == "file" && entry.lookup_key == file.lookup_key)
                || (entry.kind == "directory"
                    && super::namespace::is_descendant(&file.lookup_key, &entry.lookup_key))
        })
        .cloned()
        .collect()
}

pub(crate) fn synchronize_directory_edits(
    state: &mut State,
    commit_id: &str,
    written_paths: &[String],
    principal: &str,
    operation: &str,
) -> Result<()> {
    let current = state
        .commits
        .get(commit_id)
        .cloned()
        .ok_or_else(|| Error::new("STATE_INVALID", "directory commit missing"))?;
    for file in current
        .entries
        .iter()
        .filter(|e| e.kind == "file" && written_paths.contains(&e.path))
    {
        let Some(repo) = state
            .repositories
            .values()
            .find(|r| r.history_tree_id == current.history_tree_id)
        else {
            continue;
        };
        let Some(context) = directory::context_at(state, &repo.repository_id, &file.path) else {
            continue;
        };
        let branch_id = context.branch_id;
        if !state
            .branches
            .get(&branch_id)
            .is_some_and(|b| b.file_id.as_deref() == Some(&file.file_id))
        {
            continue;
        }

        let branch = state
            .branches
            .get(&branch_id)
            .cloned()
            .ok_or_else(|| Error::new("STATE_INVALID", "selected file branch missing"))?;
        if branch.history_tree_id != current.history_tree_id {
            continue;
        }
        let previous = state
            .commits
            .get(&branch.head_commit_id)
            .ok_or_else(|| Error::new("STATE_INVALID", "selected file head missing"))?;
        if previous
            .entries
            .iter()
            .any(|e| e.file_id == file.file_id && e.version_id == file.version_id)
        {
            continue;
        }
        let id = new_id("commit");
        state.commits.insert(
            id.clone(),
            Commit {
                commit_id: id.clone(),
                history_tree_id: current.history_tree_id.clone(),
                kind: "file_content".into(),
                parent_commit_ids: vec![branch.head_commit_id],
                tree_id: new_id("tree"),
                sequence: state.sequence,
                operation_id: operation.into(),
                author: principal.into(),
                message: current.message.clone(),
                entries: file_snapshot(&current.entries, file),
                pending_file_versions: Default::default(),
                provenance: None,
            },
        );
        let target = state.branches.get_mut(&branch_id).expect("checked");
        target.head_commit_id = id;
        target.pending_file_versions.remove(&file.file_id);
        target.file_node_id = None;
        target.generation += 1;
    }
    Ok(())
}

/// Publish one selected file into its directory. No sibling entry or payload is
/// copied, replaced or switched by this operation.
pub(crate) fn publish_selected_file(
    state: &mut State,
    branch_id: &str,
    principal: &str,
    operation: &str,
) -> Result<String> {
    publish_selected_file_at(state, branch_id, None, principal, operation)
}
pub(crate) fn publish_selected_file_at(
    state: &mut State,
    branch_id: &str,
    path: Option<&str>,
    principal: &str,
    operation: &str,
) -> Result<String> {
    let branch = state
        .branches
        .get(branch_id)
        .cloned()
        .ok_or_else(|| Error::new("BRANCH_NOT_FOUND", "file branch missing"))?;
    let file_id = branch
        .file_id
        .as_deref()
        .ok_or_else(|| Error::new("FILE_BRANCH_REQUIRED", "select an independent file branch"))?;
    let source = state
        .commits
        .get(&branch.head_commit_id)
        .and_then(|c| c.entries.iter().find(|e| e.file_id == file_id))
        .cloned()
        .ok_or_else(|| Error::new("STATE_INVALID", "file branch content missing"))?;
    let repository = state
        .repositories
        .values()
        .find(|r| r.history_tree_id == branch.history_tree_id)
        .cloned()
        .ok_or_else(|| Error::new("REPOSITORY_NOT_FOUND", "file directory missing"))?;
    let main = state
        .branches
        .get(&repository.main_branch_id)
        .cloned()
        .ok_or_else(|| Error::new("BRANCH_NOT_FOUND", "directory branch missing"))?;
    if main.state != "active" {
        return Err(Error::new("BRANCH_NOT_ACTIVE", "directory is not writable"));
    }
    let current = state
        .commits
        .get(&main.head_commit_id)
        .cloned()
        .ok_or_else(|| Error::new("STATE_INVALID", "directory head missing"))?;
    let selected_path = path
        .map(str::to_owned)
        .or_else(|| {
            state
                .file_locations
                .get(file_id)
                .filter(|p| {
                    current
                        .entries
                        .iter()
                        .any(|e| e.file_id == file_id && e.lookup_key == zxs_naming::path_key(p))
                })
                .cloned()
        })
        .or_else(|| {
            current
                .entries
                .iter()
                .find(|e| e.file_id == file_id && e.kind == "file")
                .map(|e| e.path.clone())
        })
        .ok_or_else(|| Error::new("FILE_NOT_FOUND", "file is absent from current composition"))?;
    let context = directory::context_at(state, &repository.repository_id, &selected_path);
    let inherited = context.as_ref().is_some_and(|c| c.inherited_from.is_some());
    state.file_write_origins.remove(file_id);
    let already_selected = context.as_ref().is_some_and(|c| c.branch_id == branch_id);
    if !inherited
        && context.as_ref().and_then(|c| c.node_id.as_ref()) == branch.file_node_id.as_ref()
        && already_selected
        && current.entries.iter().any(|e| {
            e.kind == "file"
                && e.lookup_key == zxs_naming::path_key(&selected_path)
                && e.version_id == source.version_id
        })
    {
        return Ok(current.commit_id);
    }
    let mut entries = current.entries.clone();
    let target = entries
        .iter_mut()
        .find(|e| {
            e.file_id == file_id
                && e.lookup_key == zxs_naming::path_key(&selected_path)
                && e.kind == "file"
        })
        .ok_or_else(|| Error::new("FILE_NOT_FOUND", "file was deleted from its directory"))?;
    target.version_id = source.version_id;
    target.size = source.size;
    let changed_path = target.path.clone();
    let mut pending_file_versions = current.pending_file_versions.clone();
    if let Some(source) = branch.pending_file_versions.get(file_id) {
        pending_file_versions.insert(file_id.into(), source.clone());
    } else {
        pending_file_versions.remove(file_id);
    }
    let id = new_id("commit");
    state.commits.insert(
        id.clone(),
        Commit {
            commit_id: id.clone(),
            history_tree_id: branch.history_tree_id,
            kind: "file_selection".into(),
            parent_commit_ids: vec![current.commit_id],
            tree_id: new_id("tree"),
            sequence: state.sequence,
            operation_id: operation.into(),
            author: principal.into(),
            message: format!("Select file branch {}", branch.name),
            entries,
            pending_file_versions,
            provenance: Some(branch.head_commit_id),
        },
    );
    let main = state
        .branches
        .get_mut(&repository.main_branch_id)
        .expect("main checked");
    main.head_commit_id = id.clone();
    // Selection delegates this file to its independent branch. An older
    // directory placeholder must not mask that branch's first real write.
    main.pending_file_versions.remove(file_id);
    main.generation += 1;
    state
        .file_selections
        .insert(file_id.into(), branch_id.into());
    state.file_contexts.insert(
        directory::context_key(&repository.repository_id, &changed_path),
        FileContext {
            node_id: branch.file_node_id.clone(),
            selection_generation: context
                .as_ref()
                .map(|c| c.selection_generation)
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| Error::new("RESOURCE_LIMIT", "file context generation exhausted"))?,
            branch_id: branch_id.into(),
            inherited_from: None,
        },
    );
    super::directory::record_directory_edits(
        state,
        &repository.repository_id,
        &[changed_path],
        None,
    )?;
    Ok(id)
}

pub(crate) fn ensure_shared_file_branch(
    state: &mut State,
    current: &Commit,
    original: &Entry,
) -> Result<String> {
    let file_id = original.file_id.as_str();
    if !state.file_selections.contains_key(file_id) {
        if let Some(id) = state
            .repositories
            .values()
            .find(|r| r.history_tree_id == current.history_tree_id)
            .and_then(|r| {
                state
                    .file_contexts
                    .get(&directory::context_key(&r.repository_id, &original.path))
            })
            .filter(|c| {
                state
                    .branches
                    .get(&c.branch_id)
                    .is_some_and(|b| b.file_id.as_deref() == Some(file_id))
            })
            .map(|c| c.branch_id.clone())
        {
            state.file_selections.insert(file_id.into(), id.clone());
            return Ok(id);
        }
        if let Some(id) = state
            .branches
            .values()
            .find(|b| b.file_id.as_deref() == Some(file_id) && b.name == "main")
            .map(|b| b.branch_id.clone())
        {
            state.file_selections.insert(file_id.into(), id.clone());
            return Ok(id);
        }
        let id = new_id("br");
        let commit_id = new_id("commit");
        state.commits.insert(
            commit_id.clone(),
            Commit {
                commit_id: commit_id.clone(),
                history_tree_id: current.history_tree_id.clone(),
                kind: "file_branch_root".into(),
                parent_commit_ids: vec![current.commit_id.clone()],
                tree_id: new_id("tree"),
                sequence: state.sequence,
                operation_id: current.operation_id.clone(),
                author: current.author.clone(),
                message: "Initialize file main branch".into(),
                entries: file_snapshot(&current.entries, original),
                pending_file_versions: current
                    .pending_file_versions
                    .iter()
                    .filter(|(file, _)| file.as_str() == file_id)
                    .map(|(file, source)| (file.clone(), source.clone()))
                    .collect(),
                provenance: Some(current.commit_id.clone()),
            },
        );
        state.branches.insert(
            id.clone(),
            Branch {
                file_node_id: None,
                branch_id: id.clone(),
                history_tree_id: current.history_tree_id.clone(),
                name: "main".into(),
                head_commit_id: commit_id,
                fork_commit_id: current.commit_id.clone(),
                branch_kind: "file".into(),
                file_id: Some(file_id.into()),
                pending_file_versions: current
                    .pending_file_versions
                    .get(file_id)
                    .map(|version| [(file_id.into(), version.clone())].into())
                    .unwrap_or_default(),
                generation: 1,
                state: "active".into(),
            },
        );
        state.file_selections.insert(file_id.into(), id);
    }
    state
        .file_selections
        .get(file_id)
        .cloned()
        .ok_or_else(|| Error::new("STATE_INVALID", "shared file branch missing"))
}

pub(crate) fn create_file_branch_change(
    engine: &Engine,
    state: &mut State,
    file_id: &str,
    source_version: &str,
    name: &str,
    principal: &str,
    operation: &str,
) -> Result<serde_json::Value> {
    let source = state
        .versions
        .get(source_version)
        .cloned()
        .filter(|v| v.file_id == file_id)
        .ok_or_else(|| {
            Error::new(
                "VERSION_NOT_FOUND",
                "selected version does not belong to this file",
            )
        })?;
    let repo = engine.repository_for_history(&source.history_tree_id)?;
    engine.require_grant(repo, principal, zxs_contract::RepositoryAccess::Write)?;
    let main = state
        .branches
        .get(&repo.main_branch_id)
        .cloned()
        .ok_or_else(|| Error::new("BRANCH_NOT_FOUND", "directory branch missing"))?;
    if main.state != "active" {
        return Err(Error::new("BRANCH_NOT_ACTIVE", "directory is not writable"));
    }
    let current = state
        .commits
        .get(&main.head_commit_id)
        .cloned()
        .ok_or_else(|| Error::new("STATE_INVALID", "directory head missing"))?;
    let original = current
        .entries
        .iter()
        .find(|e| e.file_id == file_id && e.kind == "file")
        .cloned()
        .ok_or_else(|| Error::new("FILE_NOT_FOUND", "file no longer exists in this directory"))?;
    if zxs_naming::lookup_key(name) == "main"
        || state.branches.values().any(|b| {
            b.file_id.as_deref() == Some(file_id)
                && zxs_naming::lookup_key(&b.name) == zxs_naming::lookup_key(name)
        })
    {
        return Err(Error::new(
            "NAME_COLLISION",
            "file branch name already exists",
        ));
    }
    ensure_shared_file_branch(state, &current, &original)?;
    let id = new_id("br");
    let commit_id = new_id("commit");
    let mut selected = original.clone();
    selected.version_id = source.version_id.clone();
    selected.size = source.logical_size;
    let entries = file_snapshot(&current.entries, &selected)
        .into_iter()
        .map(|entry| {
            if entry.file_id == file_id {
                selected.clone()
            } else {
                entry
            }
        })
        .collect();
    state.commits.insert(
        commit_id.clone(),
        Commit {
            commit_id: commit_id.clone(),
            history_tree_id: source.history_tree_id.clone(),
            kind: "file_branch_pending".into(),
            parent_commit_ids: vec![current.commit_id.clone()],
            tree_id: new_id("tree"),
            sequence: state.sequence,
            operation_id: operation.into(),
            author: principal.into(),
            message: format!("Create file branch {name}"),
            entries,
            pending_file_versions: [(file_id.into(), source_version.into())].into(),
            provenance: Some(source_version.into()),
        },
    );
    state.branches.insert(
        id.clone(),
        Branch {
            file_node_id: None,
            branch_id: id.clone(),
            history_tree_id: source.history_tree_id.clone(),
            name: name.into(),
            head_commit_id: commit_id.clone(),
            fork_commit_id: current.commit_id,
            branch_kind: "file".into(),
            file_id: Some(file_id.into()),
            pending_file_versions: [(file_id.into(), source_version.into())].into(),
            generation: 1,
            state: "active".into(),
        },
    );
    Ok(
        json!({"file_id":file_id,"branch_id":id,"commit_id":commit_id,"history_tree_id":source.history_tree_id,"copied_files":"0","placeholder_files":"0","materialized":false,"inherited":true,"own_version_id":null,"version_id":source_version,"source_version_id":source_version,"content_state":"PENDING","selected":false}),
    )
}

impl Engine {
    pub fn set_file_note(
        &mut self,
        file_id: &str,
        version_id: Option<&str>,
        note: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        validate_note(note)?;
        self.mutate(
            if version_id.is_some() {
                "version.metadata"
            } else {
                "file.metadata"
            },
            principal,
            operation,
            json!({"file_id":file_id,"version_id":version_id,"note":note}),
            |engine, state, _| {
                let version = state
                    .versions
                    .values()
                    .find(|v| {
                        v.file_id == file_id && version_id.is_none_or(|id| v.version_id == id)
                    })
                    .ok_or_else(|| Error::new("FILE_NOT_FOUND", "file or version not found"))?;
                engine.require_grant(
                    engine.repository_for_history(&version.history_tree_id)?,
                    principal,
                    zxs_contract::RepositoryAccess::Write,
                )?;
                let notes = if version_id.is_some() {
                    &mut state.version_notes
                } else {
                    &mut state.file_notes
                };
                let key = version_id.unwrap_or(file_id);
                if note.is_empty() {
                    notes.remove(key);
                } else {
                    notes.insert(key.into(), note.into());
                }
                Ok(json!({"file_id":file_id,"version_id":version_id,"note":note}))
            },
        )
    }
    pub fn file_namespace(
        &mut self,
        repository: &str,
        change: NamespaceOperation,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        zxs_naming::validate_path(&change.path)?;
        if let Some(destination) = &change.destination {
            zxs_naming::validate_path(destination)?;
        }
        self.mutate("file.namespace",principal,operation,json!({"repository_id":repository,"change":change}),|engine,state,_|{
            let repo=engine.state.repositories.get(repository).ok_or_else(||Error::new("REPOSITORY_NOT_FOUND","directory namespace missing"))?;
            engine.require_grant(repo,principal,zxs_contract::RepositoryAccess::Write)?;
            let branch=state.branches.get(&repo.main_branch_id).cloned().ok_or_else(||Error::new("BRANCH_NOT_FOUND","directory branch missing"))?;
            if branch.state!="active"{return Err(Error::new("BRANCH_NOT_ACTIVE","directory is not writable"));}
            let previous=state.commits.get(&branch.head_commit_id).cloned().ok_or_else(||Error::new("STATE_INVALID","directory head missing"))?;
            let mut entries=previous.entries.clone();super::apply_namespace(&mut entries,&change)?;
            validate_entries(&mut entries,engine.config.max_entries)?;
            let renamed_directories=directory::prepare_renamed_directories(state,repository,&previous.entries,std::slice::from_ref(&change))?;
            super::directory::namespace_locations(state,repository,&previous.entries,&entries,std::slice::from_ref(&change),principal,operation)?;
            let pending_file_versions=previous.pending_file_versions.iter().filter(|(file,_)|entries.iter().any(|entry|entry.kind=="file"&&entry.file_id==file.as_str())).map(|(file,source)|(file.clone(),source.clone())).collect();
            let id=new_id("commit");
            state.commits.insert(id.clone(),Commit{commit_id:id.clone(),history_tree_id:branch.history_tree_id.clone(),kind:"namespace".into(),parent_commit_ids:vec![previous.commit_id],tree_id:new_id("tree"),sequence:state.sequence,operation_id:operation.into(),author:principal.into(),message:format!("{} {}",change.kind,change.path),entries,pending_file_versions,provenance:None});
            let live_files=state.commits[&id].entries.iter().filter(|entry|entry.kind=="file").map(|entry|entry.file_id.clone()).collect::<std::collections::BTreeSet<_>>();
            let branch=state.branches.get_mut(&repo.main_branch_id).expect("checked");branch.head_commit_id=id.clone();branch.generation+=1;
            branch.pending_file_versions.retain(|file,_|live_files.contains(file));
            let mut changed_paths=vec![change.path.clone()];
            changed_paths.extend(renamed_directories);
            if let Some(destination)=&change.destination {changed_paths.push(destination.clone());}
            let directories=super::directory::record_directory_edits(state,repository,&changed_paths,None)?;
            Ok(json!({"commit_id":id,"branch_id":repo.main_branch_id,"history_tree_id":repo.history_tree_id,"path":change.path,"action":change.kind,"directory_branch_ids":directories,"file_node_ids":file_paths::operation_nodes(state,operation)}))
        })
    }
    pub fn create_file_branch(
        &mut self,
        file_id: &str,
        source_version: &str,
        name: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        super::validate_branch_name(name)?;
        self.mutate(
            "file.branch.create",
            principal,
            operation,
            json!({"file_id":file_id,"source_version_id":source_version,"name":name}),
            |engine, state, _| {
                create_file_branch_change(
                    engine,
                    state,
                    file_id,
                    source_version,
                    name,
                    principal,
                    operation,
                )
            },
        )
    }

    pub fn select_file_branch_at(
        &mut self,
        repository: &str,
        path: &str,
        file_id: &str,
        branch_id: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        zxs_naming::validate_path(path)?;
        self.mutate("file.branch.select",principal,operation,json!({"repository_id":repository,"path":path,"file_id":file_id,"branch_id":branch_id}),|engine,state,_|{
            let repo=state.repositories.get(repository).ok_or_else(||Error::new("REPOSITORY_NOT_FOUND","repository missing"))?;engine.require_grant(repo,principal,zxs_contract::RepositoryAccess::Write)?;
            let current=&state.commits[&state.branches[&repo.main_branch_id].head_commit_id];
            if !find(&current.entries,path).is_some_and(|e|e.kind=="file"&&e.file_id==file_id){return Err(Error::new("FILE_NOT_FOUND","file/path identity differs"));}
            let branch=state.branches.get(branch_id).cloned().filter(|b|b.file_id.as_deref()==Some(file_id)&&b.history_tree_id==repo.history_tree_id&&b.state=="active").ok_or_else(||Error::new("FILE_BRANCH_NOT_FOUND","file branch missing"))?;
            let selected=state.commits[&branch.head_commit_id].entries.iter().find(|e|e.file_id==file_id).ok_or_else(||Error::new("STATE_INVALID","file branch entry missing"))?;
            // A path-scoped selection has the same content safety requirement as
            // the unscoped operation. Pending branches retain frozen references.
            if !branch.pending_file_versions.contains_key(file_id) { engine.valid_replica(&selected.version_id)?; }
            let commit=publish_selected_file_at(state,branch_id,Some(path),principal,operation)?;
            Ok(json!({"file_id":file_id,"branch_id":branch_id,"commit_id":commit,"history_tree_id":branch.history_tree_id,"path":path}))
        })
    }
    pub fn select_file_branch(
        &mut self,
        file_id: &str,
        branch_id: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        self.mutate("file.branch.select",principal,operation,json!({"file_id":file_id,"branch_id":branch_id}),|engine,state,_|{
            let branch=state.branches.get(branch_id).cloned().filter(|b|b.file_id.as_deref()==Some(file_id)&&b.state=="active")
                .ok_or_else(||Error::new("FILE_BRANCH_NOT_FOUND","select an active branch of this file"))?;
            engine.require_grant(engine.repository_for_history(&branch.history_tree_id)?,principal,zxs_contract::RepositoryAccess::Write)?;
            let selected=state.commits[&branch.head_commit_id].entries.iter().find(|e|e.file_id==file_id).ok_or_else(||Error::new("STATE_INVALID","file branch entry missing"))?;
            if !branch.pending_file_versions.contains_key(file_id) { engine.valid_replica(&selected.version_id)?; }
            let commit=publish_selected_file(state,branch_id,principal,operation)?;
            Ok(json!({"file_id":file_id,"branch_id":branch_id,"commit_id":commit,"history_tree_id":branch.history_tree_id}))
        })
    }
}

#[cfg(test)]
mod pending_regressions {
    use super::*;
    use std::io::Read;

    fn fixture() -> (tempfile::TempDir, Engine, Repository) {
        let root = tempfile::tempdir().unwrap();
        let mut engine = Engine::initialize(
            &root.path().join("system"),
            Config::development([
                root.path().join("p"),
                root.path().join("b"),
                root.path().join("c"),
            ]),
        )
        .unwrap();
        let repo = engine
            .create_repository("branch regression", "P01", 3, "admin", &new_id("op"))
            .unwrap();
        (root, engine, repo)
    }
    fn save(engine: &mut Engine, branch: &str, bytes: &[u8]) -> OperationResult {
        let workspace = engine.create_workspace(branch, "admin").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "report.txt", bytes)
            .unwrap();
        commit(engine, workspace)
    }
    fn commit(engine: &mut Engine, workspace: Workspace) -> OperationResult {
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "admin".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "Regression fixture".into(),
                request_digest: None,
            })
            .unwrap()
    }
    fn file(engine: &Engine, branch: &str) -> Entry {
        engine
            .list_tree(branch)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap()
    }

    #[test]
    fn inherited_range_fences_selection_and_preserves_parent_bytes() {
        let (_root, mut engine, repo) = fixture();
        save(&mut engine, &repo.main_branch_id, b"SOURCE_BYTES");
        let original = file(&engine, &repo.main_branch_id);
        let pending = engine
            .create_file_branch(
                &original.file_id,
                &original.version_id,
                "pending",
                "admin",
                &new_id("op"),
            )
            .unwrap();
        let pending = pending.result["branch_id"].as_str().unwrap().to_owned();
        let materialized_main = engine.state().file_selections[&original.file_id].clone();
        engine
            .select_file_branch(&original.file_id, &pending, "admin", &new_id("op"))
            .unwrap();
        let old_workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        engine
            .select_file_branch(
                &original.file_id,
                &materialized_main,
                "admin",
                &new_id("op"),
            )
            .unwrap();
        let error = engine
            .stage_range(
                &old_workspace.workspace_id,
                &new_id("entry"),
                "report.txt",
                0,
                b"X",
                None,
            )
            .unwrap_err();
        assert_eq!(error.code(), "FILE_BRANCH_CONFLICT");
        assert!(
            engine
                .workspace(&old_workspace.workspace_id)
                .unwrap()
                .files
                .is_empty()
        );
        assert_eq!(engine.file_history(&original.file_id).unwrap().len(), 1);

        engine
            .select_file_branch(&original.file_id, &pending, "admin", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        let staged = engine
            .stage_range(
                &workspace.workspace_id,
                &new_id("entry"),
                "report.txt",
                0,
                b"X",
                None,
            )
            .unwrap();
        let mut content = engine
            .open_workspace_content(&workspace.workspace_id, &staged.item_id, "admin")
            .unwrap();
        let mut bytes = Vec::new();
        content.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(
            bytes, b"XOURCE_BYTES",
            "an inherited range edits the visible parent without changing that immutable version"
        );
        drop(content);
        commit(&mut engine, workspace);
        let written = file(&engine, &repo.main_branch_id);
        assert_eq!(
            engine.state().versions[&written.version_id].source_version_ids,
            vec![original.version_id]
        );
        assert_eq!(written.size, 12);
    }

    #[test]
    fn selected_file_materialization_replaces_a_split_main_placeholder() {
        let (root, mut engine, repo) = fixture();
        let original = save(&mut engine, &repo.main_branch_id, b"original");
        let pending = engine
            .create_branch(
                original.commit_id.as_ref().unwrap(),
                "pending full",
                "admin",
                &new_id("op"),
            )
            .unwrap();
        let plan = engine
            .create_split_plan(&repo.history_tree_id, &pending.head_commit_id, "admin")
            .unwrap();
        let split = engine
            .commit_plan(&plan.plan_id, "admin", &new_id("op"))
            .unwrap();
        let repository = engine
            .state()
            .repositories
            .values()
            .find(|repo| Some(&repo.history_tree_id) == split.history_tree_id.as_ref())
            .unwrap()
            .clone();
        let placeholder = file(&engine, &repository.main_branch_id);
        assert!(!placeholder.version_id.is_empty());
        assert_eq!(placeholder.size, 8);
        let source = engine
            .pending_file_source(&repository.main_branch_id, &placeholder.file_id)
            .unwrap()
            .to_owned();
        let created = engine
            .create_file_branch(
                &placeholder.file_id,
                &source,
                "actual",
                "admin",
                &new_id("op"),
            )
            .unwrap();
        let branch = created.result["branch_id"].as_str().unwrap().to_owned();
        engine
            .select_file_branch(&placeholder.file_id, &branch, "admin", &new_id("op"))
            .unwrap();
        save(&mut engine, &branch, b"written branch bytes");
        let written = file(&engine, &branch);
        let selected = file(&engine, &repository.main_branch_id);
        assert_eq!(selected.version_id, written.version_id);
        assert_eq!(selected.size, 20);
        assert!(
            engine
                .pending_file_source(&repository.main_branch_id, &placeholder.file_id)
                .is_none()
        );
        assert!(
            !engine.state().branches[&repository.main_branch_id]
                .pending_file_versions
                .contains_key(&placeholder.file_id)
        );
        drop(engine);
        let engine = Engine::open(&root.path().join("system")).unwrap();
        assert_eq!(
            file(&engine, &repository.main_branch_id).version_id,
            written.version_id
        );
        assert!(
            engine
                .pending_file_source(&repository.main_branch_id, &placeholder.file_id)
                .is_none()
        );
    }

    #[test]
    fn historical_range_edits_keep_the_explicit_old_snapshot_as_their_base() {
        let (_root, mut engine, repo) = fixture();
        let original = save(&mut engine, &repo.main_branch_id, b"old body");
        let source = original.commit_id.unwrap();
        save(&mut engine, &repo.main_branch_id, b"current body");
        let workspace = engine
            .create_historical_workspace(&repo.main_branch_id, &source, "admin")
            .unwrap();
        engine
            .stage_range(
                &workspace.workspace_id,
                &new_id("entry"),
                "report.txt",
                0,
                b"X",
                None,
            )
            .unwrap();
        let plan = engine
            .create_historical_save_plan(
                &repo.history_tree_id,
                HistoricalSaveOptions {
                    source_commit_id: source,
                    name: "edited old snapshot".into(),
                    workspace_id: Some(workspace.workspace_id),
                    primary_slot_id: None,
                    target_copies: None,
                },
                "admin",
            )
            .unwrap();
        let saved = engine
            .commit_historical_save_plan(&plan.plan_id, "admin", &new_id("op"))
            .unwrap();
        let written = file(&engine, saved.branch_id.as_ref().unwrap());
        let mut content = engine.open_content(&written.version_id).unwrap();
        let mut bytes = Vec::new();
        content.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"Xld body");
        let unchanged = file(&engine, &repo.main_branch_id);
        assert_ne!(written.version_id, unchanged.version_id);
        assert_eq!(unchanged.size, 12);
    }
}
