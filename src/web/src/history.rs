use crate::{
    api, format,
    ui::{State, current_session, fail, feedback, list, text},
};
use serde_json::Value;
use web_interface::components::{
    EmptyState, ErrorState, Feedback, LoadingState, Select, SelectOption,
};
use web_interface::tasks::spawn_local;
use web_interface::{i18n::t, prelude::*};

#[derive(Clone, Default)]
pub(crate) struct Model {
    pub events: Vec<Value>,
    pub loading: bool,
    pub loaded: bool,
    pub failure: Option<Feedback>,
    pub disk: String,
}
pub(crate) fn load(state: RwSignal<State>) {
    if state.with_untracked(|s| s.activity.loading) {
        return;
    }
    state.update(|s| s.activity.loading = true);
    let epoch = state.with_untracked(|s| s.session_epoch);
    spawn_local(async move {
        let result = api::request("GET", "/api/file-manager/history", None).await;
        if !current_session(state, epoch) {
            return;
        }
        match result {
            Ok(value) => state.update(|s| {
                s.activity.events = list(&value, "events");
                s.activity.loaded = true;
                s.activity.failure = None;
            }),
            Err(error) => {
                if error.status == 401 {
                    fail(state, error)
                } else {
                    state.update(|s| s.activity.failure = Some(feedback(&error)));
                }
            }
        }
        state.update(|s| s.activity.loading = false);
    });
}
fn action_label(action: &str) -> String {
    match action {
        "file_branch_pending" | "branch_pending" => t("分支登记", "Registered branch"),
        "upload" | "commit" | "save" | "file_save" | "content" => t("保存文件", "Saved file"),
        "directory.branches.create" => t("登记目录分支", "Registered folder branch"),
        "directory_branch" | "directory_branches" => t("创建目录分支", "Created folder branches"),
        "directory.branches.select" => t("切换目录分支", "Switched folder branch"),
        "directory_selection" | "directory_select" => t("切换目录分支", "Switched folder branches"),
        "directory.branch.rename" => t("重命名目录分支", "Renamed folder branch"),
        "file.path.rename" => t("文件路径变化", "File path changed"),
        "file.path.metadata" => t("更新节点备注", "Updated node note"),
        "file.branch.rename" => t("重命名文件分支", "Renamed file branch"),
        "file.branches.merge" => t("合并文件分支", "Merged file branches"),
        "file.tags.update" => t("更新节点标签", "Updated node tags"),
        "backup.configure" => t("更新备份策略", "Updated backup configuration"),
        "directory.scope.initialize" => t("登记目录范围", "Registered folder scope"),
        "directory.namespace.move" => t("批量移动", "Moved selected branches"),
        "directory.namespace.copy" => t("批量复制", "Copied to selected branches"),
        "directory.namespace.delete" => t("批量删除", "Deleted from selected branches"),
        "directory.namespace.rename" => t("批量重命名", "Renamed in selected branches"),
        "directory.tree.write" => t("写入目录子树", "Wrote folder subtree"),
        "directory.default.select" => t(
            "切换 SMB 默认目录分支",
            "Switched SMB default folder branch",
        ),
        "mkdir" => t("新建文件夹", "Created folder"),
        "rename" => t("重命名", "Renamed"),
        "delete" | "delete_tree" => t("删除", "Deleted"),
        "branch" | "create_branch" | "file_branch_create" | "file_branch" => {
            t("创建文件分支", "Created file branch")
        }
        "select" | "select_branch" | "file_branch_select" | "file_selection" => {
            t("切换文件分支", "Switched file branch")
        }
        "metadata" | "file_metadata" | "version_metadata" | "file_note" | "version_note"
        | "file.metadata" | "version.metadata" => t("更新备注", "Updated note"),
        _ => t("文件操作", "File operation"),
    }
}

pub(crate) fn view(state: RwSignal<State>) -> impl IntoView {
    let options = Signal::derive(move || {
        let mut options = vec![SelectOption::new("", t("所有硬盘", "All disks"))];
        options.extend(
            state
                .with(|s| s.files.disks.clone())
                .into_iter()
                .map(|disk| {
                    let id = text(&disk, &["slot_id", "id"]);
                    SelectOption::new(id.clone(), id)
                }),
        );
        options
    });
    let selected = Signal::derive(move || state.with(|s| s.activity.disk.clone()));
    let visible_events = Memo::new(move |_| {
        state.with(|s| {
            s.activity
                .events
                .iter()
                .filter(|event| {
                    s.activity.disk.is_empty() || text(event, &["slot_id"]) == s.activity.disk
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    view! {<section class="zxs-content-stack zxs-history-page zxs-stack">
        <p class="zxs-row">{t("显示最近 200 条已提交的文件操作。选择文件可查看它的版本和分支。","The latest 200 committed file operations are shown. Select a file to view its versions and branches.")}</p>
        <div class="zxs-history-toolbar zxs-flow zxs-flow--end zxs-surface-end"><Select label=t("硬盘","Disk") options=options value=selected on_change=Callback::new(move |disk:String|state.update(|s|s.activity.disk=disk))/></div>
        <Show when=move ||state.with(|s|s.activity.loading&&!s.activity.loaded)><LoadingState/></Show>
        {move ||state.with(|s|s.activity.failure.clone()).map(|failure|view!{<ErrorState message=failure.message request_id=failure.request_id/>})}
        <Show when=move || !visible_events.get().is_empty()><div class="zxs-content-panel zxs-history-list-panel zxs-frame zxs-stack zxs-stack--flush"><ul class="zxs-activity-list zxs-history-list" aria-label=t("文件操作记录","File operation history")>{move ||{
            visible_events.get().into_iter().map(|event|{
                let time=event["timestamp"].as_str().map(|value|if value.contains('T'){format::timestamp(value)}else{format::timestamp_seconds(value)}).unwrap_or_else(||t("未记录时间","Time not recorded"));
                let path=text(&event,&["path","display_path"]);let slot=text(&event,&["slot_id"]);let principal=text(&event,&["principal"]);
                view!{<li class="zxs-history-row zxs-flow zxs-flow--stretch"><span class="zxs-history-action zxs-row">{action_label(&text(&event,&["action"]))}</span><span class="zxs-history-path zxs-row">{if slot.is_empty(){path}else{format!("{slot}/{path}")}}</span><time class="zxs-row">{time}</time><span class="zxs-history-principal zxs-row">{if principal.is_empty(){"–".into()}else{principal}}</span></li>}
            }).collect_view()
        }}</ul></div></Show>
        <Show when=move ||state.with(|s|s.activity.loaded)&&visible_events.get().is_empty()><EmptyState message=t("尚无文件操作记录。","No file operations have been recorded.")/></Show>
    </section>}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_registration_is_distinct_from_legacy_content_creation() {
        assert_eq!(action_label("file_branch_pending"), "Registered branch");
        assert_eq!(action_label("branch_pending"), "Registered branch");
        assert_eq!(action_label("file_branch"), "Created file branch");
        assert_eq!(
            action_label("directory.branch.rename"),
            "Renamed folder branch"
        );
        assert_eq!(
            action_label("directory.default.select"),
            "Switched SMB default folder branch"
        );
        assert_eq!(
            action_label("directory.branches.create"),
            "Registered folder branch"
        );
        assert_eq!(
            action_label("directory.branches.select"),
            "Switched folder branch"
        );
        assert_eq!(action_label("directory.tree.write"), "Wrote folder subtree");
    }
}
