//! Captured scope reads and publication with session/generation fences.
use super::*;
pub(crate) fn start(state: RwSignal<State>, kind: Kind, entry: Option<Value>) {
    let snapshot = state.get_untracked();
    if !file_actions::write_allowed(&snapshot) {
        return;
    }
    let entries = if let Some(entry) = entry {
        vec![entry]
    } else {
        let checked = snapshot
            .files
            .entries
            .iter()
            .filter(|entry| snapshot.files.checked_entries.contains(&entry_key(entry)))
            .cloned()
            .collect::<Vec<_>>();
        if checked.is_empty() {
            snapshot.files.selected.iter().cloned().collect()
        } else {
            checked
        }
    };
    if entries.is_empty() || snapshot.files.repository_id.is_empty() {
        return;
    }
    let sources = source_branches(&snapshot.files);
    let selected_sources = sources
        .iter()
        .filter(|branch| branch["current"] == true)
        .map(branch_id)
        .collect::<BTreeSet<_>>();
    let id = api::operation_id();
    if id.is_empty() {
        return;
    }
    let repository = snapshot.files.repository_id.clone();
    let repository_path = snapshot.files.repository_path.clone();
    let disk_prefix = if repository_path.is_empty() {
        snapshot.files.directory.clone()
    } else {
        snapshot
            .files
            .directory
            .strip_suffix(&repository_path)
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_owned()
    };
    let draft = Draft {
        id: id.clone(),
        kind,
        repository: repository.clone(),
        repository_path,
        disk_prefix,
        names: entries
            .iter()
            .map(|entry| (entry_key(entry), text(entry, &["name"])))
            .collect(),
        entries,
        mappings: selected_sources
            .iter()
            .map(|source| Mapping {
                source: source.clone(),
                target: String::new(),
            })
            .collect(),
        sources,
        selected_sources,
        catalogue: vec![],
        target_path: String::new(),
        targets: vec![],
        target_resolved: false,
        loading: false,
        submitted_operation: None,
        generation: 0,
        error: String::new(),
    };
    let epoch = snapshot.session_epoch;
    if !open_draft(state, draft) {
        return;
    }
    spawn_local(async move {
        let result = api::request(
            "GET",
            &format!(
                "/api/repositories/{}/directory-branches",
                api::segment(&repository)
            ),
            None,
        )
        .await;
        if !accept_read_response(state, epoch, &result) || !same_draft(state, &id) {
            return;
        }
        match result {
            Ok(value) => state.update(|s| {
                let draft = s.files.batch.as_mut().unwrap();
                draft.catalogue = list(&value, "directory_branches");
                if draft.sources.is_empty() {
                    draft.sources = draft
                        .catalogue
                        .iter()
                        .filter(|branch| branch["path"] == draft.repository_path)
                        .cloned()
                        .collect();
                    draft.selected_sources = draft
                        .sources
                        .iter()
                        .filter(|branch| branch["current"] == true)
                        .map(branch_id)
                        .collect();
                    draft.mappings = draft
                        .selected_sources
                        .iter()
                        .map(|source| Mapping {
                            source: source.clone(),
                            target: String::new(),
                        })
                        .collect();
                }
                draft.loading = false;
            }),
            Err(error) => {
                state.update(|s| {
                    if let Some(draft) = s.files.batch.as_mut() {
                        draft.loading = false;
                        draft.error = error.to_string();
                    }
                });
                fail(state, error);
            }
        }
    });
}
pub(super) fn open_draft(state: RwSignal<State>, mut draft: Draft) -> bool {
    // Folder listings already contain complete, frozen source records.
    // Rename/delete need no destination catalogue or extra read to open.
    draft.loading = draft.kind.has_destination() || draft.sources.is_empty();
    let load_catalogue = draft.loading;
    state.update(|s| {
        s.files.batch = Some(draft);
        s.page = crate::ui::Page::Files;
    });
    load_catalogue
}
fn same_draft(state: RwSignal<State>, id: &str) -> bool {
    state.with_untracked(|s| s.files.batch.as_ref().is_some_and(|draft| draft.id == id))
}
pub(super) fn update(state: RwSignal<State>, f: impl FnOnce(&mut Draft)) {
    state.update(|s| {
        if let Some(draft) = s.files.batch.as_mut() {
            f(draft)
        }
    });
}
pub(super) fn toggle_source(state: RwSignal<State>, id: String, checked: bool) {
    update(state, |draft| {
        if checked {
            draft.selected_sources.insert(id.clone());
            if draft.kind.has_destination() {
                draft.mappings.push(Mapping {
                    source: id,
                    target: String::new(),
                });
            }
        } else {
            draft.selected_sources.remove(&id);
            draft.mappings.retain(|mapping| mapping.source != id);
        }
        draft.error.clear();
    });
}
pub(super) fn load_target(state: RwSignal<State>) {
    if !state.with_untracked(State::session_active) {
        return;
    }
    let snapshot = state.get_untracked();
    let Some(draft) = snapshot.files.batch else {
        return;
    };
    let target = draft.target_path.trim_matches('/').to_owned();
    if !valid_path(&target) {
        update(state, |draft| {
            draft.error = t("目录路径无效。", "Invalid directory path.")
        });
        return;
    }
    let matches = draft
        .catalogue
        .iter()
        .filter(|branch| branch["path"] == target)
        .cloned()
        .collect::<Vec<_>>();
    update(state, |draft| {
        draft.target_path = target.clone();
        draft.targets.clear();
        draft.target_resolved = false;
        draft.error.clear();
        draft.generation += 1;
        for mapping in &mut draft.mappings {
            mapping.target.clear();
        }
    });
    if !matches.is_empty() {
        update(state, |draft| {
            draft.targets = matches;
            draft.target_resolved = true;
            draft.loading = false;
        });
        return;
    }
    let generation = draft.generation + 1;
    let endpoint = format!(
        "/api/file-manager/disks/{}/entries?path={}",
        api::segment(&snapshot.files.slot),
        api::segment(&join_path(&draft.disk_prefix, &target))
    );
    let epoch = snapshot.session_epoch;
    let id = draft.id;
    update(state, |draft| draft.loading = true);
    spawn_local(async move {
        let result = api::request("GET", &endpoint, None).await;
        if !accept_read_response(state, epoch, &result)
            || !same_draft(state, &id)
            || !state.with_untracked(|s| {
                s.files
                    .batch
                    .as_ref()
                    .is_some_and(|draft| draft.generation == generation)
            })
        {
            return;
        }
        match result {
            Ok(value) => update(state, |draft| {
                draft.loading = false;
                if value["context"]["repository_id"] != draft.repository
                    || value["context"]["repository_path"] != target
                    || value["writable"] != true
                {
                    draft.error = t(
                        "目标必须是同一文件目录中的可写目录。",
                        "Choose a writable destination in the same file namespace.",
                    );
                } else {
                    draft.targets = list(&value, "directory_branches");
                    draft.target_resolved = true;
                    if draft.targets.is_empty() {
                        draft.error = t(
                            "目标尚无可操作的目录分支，请先登记目录分支。",
                            "Register a destination folder branch before continuing.",
                        );
                    }
                }
            }),
            Err(error) => {
                update(state, |draft| {
                    draft.loading = false;
                    draft.error = error.to_string();
                });
                fail(state, error);
            }
        }
    });
}
pub(super) fn initialize_scope(state: RwSignal<State>, destination: bool) {
    let snapshot = state.get_untracked();
    if !file_actions::write_allowed(&snapshot) {
        return;
    }
    let Some(draft) = snapshot.files.batch else {
        return;
    };
    let path = if destination {
        if !draft.target_resolved {
            return;
        }
        draft.target_path.clone()
    } else {
        draft.repository_path.clone()
    };
    let operation_id = api::operation_id();
    if operation_id.is_empty() {
        return;
    }
    let endpoint = format!(
        "/api/repositories/{}/directory-scopes",
        api::segment(&draft.repository)
    );
    let catalogue_endpoint = format!(
        "/api/repositories/{}/directory-branches",
        api::segment(&draft.repository)
    );
    let epoch = snapshot.session_epoch;
    let id = draft.id;
    state.update(|s| s.files.mutation_pending = true);
    update(state, |draft| draft.loading = true);
    spawn_local(async move {
        let result = api::request(
            "POST",
            &endpoint,
            Some(json!({"path":path,"operation_id":operation_id})),
        )
        .await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => {
                let status = file_actions::record_receipt(state, &operation_id, value);
                if matches!(
                    status,
                    crate::files::model::SaveStatus::Published
                        | crate::files::model::SaveStatus::Degraded
                ) {
                    let result = api::request("GET", &catalogue_endpoint, None).await;
                    if !current_session(state, epoch) {
                        return;
                    }
                    match result {
                        Ok(value) => {
                            if same_draft(state, &id) {
                                update(state, |draft| {
                                    draft.catalogue = list(&value, "directory_branches");
                                    let branches = draft
                                        .catalogue
                                        .iter()
                                        .filter(|branch| branch["path"] == path)
                                        .cloned()
                                        .collect::<Vec<_>>();
                                    if destination {
                                        draft.targets = branches;
                                    } else {
                                        draft.sources = branches;
                                        draft.selected_sources = draft
                                            .sources
                                            .iter()
                                            .filter(|branch| branch["current"] == true)
                                            .map(branch_id)
                                            .collect();
                                        draft.mappings = draft
                                            .selected_sources
                                            .iter()
                                            .map(|source| Mapping {
                                                source: source.clone(),
                                                target: String::new(),
                                            })
                                            .collect();
                                    }
                                    draft.error.clear();
                                });
                            }
                        }
                        Err(error) => {
                            if same_draft(state, &id) {
                                update(state, |draft| draft.error = error.to_string());
                            }
                            fail(state, error);
                        }
                    }
                }
            }
            Err(error) => {
                if error.mutation_outcome_unknown() {
                    file_actions::record_operation(
                        state,
                        json!({"operation_id":operation_id,"state":"in_doubt"}),
                    );
                }
                if same_draft(state, &id) {
                    update(state, |draft| draft.error = error.to_string());
                }
                fail(state, error);
            }
        }
        if same_draft(state, &id) {
            update(state, |draft| draft.loading = false);
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}
pub(super) fn submit(state: RwSignal<State>) {
    let snapshot = state.get_untracked();
    if !file_actions::write_allowed(&snapshot) {
        return;
    }
    let Some(draft) = snapshot.files.batch else {
        return;
    };
    if draft.loading || draft.submitted_operation.is_some() {
        return;
    }
    let mut body = match request(&draft) {
        Ok(body) => body,
        Err(error) => {
            update(state, |draft| draft.error = error);
            return;
        }
    };
    let id = api::operation_id();
    if id.is_empty() {
        set_notice(
            state,
            t(
                "浏览器无法生成安全操作编号。",
                "The browser could not generate a secure operation ID.",
            ),
        );
        return;
    }
    body["operation_id"] = id.clone().into();
    let endpoint = format!(
        "/api/file-manager/disks/{}/namespace-batch",
        api::segment(&snapshot.files.slot)
    );
    let epoch = snapshot.session_epoch;
    let draft_id = draft.id;
    state.update(|s| {
        s.files.mutation_pending = true;
        if let Some(draft) = s.files.batch.as_mut() {
            draft.submitted_operation = Some(id.clone());
        }
    });
    spawn_local(async move {
        let result = api::request("POST", &endpoint, Some(body)).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => {
                file_actions::record_receipt(state, &id, value);
            }
            Err(error) => {
                if error.mutation_outcome_unknown() {
                    file_actions::record_operation(
                        state,
                        json!({"operation_id":id,"state":"in_doubt"}),
                    );
                }
                if same_draft(state, &draft_id) {
                    update(state, |draft| {
                        draft.error = error.to_string();
                        if !error.mutation_outcome_unknown() {
                            draft.submitted_operation = None;
                        }
                    });
                }
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}
