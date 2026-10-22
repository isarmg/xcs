use super::{
    actions, backup, batch, branch, inline, merge,
    model::{
        Model, branch_name, branch_writable, file_kind, history_nodes, inherited_content,
        join_path, selected_branch, selected_entry_read_only, selected_file_branch_record,
        selected_file_read_only, visible_content,
    },
    tags, version_tree,
};
use crate::{
    api,
    icons::FileIcon,
    ui::{State, list, text},
};
use json_codec::Value;
use web_interface::components::{EmptyState, LoadingState, Select, SelectOption};
use web_interface::{html, i18n::t, prelude::*};

pub(crate) fn page(state: RwSignal<State>) -> impl IntoView {
    let selected = detail_selection(state);
    view! {
        <section class="zxs-content-stack zxs-entry-page zxs-stack">
            {super::view::download_copy(state)}
            <nav class="zxs-entry-navigation zxs-flow" aria-label=t("详情导航","Details navigation")>
                <button class="zxs-button" on:click=move |_|state.update(|s|{s.files.close_overlays();s.page="files".into();})><FileIcon name="back"/>{t("返回文件列表","Back to files")}</button>
                <span>{move ||state.with(|s|join_path(&s.files.slot,&s.files.directory))}</span>
            </nav>
            {move ||match selected.get() {
                Some(entry) => entry_view(state, entry).into_any(),
                None => view!{<EmptyState message=t("请在文件页面选择一个文件或文件夹，查看详情。","Choose a file or folder on the Files page to view its details.")/>}.into_any(),
            }}
        </section>
    }
}
/// Detail controls own browser focus and popover state. Refreshing metadata must
/// update their signals without recreating the controls for the same entry.
fn detail_selection(state: RwSignal<State>) -> Memo<Option<Value>> {
    Memo::new(move |_| state.with(|s| s.files.selected.clone()))
}

