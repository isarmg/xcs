//! Daily management, with system-account administration independent of data disks.
mod request;
use crate::{
    api,
    ui::{State, current_session, fail, list},
};
use json_codec::Value;
use web_interface::i18n::t;
use web_interface::{prelude::*, tasks::spawn_local};

#[derive(Clone, Default)]
pub(crate) struct Model {
    accounts: Vec<Value>,
    fields: request::Fields,
    kind: String,
    busy: bool,
    result: String,
    error: String,
    plan: Value,
    acknowledged: bool,
    uncertain_operation: String,
}
fn permitted(state: &State, storage: bool) -> bool {
    state.session_active()
        && state.admin
        && !state.admin_console.busy
        && (!storage || state.management.storage.writable())
}
fn allowed(state: RwSignal<State>, storage: bool) -> bool {
    state.with_untracked(|s| permitted(s, storage))
}
pub(crate) fn load(state: RwSignal<State>) {
    if !allowed(state, false) {
        return;
    }
    execute(
        state,
        "GET",
        "/api/admin/accounts".into(),
        None,
        false,
        String::new(),
    );
}
fn execute(
    state: RwSignal<State>,
    method: &'static str,
    path: String,
    body: Option<Value>,
    plan: bool,
    operation: String,
) {
    if !allowed(state, false) {
        return;
    }
    let epoch = state.with_untracked(|s| s.session_epoch);
    state.update(|s| {
        s.admin_console.busy = true;
        s.admin_console.error.clear();
    });
    spawn_local(async move {
        let result = api::request(method, &path, body).await;
        if !current_session(state, epoch) {
            return;
        }
        state.update(|s| s.admin_console.busy = false);
        match result {
            Ok(value) => state.update(|s| {
                let m = &mut s.admin_console;
                if path == "/api/admin/accounts" && method == "GET" {
                    m.accounts = list(&value, "accounts");
                }
                if plan {
                    m.plan = value.clone();
                    m.acknowledged = false;
                }
                if path.ends_with("/commit") || path.ends_with("/cancel") {
                    m.plan = Value::Null;
                    m.acknowledged = false;
                }
                m.result = json_codec::to_string_pretty(&value).unwrap_or_default();
                if method != "GET"
                    || (path == "/api/admin/accounts" && m.uncertain_operation == "unknown")
                    || (path == format!("/api/operations/{}", api::segment(&m.uncertain_operation))
                        && matches!(
                            value["commit_state"].as_str(),
                            Some("PUBLISHED" | "ACKED" | "PUBLISHED_DEGRADED" | "ABORTED")
                        ))
                {
                    m.uncertain_operation.clear();
                    if method == "GET" {
                        m.plan = Value::Null;
                        m.acknowledged = false;
                    }
                }
            }),
            Err(error) => {
                state.update(|s| {
                    s.admin_console.error = error.to_string();
                    if method != "GET" && !plan && error.mutation_outcome_unknown() {
                        s.admin_console.uncertain_operation = if operation.is_empty() {
                            "unknown".into()
                        } else {
                            operation
                        };
                    }
                });
                if error.status == 401 {
                    fail(state, error)
                }
            }
        }
    });
}
fn submit(state: RwSignal<State>) {
    let (kind, fields, uncertain) = state.with_untracked(|s| {
        (
            s.admin_console.kind.clone(),
            s.admin_console.fields.clone(),
            !s.admin_console.uncertain_operation.is_empty(),
        )
    });
    if !allowed(state, kind != "account") || uncertain {
        return;
    }
    let operation = api::operation_id();
    match request::build(&kind, &fields, &operation) {
        Ok((method, path, body)) => {
            // Never retain a password after initiating its one authorized request.
            state.update(|s| {
                s.admin_console.fields.remove("password");
                s.admin_console.plan = Value::Null;
                s.admin_console.acknowledged = false;
            });
            execute(
                state,
                method,
                path,
                Some(body),
                matches!(kind.as_str(), "purge" | "split" | "save"),
                if kind == "account" {
                    String::new()
                } else {
                    operation
                },
            );
        }
        Err(error) => state.update(|s| s.admin_console.error = error),
    }
}
fn plan_action(state: RwSignal<State>, commit: bool) {
    if !allowed(state, true)
        || state.with_untracked(|s| !s.admin_console.uncertain_operation.is_empty())
    {
        return;
    }
    let operation = api::operation_id();
    let result = state.with_untracked(|s| {
        request::plan_action(
            &s.admin_console.plan,
            commit,
            s.admin_console.acknowledged,
            &operation,
        )
    });
    match result {
        Ok((method, path, body)) => execute(state, method, path, Some(body), false, operation),
        Err(error) => state.update(|s| s.admin_console.error = error),
    }
}
fn select(state: RwSignal<State>, kind: &'static str) {
    if state
        .with_untracked(|s| s.admin_console.busy || !s.admin_console.uncertain_operation.is_empty())
    {
        return;
    }
    state.update(|s| {
        s.admin_console.kind = kind.into();
        s.admin_console.fields.clear();
        s.admin_console.plan = Value::Null;
        s.admin_console.acknowledged = false;
        s.admin_console.error.clear();
        s.admin_console.result.clear();
    });
}
fn field(
    state: RwSignal<State>,
    key: &'static str,
    label: &'static str,
    password: bool,
) -> impl IntoView {
    view! {<label class="zxs-flow"><span>{label}</span><input class="zxs-input" type=if password{"password"}else{"text"} autocomplete=if password{"new-password"}else{"off"} disabled=move ||state.with(|s|s.admin_console.busy) prop:value=move ||state.with(|s|s.admin_console.fields.get(key).cloned().unwrap_or_default()) on:input=move |ev|state.update(|s|{s.admin_console.fields.insert(key.into(),event_target_value(&ev));s.admin_console.acknowledged=false;})/></label>}
}
fn toggle(state: RwSignal<State>, key: &'static str, label: &'static str) -> impl IntoView {
    view! {<label class="zxs-row"><input type="checkbox" disabled=move ||state.with(|s|s.admin_console.busy) prop:checked=move ||state.with(|s|s.admin_console.fields.get(key).is_some_and(|v|v=="true")) on:change=move |ev|state.update(|s|{s.admin_console.fields.insert(key.into(),event_target_checked(&ev).to_string());})/>{label}</label>}
}
fn query(
    state: RwSignal<State>,
    prefix: &'static str,
    key: &'static str,
    suffix: &'static str,
    plan: bool,
) {
    if !allowed(state, false) {
        return;
    }
    let value =
        state.with_untracked(|s| s.admin_console.fields.get(key).cloned().unwrap_or_default());
    if !key.is_empty() && value.trim().is_empty() {
        state.update(|s| s.admin_console.error = format!("{key} is required"));
        return;
    }
    let path = if key.is_empty() {
        prefix.into()
    } else {
        format!("{prefix}/{}{suffix}", api::segment(&value))
    };
    execute(state, "GET", path, None, plan, String::new());
}
pub(crate) fn view(state: RwSignal<State>) -> impl IntoView {
    view! {<section class="zxs-flow" aria-label="Administration">
    <h2>{t("日常管理","Daily administration")}</h2>
    <p>{t("账户由系统盘管理。数据盘不可写时，存储操作禁用。文件内容请使用 SMB。","Accounts are managed on the system disk. Storage changes are disabled unless data storage is writable. Use SMB for file contents.")}</p>
    <nav class="zxs-flow">{[("account","账户 Accounts"),("repository","仓库 Repository"),("grants","访问权限 ACL"),("smb","SMB 绑定 Binding"),("policy","副本策略 Policy"),("rule","主盘规则 Placement"),("branch","历史分支 Branch"),("link","合并连线 Link"),("purge","清理预览 Purge"),("split","拆分预览 Split"),("save","历史保存预览 Historical save"),("inspect","检查 / 查询 Inspect")].into_iter().map(move |(kind,label)|view!{<button class="zxs-button" disabled=move ||state.with(|s|s.admin_console.busy||!s.admin_console.uncertain_operation.is_empty()) on:click=move |_|select(state,kind)>{label}</button>}).collect_view()}</nav>
    <p role="alert">{move ||state.with(|s|s.admin_console.error.clone())}</p>
    <Show when=move ||state.with(|s|!s.admin_console.uncertain_operation.is_empty())><p role="alert">"结果未确认，请先查询操作状态或刷新账户；不要重复提交。 Outcome unknown. Inspect the original operation or refresh accounts before another mutation."</p><p>{move ||state.with(|s|s.admin_console.uncertain_operation.clone())}</p><button class="zxs-button" on:click=move |_|{let id=state.with_untracked(|s|s.admin_console.uncertain_operation.clone());if id=="unknown"{load(state)}else{execute(state,"GET",format!("/api/operations/{}",api::segment(&id)),None,false,String::new())}}>"查询原操作 / Check outcome"</button></Show>
    <Show when=move ||state.with(|s|s.admin_console.kind=="account" || s.admin_console.kind.is_empty())>
        <button class="zxs-button" disabled=move ||state.with(|s|s.admin_console.busy) on:click=move |_|load(state)>"刷新账户 Refresh accounts"</button>
        {move ||state.with(|s|s.admin_console.accounts.clone()).into_iter().map(move |account|{let label=format!("{} · admin={} · disabled={} · repositories={}",account["username"].as_str().unwrap_or(""),account["admin"],account["disabled"],account["repositories"]);view!{<button class="zxs-button" disabled=move ||state.with(|s|s.admin_console.busy||!s.admin_console.uncertain_operation.is_empty()) on:click=move |_|{select(state,"account");state.update(|s|{for key in ["username","admin","disabled"]{s.admin_console.fields.insert(key.into(),account[key].as_str().map(str::to_owned).unwrap_or_else(||account[key].to_string()));}let repos=list(&account,"repositories").iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ");s.admin_console.fields.insert("repositories".into(),repos);});}>{label}</button>}}).collect_view()}
        {field(state,"username","用户名 Username",false)}{field(state,"password","密码 Password (blank preserves existing)",true)}{field(state,"repositories","仓库授权 Repository IDs (comma separated)",false)}{toggle(state,"admin","管理员 Administrator")}{toggle(state,"disabled","禁用 Disabled")}
        <p>"保存会撤销该账户已有会话；最后一个管理员不能被禁用。 Saving revokes existing sessions. The last administrator is protected."</p>
    </Show>
    {move ||{let kind=state.with(|s|s.admin_console.kind.clone()); let fields:Vec<(&'static str,&'static str)>=match kind.as_str(){
        "repository"=>vec![("name","名称 Name"),("slot","主盘槽位 Primary slot"),("copies","副本数 Copies")],
        "grants"=>vec![("repository","仓库 ID"),("read","读取用户 Read principals"),("write","写入用户 Write principals"),("manage","管理用户 Manage principals")],
        "smb"=>vec![("repository","仓库 ID"),("branch","分支 ID"),("generation","Expected history generation")],
        "policy"=>vec![("id","策略 ID"),("repository","仓库 ID"),("prefix","路径前缀 Path prefix"),("copies","副本数 Copies"),("generation","Expected generation (0 for new)")],
        "rule"=>vec![("id","规则 ID"),("repository","仓库 ID"),("prefix","路径前缀 Path prefix"),("slot","主盘槽位 Primary slot"),("generation","Expected generation (0 for new)")],
        "branch"=>vec![("repository","仓库 ID"),("commit","来源提交 Source commit"),("name","名称 Name")],
        "link"=>vec![("history","历史树 History ID"),("source_branch","来源分支 Source branch"),("source_commit","来源提交 Source commit"),("target_branch","目标分支 Target branch"),("target_commit","目标提交 Target commit")],
        "purge"=>vec![("history","历史树 History ID"),("versions","版本 IDs (comma separated)")],
        "split"=>vec![("history","历史树 History ID"),("commit","切分提交 Cut commit ID")],
        "save"=>vec![("history","历史树 History ID"),("commit","来源提交 Source commit"),("name","名称 Name"),("workspace","Existing workspace ID (optional)"),("slot","Primary slot (optional)"),("copies","Copies (optional)")],
        "inspect"=>vec![("repository","仓库 ID"),("id","操作 / 计划 / 策略 / 拓扑 ID")],_=>vec![]};fields.into_iter().map(move |(key,label)|field(state,key,label,false)).collect_view()}}
    <Show when=move ||state.with(|s|s.admin_console.kind=="smb")>{toggle(state,"enabled","启用 SMB Enabled")}</Show>
    <Show when=move ||state.with(|s|s.admin_console.kind!="inspect")><button class="zxs-button" disabled=move ||!allowed(state,state.with(|s|s.admin_console.kind!="account"&&!s.admin_console.kind.is_empty()))||state.with(|s|!s.admin_console.uncertain_operation.is_empty()) on:click=move |_|{if state.with_untracked(|s|s.admin_console.kind.is_empty()){state.update(|s|s.admin_console.kind="account".into());}submit(state)}>"保存配置 / 创建预览 Save configuration / Create preview"</button></Show>
    <Show when=move ||state.with(|s|s.admin_console.kind=="inspect")>{[("Repositories","/api/repositories","","",false),("Disks","/api/admin/disks","","",false),("Integrity check","/api/admin/check","","",false),("Repository / ACL","/api/repositories","repository","",false),("SMB binding","/api/repositories","repository","/smb-binding",false),("Operation","/api/operations","id","",false),("Plan","/api/plans","id","",true),("Branch tree","/api/branches","id","/tree",false),("Branch details","/api/branches","id","",false),("File history","/api/files","id","/history",false),("History graph","/api/history-trees","id","/graph",false),("Policy","/api/policies","id","",false),("Placement rule","/api/primary-placement-rules","id","",false),("Topology","/api/backup-topologies/default","","",false)].into_iter().map(move |(label,prefix,key,suffix,plan)|view!{<button class="zxs-button" disabled=move ||state.with(|s|s.admin_console.busy) on:click=move |_|query(state,prefix,key,suffix,plan)>{label}</button>}).collect_view()}</Show>
    <pre>{move ||state.with(|s|s.admin_console.result.clone())}</pre>
    <Show when=move ||state.with(|s|s.admin_console.plan.is_object())><h3>"计划预览 Plan review"</h3><pre>{move ||state.with(|s|json_codec::to_string_pretty(&s.admin_console.plan).unwrap_or_default())}</pre><label><input type="checkbox" prop:checked=move ||state.with(|s|s.admin_console.acknowledged) disabled=move ||state.with(|s|s.admin_console.busy) on:change=move |ev|state.update(|s|s.admin_console.acknowledged=event_target_checked(&ev))/>{t("我已审阅此计划及永久删除等后果，确认执行所示摘要对应的计划。","I reviewed this plan and its consequences, including permanent deletion where applicable, and confirm this exact digest.")}</label><button class="zxs-button" disabled=move ||!allowed(state,true)||state.with(|s|!s.admin_console.acknowledged||!s.admin_console.uncertain_operation.is_empty()) on:click=move |_|plan_action(state,true)>"提交所示计划 Commit reviewed plan"</button><button class="zxs-button" disabled=move ||!allowed(state,true)||state.with(|s|!s.admin_console.uncertain_operation.is_empty()) on:click=move |_|plan_action(state,false)>"取消计划 Cancel plan"</button></Show>
    </section>}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::management::Storage;
    #[test]
    fn account_administration_survives_data_failure_but_storage_writes_fail_closed() {
        let mut state = State {
            authenticated: true,
            admin: true,
            ..Default::default()
        };
        for storage in [
            Storage::Unknown,
            Storage::ReadOnly,
            Storage::Unavailable,
            Storage::Unconfigured,
            Storage::RecoveryRequired,
        ] {
            state.management.storage = storage;
            assert!(permitted(&state, false));
            assert!(!permitted(&state, true));
        }
        state.management.storage = Storage::Ready;
        assert!(permitted(&state, true));
        state.admin_console.busy = true;
        assert!(!permitted(&state, false));
        state.admin_console.busy = false;
        state.logout_pending = true;
        assert!(!permitted(&state, false));
        state.logout_pending = false;
        state.admin = false;
        assert!(!permitted(&state, false));
    }
}
