use super::{Model, UploadRow};
use crate::{
    files::{SaveStatus, check_upload},
    format,
    ui::{State, navigate},
};
use web_interface::{i18n::t, prelude::*};

const STATUSES: [SaveStatus; 7] = [
    SaveStatus::Waiting,
    SaveStatus::Uploading,
    SaveStatus::Saving,
    SaveStatus::Unknown,
    SaveStatus::Published,
    SaveStatus::Degraded,
    SaveStatus::Rejected,
];
fn status_key(status: SaveStatus) -> &'static str {
    match status {
        SaveStatus::Waiting => "waiting",
        SaveStatus::Uploading => "uploading",
        SaveStatus::Saving => "saving",
        SaveStatus::Unknown => "unknown",
        SaveStatus::Published => "published",
        SaveStatus::Degraded => "degraded",
        SaveStatus::Rejected => "rejected",
    }
}
fn counts(model: &Model) -> String {
    format!(
        "{} {} · {} {} · {} {}",
        model.rows.iter().filter(|r| r.status.pending()).count(),
        t("进行中", "in progress"),
        model.count(Some(SaveStatus::Published)) + model.count(Some(SaveStatus::Degraded)),
        t("已保存", "saved"),
        model.count(Some(SaveStatus::Rejected)),
        t("未保存", "not saved")
    )
}
pub(crate) fn summary(state: RwSignal<State>) -> impl IntoView {
    view! {
        <Show when=move ||state.with(|s|!s.transfers.rows.is_empty())>
            <aside class="zxs-transfer-summary zxs-content-panel zxs-frame zxs-flow" aria-label=t("传输概况","Transfer summary")>
                <span class="zxs-row" role="status">{move ||state.with(|s|counts(&s.transfers))}</span>
                <button class="zxs-button" on:click=move |_|navigate(state,"transfers")>{t("查看传输任务","View transfers")}</button>
            </aside>
        </Show>
    }
}
pub(crate) fn view(state: RwSignal<State>) -> impl IntoView {
    view! {
        <section class="zxs-transfers-page zxs-content-stack zxs-stack" aria-label=t("传输任务","Transfers")>
            <header class="zxs-transfer-heading zxs-flow">
                <h1 class="zxs-row">{t("传输任务","Transfers")}</h1>
                <button class="zxs-button" disabled=move ||state.with(|s|!s.transfers.rows.iter().any(|row|matches!(row.status, SaveStatus::Published|SaveStatus::Degraded))) on:click=move |_|state.update(|s|s.transfers.clear_completed())>{t("清除已完成","Clear completed")}</button>
            </header>
            <p class="zxs-row">{t("仅记录本次登录会话。退出或重载后清空；清除记录不会删除服务器文件。","This sign-in session only. Signing out or reloading clears these records; clearing records never deletes server files.")}</p>
            <p class="zxs-row" role="status" aria-live="polite">{move ||state.with(|s|counts(&s.transfers))}</p>
            <nav class="zxs-transfer-filters zxs-flow" aria-label=t("按传输状态筛选","Filter transfers by status")>
                <button class="zxs-button" aria-pressed=move ||state.with(|s|s.transfers.filter.is_none()).to_string() on:click=move |_|state.update(|s|s.transfers.filter=None)>{move ||format!("{} ({})",t("全部","All"),state.with(|s|s.transfers.rows.len()))}</button>
                {STATUSES.into_iter().map(move |status|view! {
                    <button class="zxs-button" data-status=status_key(status) aria-pressed=move ||state.with(|s|s.transfers.filter==Some(status)).to_string() on:click=move |_|state.update(|s|s.transfers.filter=Some(status))>{move ||format!("{} ({})",status.label(),state.with(|s|s.transfers.count(Some(status))))}</button>
                }).collect_view()}
            </nav>
            <Show when=move ||state.with(|s|s.transfers.count(s.transfers.filter)==0)>
                <div class="zxs-content-panel zxs-frame zxs-stack"><p class="zxs-row">{move ||state.with(|s|if s.transfers.rows.is_empty(){t("暂无传输任务。在文件页选择硬盘和目录后上传文件。","No transfers yet. Choose a disk and folder on Files, then upload.")}else{t("没有符合此状态的任务。","No transfers match this status.")})}</p><button class="zxs-button" on:click=move |_|navigate(state,"files")>{t("前往文件","Go to files")}</button></div>
            </Show>
            <ul class="zxs-transfer-list zxs-stack">
                <ForSnapshot
                    each=move ||state.with(|s|s.transfers.rows.iter().filter(|row|s.transfers.filter.is_none_or(|status|row.status==status)).rev().cloned().collect::<Vec<_>>())
                    key=|row|row.operation_id.clone()
                    children=move |row|transfer_card(state,row)
                />
            </ul>
        </section>
    }
}