fn file_note_ready(model: &Model) -> bool {
    !model.history_loading && model.file_history["note"].is_string()
}
fn entry_view(state: RwSignal<State>, entry: Value) -> impl IntoView {
    let name = text(&entry, &["name"]);
    let is_file = file_kind(&entry);
    let rename_entry = entry.clone();
    let delete_entry = StoredValue::new(entry.clone());
    let backup_entry = entry.clone();
    let open_path = text(&entry, &["path"]);
    let preview_entry = entry.clone();
    let download_entry = entry.clone();
    let file_input = NodeRef::<html::Input>::new();
    let folder_path = text(&entry, &["path"]);
    let folder_select_path = folder_path.clone();
    let options = Signal::derive(move || {
        list(&state.with(|s| s.files.file_history.clone()), "branches")
            .into_iter()
            .map(|branch| {
                let readonly = !branch_writable(&branch);
                let label = if readonly {
                    format!("{} · {}", branch_name(&branch), t("只读", "Read only"))
                } else {
                    branch_name(&branch)
                };
                let mut option = SelectOption::new(text(&branch, &["branch_id"]), label);
                option.disabled = readonly;
                option
            })
            .collect::<Vec<_>>()
    });
    let value = Signal::derive(move || state.with(|s| selected_branch(&s.files)));
    let rename_branch = Memo::new(move |_| state.with(|s| selected_file_branch_record(&s.files)));
    let disabled = Signal::derive(move || {
        state.with(|s| {
            !actions::write_allowed(s)
                || s.files.history_loading
                || list(&s.files.file_history, "branches").is_empty()
        })
    });
    let merge_disabled = Signal::derive(move || {
        state.with(|s| !actions::write_allowed(s) || !merge::available(&s.files))
    });
    let can_modify = Signal::derive(move || {
        state.with(|s| s.files.writable && !selected_entry_read_only(&s.files))
    });
    let mutation_disabled =
        Signal::derive(move || !can_modify.get() || !state.with(actions::write_allowed));
    let note_disabled = Signal::derive(move || {
        mutation_disabled.get() || state.with(|s| !file_note_ready(&s.files))
    });
    let content_ready = Signal::derive(move || {
        state.with(|s| s.session_active() && s.files.selected.as_ref().is_some_and(visible_content))
    });
    let inherited = Signal::derive(move || {
        state.with(|s| s.files.selected.as_ref().is_some_and(inherited_content))
    });
    let current_branch_disabled = Signal::derive(move || {
        mutation_disabled.get() || !content_ready.get() || state.with(|s| s.files.loading)
    });
    let upload_disabled = Signal::derive(move || {
        mutation_disabled.get()
            || state.with(|s| {
                s.files.loading || s.files.history_loading || selected_file_read_only(&s.files)
            })
    });
    view! {
        <article class="zxs-content-panel zxs-entry-content zxs-frame zxs-stack" aria-label=t("文件详情","File details")>
            <div class="zxs-file-heading zxs-flow"><h2 class="zxs-row">{name}</h2><div class="zxs-entry-actions zxs-flow">
                {(!is_file).then(move ||view!{<button class="zxs-button" on:click=move |_|actions::go_directory(state,open_path.clone())><FileIcon name="folder"/>{t("打开文件夹","Open folder")}</button>})}
                <Show when=move ||can_modify.get()>{inline::control(state,t("重命名","Rename"),Some("rename"),inline::Target::RenameEntry(rename_entry.clone()),mutation_disabled)}</Show>
                <button class="zxs-button" disabled=move ||state.with(|s|actions::pending(s)||super::model::file_context(&s.files,&backup_entry).is_none()) on:click=move |_|if let Some(entry)=state.with_untracked(|s|s.files.selected.clone()){backup::start_entry(state,entry);}>{t("备份设置","Backup settings")}</button>
                <Show when=move ||can_modify.get()><button class="zxs-button" disabled=move ||mutation_disabled.get() on:click=move |_|batch::start(state,batch::Kind::Delete,Some(delete_entry.get_value()))><FileIcon name="delete"/>{t("删除","Delete")}</button></Show>
            </div></div>
            {backup::view(state)}
            <dl class="zxs-file-properties"><div><dt class="zxs-row">{t("位置","Location")}</dt><dd class="zxs-row">{join_path(&state.with_untracked(|s|s.files.slot.clone()),&text(&entry,&["path"]))}</dd></div><div><dt class="zxs-row">{t("类型","Type")}</dt><dd class="zxs-row">{if is_file{t("文件","File")}else{t("文件夹","Folder")}}</dd></div></dl>
            {(!is_file).then(move ||view!{<Show when=move ||can_modify.get()><section class="zxs-entry-section zxs-stack zxs-stack--flush"><h3 class="zxs-row">{t("目录分支","Folder branches")}</h3><p class="zxs-row">{t("目录分支记录此目录及子目录的文件分支组合，不创建目录私有的文件分支。修改只在当前分支追加子树节点；切换恢复所选组合，未写入的文件继续显示父版本，目录外文件保持不变。","A folder branch records the shared file branch combination in this folder and its subfolders. Changes append subtree nodes within the current branch. Switching restores the selected combination; unwritten files inherit their parent versions and files outside the folder are unchanged.")}</p><div class="zxs-file-toolbar zxs-flow">{branch::control(state,t("创建目录分支","Create folder branch"),branch::Target::Directory{path:folder_path.clone(),select:false},mutation_disabled)}{branch::control(state,t("切换目录分支","Switch folder branch"),branch::Target::Directory{path:folder_select_path.clone(),select:true},mutation_disabled)}</div><p class="zxs-row">{t("目录分支合并待确认，暂不提供操作。","Folder branch merging awaits confirmation.")}</p></section></Show>})}
            {is_file.then(move ||view! {
                <section class="zxs-entry-section zxs-stack zxs-stack--flush"><h3 class="zxs-row">{t("文件备注","File note")}</h3><Show when=move ||can_modify.get() fallback=move ||view!{<p class="zxs-row">{move ||state.with(|s|text(&s.files.file_history,&["note"]))}</p>}>{inline::control(state,t("编辑文件备注","Edit file note"),None,inline::Target::FileNote,note_disabled)}</Show></section>
                <div class="zxs-file-toolbar zxs-flow"><button class="zxs-button" disabled=move ||!content_ready.get() on:click=move |_|actions::preview(state,preview_entry.clone())>{t("预览","Preview")}</button><button class="zxs-button" disabled=move ||!content_ready.get()||state.with(|s|s.files.download_pending||s.files.download_copy.is_some()) on:click=move |_|actions::download(state,download_entry.clone())><FileIcon name="download"/>{t("下载","Download")}</button><Show when=move ||state.with(|s|s.files.download_pending||s.files.download_cleanup_pending)><button class="zxs-button" on:click=move |_|api::cancel_download()>{t("取消下载","Cancel download")}</button><span role="status">{move ||state.with(|s|if s.files.download_pending {t("正在下载并校验…","Downloading and verifying…")} else {t("正在清理临时副本…","Cleaning up temporary copy…")})}</span></Show><Show when=move ||can_modify.get()><button class="zxs-button" disabled=move ||upload_disabled.get() on:click=move |_|{if actions::pick_upload(state,true)&& let Some(input)=file_input.get_untracked(){input.click();}}><FileIcon name="upload"/>{t("写入当前分支","Write to current branch")}</button><input node_ref=file_input hidden type="file" on:cancel=move |_: web_sys::Event|actions::cancel_picker(state) on:change=move |_|{if let Some(input)=file_input.get_untracked(){actions::upload_files(state,input.files());input.set_value("");}}/></Show></div>
                <Show when=move ||inherited.get()><p class="zxs-row" role="status">{t("此分支尚未写入，正在显示父版本，可预览和下载；写入后才保存本分支自己的版本。","This unwritten branch is displaying its parent version, available for preview and download. Writing creates this branch’s own version.")}</p></Show>
                <Show when=move ||can_modify.get()><section class="zxs-entry-section zxs-file-branches zxs-stack zxs-stack--flush"><h3 class="zxs-row">{t("文件分支","File branches")}</h3><p class="zxs-row">{t("创建时只登记分支并引用父节点，不创建占位文件或复制正文。首次写入后形成自己的版本；切换只更新当前目录的文件引用。","Creating a branch only registers a reference to its parent node, without placeholders or copied content. The first write creates its own version; switching updates only this folder’s file reference.")}</p>
                    <div class="zxs-file-toolbar zxs-flow"><Select label=t("当前分支","Current branch") options=options value=value disabled=disabled placeholder=t("主版本","Main") on_change=Callback::new(move |branch:String|actions::select_branch(state,branch))/>
                        {move ||rename_branch.get().map(|(file,branch_id,name)|inline::control(state,t("重命名文件分支","Rename file branch"),Some("rename"),inline::Target::RenameFileBranch{file,branch_id,name},disabled))}
                    </div>
                    <div class="zxs-file-toolbar zxs-flow">{branch::control(state,t("从当前内容创建分支","Branch from current content"),branch::Target::CurrentFileVersion,current_branch_disabled)}<button class="zxs-button" disabled=move ||merge_disabled.get() on:click=move |_|merge::start(state)>{t("合并文件分支","Merge file branches")}</button></div>
                    {merge::view(state)}
                </section></Show>
                <section class="zxs-entry-section zxs-entry-version-history zxs-stack"><h3 class="zxs-row">{t("版本历史","Version history")}</h3><Show when=move ||state.with(|s|s.files.history_loading)><LoadingState/></Show>
                    <Show when=move ||can_modify.get()>{tags::view(state)}</Show>
                    {version_tree::view(state)}
                    {merge::history(state)}
                    <Show when=move ||state.with(|s|!s.files.history_loading&&history_nodes(&s.files.file_history).is_empty())><p class="zxs-row">{t("尚无版本记录。","No versions have been recorded.")}</p></Show>
                </section>
            })}
        </article>
    }
}

#[cfg(test)]
#[path = "../../../../test/web/unit/files/detail/detail_tests.rs"]
mod detail_tests;
