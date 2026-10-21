//! Disk/folder navigation and generation-guarded metadata reads.
use super::disk_path;
use crate::{
    api,
    ui::{State, current_session, fail, list, text},
};
use json_codec::Value;
use web_interface::prelude::*;
use web_interface::tasks::spawn_local;

pub(crate) fn load_disks(state: RwSignal<State>) {
    if state.with_untracked(|s| s.files.loading_disks) {
        return;
    }
    state.update(|s| s.files.loading_disks = true);
    let epoch = state.with_untracked(|s| s.session_epoch);
    spawn_local(async move {
        let result = api::request("GET", "/api/file-manager/disks", None).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => state.update(|s| {
                s.files.disks = list(&value, "disks");
                s.files.disks_loaded = true;
            }),
            Err(error) => fail(state, error),
        }
        state.update(|s| s.files.loading_disks = false);
    });
}
pub(crate) fn open_disk(state: RwSignal<State>, slot: String) {
    state.update(|s| {
        s.page = "files".into();
        s.files.slot = slot;
        s.files.writable = false;
        s.files.directory.clear();
        s.files.query.clear();
        s.files.directory_branches.clear();
        s.files.directory_file_count = 0;
        s.files.directory_branches_supported = false;
        s.files.repository_id.clear();
        s.files.repository_path.clear();
        s.files.default_branch_scope = false;
        s.files.clear_selection();
        s.files.checked_entries.clear();
        s.files.batch = None;
        s.files.entries.clear();
        s.files.loaded = false;
    });
    load_entries(state);
}
pub(crate) fn go_root(state: RwSignal<State>) {
    state.update(|s| {
        s.page = "files".into();
        s.files.directory_branches.clear();
        s.files.directory_file_count = 0;
        s.files.directory_branches_supported = false;
        s.files.repository_id.clear();
        s.files.repository_path.clear();
        s.files.default_branch_scope = false;
        s.files.slot.clear();
        s.files.writable = false;
        s.files.loading = false;
        s.files.directory.clear();
        s.files.entries.clear();
        s.files.clear_selection();
        s.files.checked_entries.clear();
        s.files.batch = None;
        s.files.generation += 1;
    });
    load_disks(state);
}
pub(crate) fn go_directory(state: RwSignal<State>, directory: String) {
    state.update(|s| {
        s.page = "files".into();
        s.files.directory = directory;
        s.files.writable = false;
        s.files.entries.clear();
        s.files.query.clear();
        s.files.directory_branches.clear();
        s.files.directory_file_count = 0;
        s.files.directory_branches_supported = false;
        s.files.repository_id.clear();
        s.files.repository_path.clear();
        s.files.default_branch_scope = false;
        s.files.clear_selection();
        s.files.checked_entries.clear();
        s.files.batch = None;
        s.files.loaded = false;
    });
    load_entries(state);
}
pub(crate) fn load_entries(state: RwSignal<State>) {
    let epoch = state.with_untracked(|s| s.session_epoch);
    let (slot, directory, generation) = state.with_untracked(|s| {
        (
            s.files.slot.clone(),
            s.files.directory.clone(),
            s.files.generation + 1,
        )
    });
    if slot.is_empty() {
        return;
    }
    state.update(|s| {
        s.files.generation = generation;
        s.files.loading = true;
    });
    spawn_local(async move {
        let result = api::request(
            "GET",
            &format!(
                "{}/entries?path={}",
                disk_path(&slot),
                api::segment(&directory)
            ),
            None,
        )
        .await;
        if !current_listing(state, epoch, &slot, &directory, generation) {
            return;
        }
        let refreshed = result.is_ok();
        match result {
            Ok(value) => state.update(|s| {
                s.files.entries = list(&value, "entries");
                s.files.directory_branches = list(&value, "directory_branches");
                s.files.repository_id = text(&value["context"], &["repository_id"]);
                s.files.repository_path = text(&value["context"], &["repository_path"]);
                s.files.default_branch_scope =
                    value["default_branch_scope"].as_bool().unwrap_or(false);
                s.files.directory_branches_supported = value["directory_branches_supported"]
                    .as_bool()
                    .unwrap_or(false);
                s.files.directory_file_count =
                    crate::format::parse_u64(&value["directory_file_count"]).unwrap_or(0);
                s.files.writable = value["writable"].as_bool().unwrap_or(false);
                s.files.loaded = true;
                s.files.reconcile_selection();
                if s.page == "details" && s.files.selected.is_none() {
                    s.page = "files".into();
                }
            }),
            Err(error) => fail(state, error),
        }
        state.update(|s| s.files.loading = false);
        if refreshed && state.with_untracked(|s| s.files.selected.is_some()) {
            load_file_history(state);
        }
    });
}
fn current_listing(
    state: RwSignal<State>,
    epoch: u64,
    slot: &str,
    directory: &str,
    generation: u64,
) -> bool {
    current_session(state, epoch)
        && state.with_untracked(|s| {
            s.files.slot == slot
                && s.files.directory == directory
                && s.files.generation == generation
        })
}
pub(crate) fn select_entry(state: RwSignal<State>, entry: Value) {
    state.update(|s| {
        s.files.clear_selection();
        s.files.selected = Some(entry.clone());
        s.page = "details".into();
    });
    if entry["kind"] == "file" {
        load_file_history(state);
    }
}
pub(crate) fn load_file_history(state: RwSignal<State>) {
    let epoch = state.with_untracked(|s| s.session_epoch);
    let (id, context) = state.with_untracked(|s| {
        s.files
            .selected
            .as_ref()
            .map(|entry| {
                (
                    text(entry, &["file_id"]),
                    crate::files::model::file_context(&s.files, entry),
                )
            })
            .unwrap_or_default()
    });
    if id.is_empty() {
        return;
    }
    let generation = state.with_untracked(|s| s.files.history_generation + 1);
    state.update(|s| {
        s.files.history_loading = true;
        s.files.history_generation = generation;
    });
    spawn_local(async move {
        let result = api::request(
            "GET",
            &format!(
                "/api/file-manager/files/{}/history{}",
                api::segment(&id),
                context
                    .as_ref()
                    .map(|(repository, path)| format!(
                        "?repository_id={}&path={}",
                        api::segment(repository),
                        api::segment(path)
                    ))
                    .unwrap_or_default()
            ),
            None,
        )
        .await;
        if !current_session(state, epoch)
            || !state.with_untracked(|s| {
                s.files.history_generation == generation
                    && s.files
                        .selected
                        .as_ref()
                        .is_some_and(|entry| text(entry, &["file_id"]) == id)
            })
        {
            return;
        }
        match result {
            Ok(value) => state.update(|s| {
                s.files.file_history = s
                    .files
                    .selected
                    .as_ref()
                    .map(|entry| crate::files::model::history_for_entry(value.clone(), entry))
                    .unwrap_or(value);
            }),
            Err(error) => {
                // A failed refresh must not re-enable controls for stale heads.
                state.update(|s| s.files.file_history = Value::Null);
                fail(state, error);
            }
        }
        state.update(|s| s.files.history_loading = false);
    });
}

#[cfg(test)]
#[path = "../../../../../test/web/unit/files/actions/browse/tests.rs"]
mod tests;
