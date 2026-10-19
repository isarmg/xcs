//! Namespace, branch and note transactions; writes never bypass the pending-save gate.
use super::{disk_path, operation::record_operation, pending};
use crate::{
    api,
    files::model::{NamespaceAction, SaveStatus, join_path, operation_status},
    ui::{State, current_session, fail, set_notice, text},
};
use serde_json::{Value, json};
use wasm_bindgen_futures::spawn_local;
use xcs_web::{i18n::t, prelude::*};

pub(crate) fn namespace(state: RwSignal<State>, action: NamespaceAction, name: String) {
    let snapshot = state.get_untracked();
    if !snapshot.files.writable || pending(&snapshot) {
        return;
    }
    let mut body = match action {
        NamespaceAction::CreateDirectory => {
            json!({"kind":"mkdir","path":join_path(&snapshot.files.directory,&name)})
        }
        NamespaceAction::Rename(entry) => {
            json!({"kind":"rename","path":text(&entry,&["path"]),"destination":join_path(&snapshot.files.directory,&name)})
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
    let endpoint = format!("{}/namespace", disk_path(&snapshot.files.slot));
    state.update(|s| {
        s.files.mutation_pending = true;
    });
    let epoch = state.with_untracked(|s| s.session_epoch);
    spawn_local(async move {
        let result = api::request("POST", &endpoint, Some(body)).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => record_operation(state, value),
            Err(error) => {
                if error.status == 0 || error.status >= 500 {
                    record_operation(state, json!({"operation_id":id,"state":"in_doubt"}));
                }
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}
pub(crate) fn directory_branch(state: RwSignal<State>, path: String, name: String, select: bool) {
    if state.with_untracked(|s| !s.files.writable || pending(s)) {
        return;
    }
    let operation_id = api::operation_id();
    if operation_id.is_empty() {
        return;
    }
    let slot = state.with_untracked(|s| s.files.slot.clone());
    let action = if select {
        "directory-select"
    } else {
        "directory-branches"
    };
    let endpoint = format!("{}/{action}", disk_path(&slot));
    let epoch = state.with_untracked(|s| s.session_epoch);
    state.update(|s| s.files.mutation_pending = true);
    spawn_local(async move {
        let result = api::request(
            "POST",
            &endpoint,
            Some(json!({"path":path,"name":name,"operation_id":operation_id})),
        )
        .await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => {
                let status = operation_status(&value);
                record_operation(state, value);
                if matches!(status, SaveStatus::Published | SaveStatus::Degraded) {
                    set_notice(
                        state,
                        if select {
                            t(
                                "已恢复此目录的文件分支组合，目录外文件保持不变。",
                                "Restored this folder’s file branch combination; files outside the folder were left unchanged.",
                            )
                        } else {
                            t(
                                "目录分支组合已登记，尚未切换。",
                                "The folder branch combination was registered without switching to it.",
                            )
                        },
                    );
                }
            }
            Err(error) => {
                if error.status == 0 || error.status >= 500 {
                    record_operation(
                        state,
                        json!({"operation_id":operation_id,"state":"in_doubt"}),
                    );
                }
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}

pub(crate) fn select_default_branch(state: RwSignal<State>, branch_id: String) {
    let repository = state.with_untracked(|s| s.files.repository_id.clone());
    if repository.is_empty() || branch_id.is_empty() {
        return;
    }
    record_mutation(
        state,
        "PUT",
        format!(
            "/v1/repositories/{}/default-branch",
            api::segment(&repository)
        ),
        json!({"directory_branch_id":branch_id}),
    );
}
pub(crate) fn rename_directory_branch(
    state: RwSignal<State>,
    repository: String,
    branch_id: String,
    name: String,
) {
    if repository.is_empty()
        || branch_id.is_empty()
        || name.len() > xcs_contract::MAX_NOTE_BYTES
        || !crate::files::model::valid_name(&name)
    {
        return;
    }
    record_mutation(
        state,
        "PATCH",
        format!(
            "/v1/repositories/{}/directory-branches/{}",
            api::segment(&repository),
            api::segment(&branch_id)
        ),
        json!({"name":name}),
    );
}
pub(crate) fn rename_file_branch(
    state: RwSignal<State>,
    file: String,
    branch_id: String,
    name: String,
) {
    if file.is_empty()
        || branch_id.is_empty()
        || name.len() > xcs_contract::MAX_NOTE_BYTES
        || !crate::files::model::valid_name(&name)
    {
        return;
    }
    record_mutation(
        state,
        "PATCH",
        format!(
            "/v1/file-manager/files/{}/branches/{}",
            api::segment(&file),
            api::segment(&branch_id)
        ),
        json!({"name":name}),
    );
}
fn record_mutation(
    state: RwSignal<State>,
    method: &'static str,
    endpoint: String,
    mut body: Value,
) {
    if state.with_untracked(|s| !s.files.writable || s.files.loading || pending(s)) {
        return;
    }
    let operation_id = api::operation_id();
    if operation_id.is_empty() {
        return;
    }
    body["operation_id"] = operation_id.clone().into();
    let epoch = state.with_untracked(|s| s.session_epoch);
    state.update(|s| s.files.mutation_pending = true);
    spawn_local(async move {
        let result = api::request(method, &endpoint, Some(body)).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => record_operation(state, value),
            Err(error) => {
                if error.status == 0 || error.status >= 500 {
                    record_operation(
                        state,
                        json!({"operation_id":operation_id,"state":"in_doubt"}),
                    );
                }
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}

pub(crate) fn update_note(state: RwSignal<State>, version: Option<String>, note: String) {
    if xcs_contract::validate_note(&note).is_err() {
        return;
    }
    let file = state.with_untracked(|s| {
        s.files
            .selected
            .as_ref()
            .map(|entry| text(entry, &["file_id"]))
            .unwrap_or_default()
    });
    let (collection, id) = match version {
        Some(version) => ("versions", version),
        None => ("files", file),
    };
    if id.is_empty() || state.with_untracked(|s| !s.files.writable || pending(s)) {
        return;
    }
    let operation_id = api::operation_id();
    if operation_id.is_empty() {
        return;
    }
    let endpoint = format!(
        "/v1/file-manager/{collection}/{}/metadata",
        api::segment(&id)
    );
    let epoch = state.with_untracked(|s| s.session_epoch);
    state.update(|s| {
        s.files.mutation_pending = true;
    });
    spawn_local(async move {
        let result = api::request(
            "POST",
            &endpoint,
            Some(json!({"note":note,"operation_id":operation_id})),
        )
        .await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => record_operation(state, value),
            Err(error) => {
                if error.status == 0 || error.status >= 500 {
                    record_operation(
                        state,
                        json!({"operation_id":operation_id,"state":"in_doubt"}),
                    );
                }
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}

pub(crate) fn update_node_note(state: RwSignal<State>, node: String, note: String) {
    if node.is_empty() || xcs_contract::validate_note(&note).is_err() {
        return;
    }
    let file = state.with_untracked(|s| {
        s.files
            .selected
            .as_ref()
            .map(|entry| text(entry, &["file_id"]))
            .unwrap_or_default()
    });
    if file.is_empty() {
        return;
    }
    record_mutation(
        state,
        "PATCH",
        format!(
            "/v1/file-manager/files/{}/nodes/{}",
            api::segment(&file),
            api::segment(&node)
        ),
        json!({"note":note}),
    );
}
pub(crate) fn create_branch(
    state: RwSignal<State>,
    version: String,
    node: Option<String>,
    name: String,
) {
    if version.is_empty() {
        return;
    }
    let file = state.with_untracked(|s| {
        s.files
            .selected
            .as_ref()
            .map(|entry| text(entry, &["file_id"]))
            .unwrap_or_default()
    });
    if file.is_empty() {
        return;
    }
    let mut body = json!({"source_version_id":version,"name":name});
    if let Some(node) = node.filter(|id| !id.is_empty()) {
        body["source_node_id"] = node.into();
    }
    file_mutation(state, &file, "branches", body);
}
pub(crate) fn select_branch(state: RwSignal<State>, branch: String) {
    let file = state.with_untracked(|s| {
        s.files
            .selected
            .as_ref()
            .map(|entry| text(entry, &["file_id"]))
            .unwrap_or_default()
    });
    if file.is_empty() {
        return;
    }
    file_mutation(state, &file, "select", json!({"branch_id":branch}));
}
fn file_mutation(state: RwSignal<State>, file: &str, action: &str, mut body: Value) {
    if state.with_untracked(|s| !s.files.writable || pending(s)) {
        return;
    }
    let id = api::operation_id();
    if id.is_empty() {
        return;
    }
    if action == "select" {
        if let Some((repository, path)) = state.with_untracked(|s| {
            s.files
                .selected
                .as_ref()
                .and_then(|entry| crate::files::model::file_context(&s.files, entry))
        }) {
            body["repository_id"] = repository.into();
            body["path"] = path.into();
        }
    }
    body["operation_id"] = id.clone().into();
    let endpoint = format!("/v1/file-manager/files/{}/{action}", api::segment(file));
    state.update(|s| s.files.mutation_pending = true);
    let epoch = state.with_untracked(|s| s.session_epoch);
    spawn_local(async move {
        let result = api::request("POST", &endpoint, Some(body)).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => record_operation(state, value),
            Err(error) => {
                if error.status == 0 || error.status >= 500 {
                    record_operation(state, json!({"operation_id":id,"state":"in_doubt"}));
                }
                fail(state, error);
            }
        }
        state.update(|s| s.files.mutation_pending = false);
    });
}
