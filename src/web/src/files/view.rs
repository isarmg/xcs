use super::{
    actions, backup, batch, branch, inline,
    model::{
        breadcrumbs, directory_branch_name, directory_branch_value, entry_key, file_kind,
        selected_directory_branch,
    },
};
use crate::{
    format,
    icons::FileIcon,
    ui::{State, text},
};
use web_interface::components::{Dialog, EmptyState, LoadingState, Select, SelectOption};
use web_interface::{html, i18n::t, prelude::*};

pub(crate) fn view(state: RwSignal<State>) -> impl IntoView {
    let slot = Memo::new(move |_| state.with(|s| s.files.slot.clone()));
    view! {
        <section class="zxs-content-stack zxs-file-manager zxs-stack">
            {move ||if slot.get().is_empty(){disk_roots(state).into_any()}else{directory(state).into_any()}}
            {upload_queue(state)}
        </section>
    }
}
fn disk_roots(state: RwSignal<State>) -> impl IntoView {
    view! {
        <p class="zxs-row">{t("选择一个硬盘目录查看文件。","Choose a disk directory to browse its files.")}</p>
        {move ||{
            let model=state.with(|s|s.files.clone());
            if model.loading_disks&&!model.disks_loaded {return view!{<LoadingState/>}.into_any();}
            if model.disks_loaded&&model.disks.is_empty(){return view!{<EmptyState message=t("没有可访问的硬盘目录。","No disk directories are available.")/>}.into_any();}
            view!{<div class="zxs-file-layout zxs-stack zxs-surface"><section class="zxs-content-panel zxs-file-list-panel zxs-frame zxs-stack zxs-stack--flush" aria-label=t("硬盘目录","Disk directories")><div class="zxs-table-scroll">
                <table class="zxs-table zxs-file-table zxs-disk-table">
                    <thead><tr><th scope="col">{t("名称","Name")}</th><th scope="col">{t("访问权限","Access")}</th><th scope="col">{t("可用空间","Available space")}</th><th scope="col">{t("操作","Actions")}</th></tr></thead>
                    <tbody>{model.disks.into_iter().map(move |disk|{
                        let id=text(&disk,&["slot_id","id"]);let data_slot=id.clone();let row_slot=id.clone();let name_slot=id.clone();let action_slot=id.clone();
                        let online=disk["online"].as_bool().unwrap_or(false);let writable=disk["writable"].as_bool().unwrap_or(false);
                        let available=format::parse_u64(&disk["available_bytes"]).map(format::bytes).unwrap_or_else(||t("容量不可用","Capacity unavailable"));
                        view!{<tr data-online=online.to_string() on:click=move |_|{if online{actions::open_disk(state,row_slot.clone());}}>
                            <td><button class="zxs-button zxs-file-name zxs-disk-entry" data-slot=data_slot disabled=!online on:click=move |event|{event.stop_propagation();actions::open_disk(state,name_slot.clone());}><FileIcon name="folder"/><span>{id}</span></button></td>
                            <td>{if online{if writable{t("可读写","Read and write")}else{t("只读","Read only")}}else{t("离线","Offline")}}</td>
                            <td>{available}</td>
                            <td><div class="zxs-entry-actions zxs-flow"><button class="zxs-button" disabled=!online on:click=move |event|{event.stop_propagation();actions::open_disk(state,action_slot.clone());}>{t("打开","Open")}</button></div></td>
                        </tr>}
                    }).collect_view()}</tbody>
                </table>
            </div></section></div>}.into_any()
        }}
    }
}
pub(crate) fn overlays(state: RwSignal<State>) -> impl IntoView {
    let preview = Memo::new(move |_| state.with(|s| s.files.preview.clone()));
    view! {
        {move ||preview.get().map(|(name,content)|view!{<Dialog title=name close=Callback::new(move |_|state.update(|s|s.files.close_overlays()))><p class="zxs-row">{t("文本预览 · 已校验内容完整性","Text preview · Content integrity verified")}</p><pre class="zxs-file-preview">{content.clone()}</pre></Dialog>})}
    }
}
fn directory(state: RwSignal<State>) -> impl IntoView {
    let file_input = NodeRef::<html::Input>::new();
    let default_scope = Memo::new(move |_| state.with(|s| s.files.default_branch_scope));
    let rename_branch = Memo::new(move |_| state.with(|s| selected_directory_branch(&s.files)));
    let folder_options = Signal::derive(move || {
        state.with(|s| {
            s.files
                .directory_branches
                .iter()
                .map(|branch| {
                    SelectOption::new(
                        directory_branch_value(branch, s.files.default_branch_scope),
                        directory_branch_name(branch),
                    )
                })
                .collect::<Vec<_>>()
        })
    });
    let folder_value = Signal::derive(move || {
        state.with(|s| {
            s.files
                .directory_branches
                .iter()
                .find(|branch| branch["current"] == true)
                .map(|branch| directory_branch_value(branch, s.files.default_branch_scope))
                .unwrap_or_default()
        })
    });
    let folder_disabled = Signal::derive(move || {
        state.with(|s| {
            !actions::write_allowed(s)
                || !s.files.loaded
                || !s.files.directory_branches_supported
                || s.files.directory_branches.is_empty()
        })
    });
    let create_branch_disabled = Signal::derive(move || {
        state.with(|s| {
            !actions::write_allowed(s) || !s.files.loaded || !s.files.directory_branches_supported
        })
    });

    let mutation_disabled = Signal::derive(move || state.with(|s| !actions::write_allowed(s)));
    let batch_disabled = Signal::derive(move || {
        state.with(|s| !actions::write_allowed(s) || !batch::available(&s.files))
    });
    view! {
        <nav class="zxs-breadcrumb zxs-flow" aria-label=t("当前位置","Current location")>
            <button class="zxs-button" on:click=move |_|actions::go_root(state)><FileIcon name="home"/>{t("所有硬盘","All disks")}</button><span aria-hidden="true">"/"</span>
            <button class="zxs-button" on:click=move |_|actions::go_directory(state,String::new())>{move ||state.with(|s|s.files.slot.clone())}</button>
            {move ||breadcrumbs(&state.with(|s|s.files.directory.clone())).into_iter().map(move |(name,path)|view!{<span aria-hidden="true">"/"</span><button class="zxs-button" on:click=move |_|actions::go_directory(state,path.clone())>{name}</button>}).collect_view()}
        </nav>
        <div class="zxs-file-toolbar zxs-flow">
            <button class="zxs-button" disabled=move ||state.with(|s|!actions::write_allowed(s)) on:click=move |_|{if actions::pick_upload(state,false)&& let Some(input)=file_input.get_untracked(){input.click();}}><FileIcon name="upload"/>{t("上传文件","Upload files")}</button>
            <input node_ref=file_input hidden type="file" on:cancel=move |_: web_sys::Event|actions::cancel_picker(state) multiple on:change=move |_|{if let Some(input)=file_input.get_untracked(){actions::upload_files(state,input.files());input.set_value("");}}/>
            {inline::control(state,t("新建文件夹","New folder"),Some("create"),inline::Target::CreateDirectory,mutation_disabled)}
            <button class="zxs-button" disabled=move ||batch_disabled.get() on:click=move |_|batch::start(state,batch::Kind::Move,None)>{t("移动","Move")}</button>
            <button class="zxs-button" disabled=move ||batch_disabled.get() on:click=move |_|batch::start(state,batch::Kind::Copy,None)>{t("复制","Copy")}</button>
            <button class="zxs-button" disabled=move ||batch_disabled.get() on:click=move |_|batch::start(state,batch::Kind::Rename,None)><FileIcon name="rename"/>{t("重命名","Rename")}</button>
            <button class="zxs-button" disabled=move ||batch_disabled.get() on:click=move |_|batch::start(state,batch::Kind::Delete,None)><FileIcon name="delete"/>{t("删除","Delete")}</button>
            {branch::control(state,t("创建目录分支","Create folder branch"),branch::Target::CurrentDirectory,create_branch_disabled)}
            <button class="zxs-button" disabled=move ||state.with(|s|s.files.repository_id.is_empty()||actions::pending(s)) on:click=move |_|backup::start_current(state)>{t("备份设置","Backup settings")}</button>
            <label class="zxs-file-search"><span class="zxs-visually-hidden">{t("搜索当前文件夹","Search this folder")}</span><input class="zxs-input zxs-input--compact" type="search" aria-label=t("搜索当前文件夹","Search this folder") placeholder=t("搜索当前文件夹","Search this folder") prop:value=move ||state.with(|s|s.files.query.clone()) on:input=move |event|state.update(|s|s.files.query=event_target_value(&event))/></label>
        </div>
        <div class="zxs-file-toolbar zxs-flow zxs-surface-end">{move ||view!{<Select label=if default_scope.get(){t("当前主分支（SMB 默认）","Current main branch (SMB default)")}else{t("当前目录分支","Current folder branch")} options=folder_options value=folder_value disabled=folder_disabled placeholder=if default_scope.get(){t("主版本","Main")}else{t("尚无目录分支","No folder branches")} on_change=Callback::new(move |value:String|{if state.with_untracked(|s|s.files.default_branch_scope){actions::select_default_branch(state,value)}else{let path=state.with_untracked(|s|s.files.directory.clone());actions::directory_branch(state,path,value,true);}})/>}}
            {move ||rename_branch.get().map(|(repository,branch_id,name)|inline::control(state,t("重命名分支","Rename branch"),Some("rename"),inline::Target::RenameDirectoryBranch{repository,branch_id,name},folder_disabled))}
            <p class="zxs-row">{move ||if default_scope.get(){t("恢复所选文件分支组合，并作为 SMB 默认视图。","Restores the selected file branch combination as the default SMB view.")}else{t("恢复此目录保存的文件分支组合，不改变目录外文件。","Restores this folder’s saved file branch combination without changing files outside it.")}}</p></div>
        <Show when=move ||state.with(|s|s.files.loaded&&s.files.writable&&!s.files.directory_branches_supported)><p class="zxs-row">{t("请选择具体文件夹后管理目录分支；此硬盘根目录包含多个已有的文件目录。","Select a specific folder to manage folder branches; this disk root contains multiple existing file namespaces.")}</p></Show>
        <Show when=move ||state.with(|s|s.files.loaded&&!s.files.writable)><p class="zxs-row">{t("此硬盘目录当前只读。","This disk directory is read only.")}</p></Show>
        {backup::view(state)}
        {batch::view(state)}
        <div class="zxs-file-layout zxs-stack zxs-surface"><section class="zxs-content-panel zxs-file-list-panel zxs-frame zxs-stack zxs-stack--flush">{file_table(state)}</section></div>
    }
}
fn file_table(state: RwSignal<State>) -> impl IntoView {
    view! {
        <div aria-busy=move ||state.with(|s|s.files.loading).to_string()>
            <Show when=move ||state.with(|s|s.files.loading&&!s.files.loaded)><LoadingState/></Show>
            <div class="zxs-table-scroll"><table class="zxs-table zxs-file-table"><thead><tr><th scope="col"><label class="zxs-file-check zxs-flow"><input type="checkbox" aria-label=t("选择当前显示的全部条目","Select all visible entries") disabled=move ||state.with(|s|!s.files.writable||actions::pending(s)) prop:checked=move ||state.with(|s|{let shown=s.files.entries.iter().filter(|entry|text(entry,&["name"]).to_lowercase().contains(&s.files.query.to_lowercase())).collect::<Vec<_>>();!shown.is_empty()&&shown.iter().all(|entry|s.files.checked_entries.contains(&entry_key(entry)))}) on:change=move |event|{let checked=event_target_checked(&event);state.update(|s|{for entry in s.files.entries.iter().filter(|entry|text(entry,&["name"]).to_lowercase().contains(&s.files.query.to_lowercase())){let id=entry_key(entry);if checked{s.files.checked_entries.insert(id);}else{s.files.checked_entries.remove(&id);}}});}/><span>{t("名称","Name")}</span></label></th><th scope="col">{t("类型","Type")}</th><th scope="col">{t("大小","Size")}</th><th scope="col">{t("操作","Actions")}</th></tr></thead>
                <tbody>{move ||{
                    let model=state.with(|s|s.files.clone());
                    let query=model.query.to_lowercase();let selected=model.selected.as_ref().map(|entry|text(entry,&["entry_id","path"]));
                    let mut entries=model.entries.into_iter().filter(|entry|text(entry,&["name"]).to_lowercase().contains(&query)).collect::<Vec<_>>();
                    entries.sort_by_key(|entry|(file_kind(entry),text(entry,&["name"]).to_lowercase()));
                    entries.into_iter().map(move |entry|{
                        let name=text(&entry,&["name"]);let title=name.clone();let directory=!file_kind(&entry);let selected=selected.as_ref()==Some(&text(&entry,&["entry_id","path"]));
                        let row_entry=entry.clone();let name_entry=entry.clone();let detail_entry=entry.clone();let path=text(&entry,&["path"]);let check_id=entry_key(&entry);let input_id=check_id.clone();let check_name=format!("{} {}",t("选择","Select"),name);
                        view!{<tr class:is-selected=selected aria-selected=selected.to_string() on:click=move |_|actions::select_entry(state,row_entry.clone())>
                            <td><div class="zxs-file-select-name zxs-flow"><input class="zxs-entry-checkbox" type="checkbox" aria-label=check_name disabled=move ||state.with(|s|!s.files.writable||actions::pending(s)) prop:checked=move ||state.with(|s|s.files.checked_entries.contains(&check_id)) on:click=move |event|event.stop_propagation() on:change=move |event|{let checked=event_target_checked(&event);state.update(|s|{if checked{s.files.checked_entries.insert(input_id.clone());}else{s.files.checked_entries.remove(&input_id);}});}/><button class="zxs-button zxs-file-name" title=title on:click=move |event|{event.stop_propagation();if directory{actions::go_directory(state,path.clone())}else{actions::select_entry(state,name_entry.clone())}}><FileIcon name=if directory{"folder"}else{"file"}/><span>{name}</span></button></div></td>
                            <td>{if directory{t("文件夹","Folder")}else{t("文件","File")}}</td><td>{if directory{"—".into()}else{format::parse_u64(&entry["size"]).map(format::bytes).unwrap_or_else(||"—".into())}}</td>
                            <td><div class="zxs-entry-actions zxs-flow"><button class="zxs-button" on:click=move |event|{event.stop_propagation();actions::select_entry(state,detail_entry.clone());}>{t("详情","Details")}</button></div></td>
                        </tr>}
                    }).collect_view()
                }}</tbody>
            </table></div>
            <Show when=move ||state.with(|s|s.files.loaded&&s.files.entries.is_empty())><p class="zxs-file-empty">{t("这个文件夹是空的。","This folder is empty.")}</p></Show>
            <Show when=move ||state.with(|s|s.files.loaded&&!s.files.entries.is_empty()&&!s.files.entries.iter().any(|entry|text(entry,&["name"]).to_lowercase().contains(&s.files.query.to_lowercase())))><p class="zxs-file-empty">{t("没有匹配的文件。","No files match your search.")}</p></Show>
        </div>
    }
}
fn upload_queue(state: RwSignal<State>) -> impl IntoView {
    view! {
        {crate::transfers::summary(state)}
    }
}