fn transfer_card(state: RwSignal<State>, row: ReadSignal<UploadRow>) -> impl IntoView {
    view! {
        <li class="zxs-transfer-card zxs-content-panel zxs-frame zxs-stack" data-status=move ||row.with(|row|status_key(row.status))>
            <div class="zxs-transfer-heading zxs-flow"><h2 class="zxs-row">{move ||row.with(|row|row.name.clone())}</h2><span class="zxs-row">{move ||row.with(|row|row.status.label())}</span></div>
            <dl class="zxs-transfer-meta">
                <div><dt class="zxs-row">{t("目标位置","Destination")}</dt><dd class="zxs-row">{move ||row.with(|row|if row.path.is_empty() {t("未确定（未发送文件）","Undetermined (no file sent)")} else {format!("{} / {}",row.slot,row.path)})}</dd></div>
                <div><dt class="zxs-row">{t("大小","Size")}</dt><dd class="zxs-row">{move ||row.with(|row|row.size.map(format::bytes).unwrap_or_else(||t("未知","Unknown")))}</dd></div>
                <div><dt class="zxs-row">{t("开始时间","Started")}</dt><dd class="zxs-row">{move ||row.with(|row|row.started_at.as_deref().map(format::timestamp).unwrap_or_else(||t("未开始","Not started")))}</dd></div>
                <div><dt class="zxs-row">{t("结束时间","Finished")}</dt><dd class="zxs-row">{move ||row.with(|row|row.completed_at.as_deref().map(format::timestamp).unwrap_or_else(||t("尚未结束","Not finished")))}</dd></div>
            </dl>
            <p class="zxs-transfer-message zxs-row">{move ||row.with(|row|row.message.clone())}</p>
            <Show when=move ||row.with(|row|row.status==SaveStatus::Saving)><p class="zxs-row">{t("文件已传输，正在等待服务器确认保存。","File bytes transferred; waiting for server confirmation of the save.")}</p></Show>
            <Show when=move ||row.with(|row|row.status==SaveStatus::Unknown)><p class="zxs-row">{t("结果尚未确认。仅查询原操作，不会重新上传。","The result is unconfirmed. Only the original operation is queried; this does not upload again.")}</p></Show>
            {move ||row.with(|row|row.request_id.clone()).map(|request|view!{<p class="zxs-row">{format!("{}: {request}",t("请求编号","Request ID"))}</p>})}
            <details class="zxs-transfer-operation"><summary class="zxs-row">{t("操作详情","Operation details")}</summary><p class="zxs-row">{move ||row.with(|row|format!("{}: {}",if row.commit_attempted {t("操作编号","Operation ID")} else {t("本地任务编号（未提交保存）","Local task ID (save not submitted)")},row.operation_id))}</p>
                {move ||row.with(|row|(!row.branch_id.is_empty()).then(||row.branch_id.clone())).map(|branch|view!{<p class="zxs-row">{format!("{}: {branch}",t("分支","Branch"))}</p>})}
            </details>
            <Show when=move ||row.with(|row|row.status==SaveStatus::Unknown)><button class="zxs-button" disabled=move ||row.with(|row|row.polling) on:click=move |_|{let id=row.with_untracked(|row|row.operation_id.clone());check_upload(state,&id)}>{move ||row.with(|row|if row.polling{t("正在查询…","Checking…")}else{t("查询原操作","Check original operation")})}</button></Show>
        </li>
    }
}
