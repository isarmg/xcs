//! Disk/folder navigation and generation-guarded metadata reads.
use super::disk_path;
use crate::{
    api,
    files::model::Disk,
    ui::{State, accept_read_response, current_session, fail, list, text},
};
use json_codec::Value;
use web_interface::prelude::*;
use web_interface::tasks::spawn_local;

fn begin_disk_load(state: &mut State) -> Option<(u64, u64)> {
    if !state.session_active() || state.files.loading_disks {
        return None;
    }
    state.files.loading_disks = true;
    Some((state.session_epoch, state.files.disks_generation))
}

/// Return true only when recovery invalidated this read and storage is readable.
/// Old-session completions cannot write data, release a newer read, or reload.
fn complete_disk_load(
    state: &mut State,
    epoch: u64,
    generation: u64,
    result: Result<Vec<Disk>, api::Error>,
) -> Result<bool, api::Error> {
    if !state.session_active() || state.session_epoch != epoch {
        return Ok(false);
    }
    state.files.loading_disks = false;
    // A current-session 401 revokes authentication, not just this disk snapshot.
    if state.files.disks_generation != generation
        && !matches!(&result, Err(error) if error.status == 401)
    {
        state.files.disks_loaded = false;
        // Storage may have become unavailable again while this read was pending.
        return Ok(state.management.storage.readable());
    }
    match result {
        Ok(disks) => {
            state.files.disks = disks;
            state.files.disks_loaded = true;
            Ok(false)
        }
        Err(error) => {
            // Keep cached rows, but retain the demand for a fresh snapshot.
            // Healthy management polling will retry this current-generation read.
            state.files.disks_loaded = false;
            Err(error)
        }
    }
}

pub(crate) fn load_disks(state: RwSignal<State>) {
    let Some((epoch, generation)) = state.try_update(begin_disk_load).flatten() else {
        return;
    };
    spawn_local(async move {
        let result = api::request("GET", "/api/file-manager/disks", None)
            .await
            .and_then(|value| super::super::model::parse_disks(&value));
        match state.try_update(|s| complete_disk_load(s, epoch, generation, result)) {
            Some(Ok(true)) => load_disks(state),
            Some(Err(error)) => fail(state, error),
            _ => {}
        }
    });
}

pub(crate) fn open_disk(state: RwSignal<State>, slot: String) {
    state.update(|s| {
        s.page = crate::ui::Page::Files;
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
        s.files.entries = Default::default();
        s.files.loaded = false;
    });
    load_entries(state);
}
pub(crate) fn go_root(state: RwSignal<State>) {
    state.update(|s| {
        s.page = crate::ui::Page::Files;
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
        s.files.entries = Default::default();
        s.files.clear_selection();
        s.files.checked_entries.clear();
        s.files.batch = None;
        s.files.generation += 1;
    });
    load_disks(state);
}
pub(crate) fn go_directory(state: RwSignal<State>, directory: String) {
    state.update(|s| {
        s.page = crate::ui::Page::Files;
        s.files.directory = directory;
        s.files.writable = false;
        s.files.entries = Default::default();
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
    if !state.with_untracked(State::session_active) {
        return;
    }
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
        if !accept_read_response(state, epoch, &result)
            || !current_listing(state, epoch, &slot, &directory, generation)
        {
            return;
        }
        let refreshed = result.is_ok();
        match result {
            Ok(value) => state.update(|s| {
                s.files.entries = list(&value, "entries").into();
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
                if s.page == crate::ui::Page::Details && s.files.selected.is_none() {
                    s.page = crate::ui::Page::Files;
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
        s.page = crate::ui::Page::Details;
    });
    if entry["kind"] == "file" {
        load_file_history(state);
    }
}
pub(crate) fn load_file_history(state: RwSignal<State>) {
    if !state.with_untracked(State::session_active) {
        return;
    }
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
        if !accept_read_response(state, epoch, &result)
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
                    .unwrap_or(value)
                    .into();
            }),
            Err(error) => {
                // A failed refresh must not re-enable controls for stale heads.
                state.update(|s| s.files.file_history = Value::Null.into());
                fail(state, error);
            }
        }
        state.update(|s| s.files.history_loading = false);
    });
}

#[cfg(test)]
#[path = "../../../../../test/web/unit/files/actions/browse/tests.rs"]
mod tests;
