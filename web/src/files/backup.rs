//! Per-scope protection and independent logical prefixes on backup disks.
use super::{actions, model::file_context, review};
use crate::{
    api, format,
    ui::{State, current_session, fail, text},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use zxs_web::components::{LoadingState, Select, SelectOption};
use zxs_web::tasks::spawn_local;
use zxs_web::{i18n::t, prelude::*};

#[derive(Clone)]
pub(crate) struct Draft {
    id: String,
    repository: String,
    path: String,
    configuration: Value,
    generation: String,
    scope_id: String,
    target: u32,
    prefixes: BTreeMap<String, String>,
    loading: bool,
    can_manage: bool,
}
pub(crate) fn start_entry(state: RwSignal<State>, entry: Value) {
    let Some((repository, path)) = state.with_untracked(|s| file_context(&s.files, &entry)) else {
        return;
    };
    start(state, repository, path);
}
pub(crate) fn start_current(state: RwSignal<State>) {
    let (repository, path) = state.with_untracked(|s| {
        (
            s.files.repository_id.clone(),
            s.files.repository_path.clone(),
        )
    });
    if !repository.is_empty() {
        start(state, repository, path);
    }
}
fn start(state: RwSignal<State>, repository: String, path: String) {
    if state.with_untracked(actions::pending) {
        return;
    }
    let id = api::operation_id();
    if id.is_empty() {
        return;
    }
    let epoch = state.with_untracked(|s| s.session_epoch);
    review::reset(state, review::Kind::Backup);
    state.update(|s| {
        s.files.backup = Some(Draft {
            id: id.clone(),
            repository: repository.clone(),
            path: path.clone(),
            configuration: Value::Null,
            generation: String::new(),
            scope_id: String::new(),
            target: 0,
            prefixes: BTreeMap::new(),
            loading: true,
            can_manage: false,
        })
    });
    spawn_local(async move {
        let result = api::request(
            "GET",
            &format!(
                "/api/repositories/{}/backup-configuration?path={}",
                api::segment(&repository),
                api::segment(&path)
            ),
            None,
        )
        .await;
        if !current_session(state, epoch)
            || !state
                .with_untracked(|s| s.files.backup.as_ref().is_some_and(|draft| draft.id == id))
        {
            return;
        }
        match result {
            Ok(configuration) => state.update(|s| {
                if let Some(draft) = s.files.backup.as_mut() {
                    draft.target = format::parse_u64(&configuration["target_copies"])
                        .and_then(|n| u32::try_from(n).ok())
                        .unwrap_or(0);
                    draft.generation = text(&configuration, &["generation"]);
                    draft.scope_id = text(&configuration["scope"], &["id"]);
                    draft.prefixes = configuration["backup_prefixes"]
                        .as_object()
                        .into_iter()
                        .flatten()
                        .filter_map(|(slot, prefix)| {
                            prefix
                                .as_str()
                                .map(|prefix| (slot.clone(), prefix.to_owned()))
                        })
                        .collect();
                    draft.can_manage = configuration["can_manage"] == true;
                    draft.configuration = configuration;
                    draft.loading = false;
                }
            }),
            Err(error) => {
                state.update(|s| {
                    if let Some(draft) = s.files.backup.as_mut() {
                        draft.loading = false;
                    }
                });
                review::error(state, review::Kind::Backup, error.to_string());
                fail(state, error);
            }
        }
    });
}
fn routes(configuration: &Value) -> Vec<Vec<String>> {
    let parse = |value: &Value| {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    if let Some(chains) = configuration["backup_routes"]
        .as_array()
        .filter(|chains| !chains.is_empty())
    {
        return chains.iter().map(parse).collect();
    }
    let route = parse(
        configuration
            .get("available_route_slot_ids")
            .or_else(|| configuration.get("route_slot_ids"))
            .unwrap_or(&Value::Null),
    );
    if route.is_empty() {
        vec![]
    } else {
        vec![route]
    }
}
fn max_copies(configuration: &Value) -> usize {
    let available = routes(configuration)
        .iter()
        .map(Vec::len)
        .min()
        .unwrap_or(0);
    crate::format::parse_u64(&configuration["max_copies"])
        .and_then(|n| usize::try_from(n).ok())
        .map(|n| n.min(available))
        .unwrap_or(available)
}
fn target_slots(configuration: &Value, target: u32) -> Vec<String> {
    routes(configuration)
        .into_iter()
        .flat_map(|route| {
            route
                .into_iter()
                .skip(1)
                .take(target.saturating_sub(1) as usize)
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn valid_prefix(prefix: &str) -> bool {
    prefix.len() <= 768 && (prefix.is_empty() || zxs_naming::validate_path(prefix).is_ok())
}
fn request(draft: &Draft) -> Result<Value, String> {
    if draft.loading || !draft.can_manage {
        return Err(t(
            "当前无权修改该备份策略。",
            "You cannot change this backup configuration.",
        ));
    }
    if draft.scope_id.is_empty() || draft.generation.parse::<u64>().is_err() {
        return Err(t(
            "保护策略依据缺失，请关闭后重新读取。",
            "The policy baseline is missing. Close and reload.",
        ));
    }
    if draft.target == 0 || draft.target as usize > max_copies(&draft.configuration) {
        return Err(t(
            "所选份数超出这条备份路径的能力。",
            "The copy count exceeds this backup route’s capacity.",
        ));
    }
    let mut prefixes = BTreeMap::new();
    for slot in target_slots(&draft.configuration, draft.target) {
        let prefix = draft.prefixes.get(&slot).cloned().unwrap_or_default();
        if !valid_prefix(&prefix) {
            return Err(t(
                "备份前缀必须是相对路径，最多768个UTF-8字节，可留空。",
                "Backup prefixes must be relative paths of at most 768 UTF-8 bytes, or empty.",
            ));
        }
        prefixes.insert(slot, prefix);
    }
    Ok(
        json!({"path":draft.path,"target_copies":draft.target,"backup_prefixes":prefixes,"expected_generation":draft.generation,"expected_scope_id":draft.scope_id}),
    )
}
fn submit(state: RwSignal<State>) {
    let Some(draft) = state.with_untracked(|s| s.files.backup.clone()) else {
        return;
    };
    match request(&draft) {
        Ok(body) => review::submit(
            state,
            review::Kind::Backup,
            "PUT",
            format!(
                "/api/repositories/{}/backup-configuration",
                api::segment(&draft.repository)
            ),
            body,
        ),
        Err(error) => review::error(state, review::Kind::Backup, error),
    }
}
fn backup_paths(configuration: &Value) -> Vec<(String, String)> {
    if let Some(paths) = configuration["backup_paths"].as_object() {
        return paths
            .iter()
            .filter_map(|(slot, path)| path.as_str().map(|path| (slot.clone(), path.to_owned())))
            .collect();
    }
    configuration["backup_paths"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|value| {
            (
                text(value, &["slot_id", "slot"]),
                text(value, &["logical_path", "path"]),
            )
        })
        .filter(|(slot, path)| !slot.is_empty() && !path.is_empty())
        .collect()
}
pub(crate) fn view(state: RwSignal<State>) -> impl IntoView {
    let open = Memo::new(move |_| state.with(|s| s.files.backup.is_some()));
    let disabled = Signal::derive(move || {
        state.with(|s| {
            actions::pending(s)
                || !s.files.writable
                || s.files
                    .backup
                    .as_ref()
                    .is_none_or(|draft| draft.loading || !draft.can_manage)
        })
    });
    let options = Signal::derive(move || {
        state.with(|s| {
            s.files
                .backup
                .as_ref()
                .map(|draft| {
                    (1..=max_copies(&draft.configuration))
                        .map(|n| {
                            SelectOption::new(
                                n.to_string(),
                                format!("{n} {}", t("份（含主盘）", "copies (including primary)")),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    });
    let target = Signal::derive(move || {
        state.with(|s| {
            s.files
                .backup
                .as_ref()
                .map(|draft| draft.target.to_string())
                .unwrap_or_default()
        })
    });
    let slots = Memo::new(move |_| {
        state.with(|s| {
            s.files
                .backup
                .as_ref()
                .map(|draft| target_slots(&draft.configuration, draft.target))
                .unwrap_or_default()
        })
    });
    let paths = Memo::new(move |_| {
        state.with(|s| {
            s.files
                .backup
                .as_ref()
                .map(|draft| backup_paths(&draft.configuration))
                .unwrap_or_default()
        })
    });
    view! {<Show when=move ||open.get()><section class="zxs-frame zxs-stack zxs-backup-review" aria-label=t("备份保护设置","Backup protection settings")>
        <div class="zxs-file-toolbar zxs-flow"><h3 class="zxs-row">{t("备份保护设置","Backup protection settings")}</h3><button class="zxs-button" disabled=move ||state.with(actions::pending) on:click=move |_|{state.update(|s|s.files.backup=None);review::reset(state,review::Kind::Backup);}>{t("关闭","Close")}</button></div>
        <p class="zxs-row">{move ||state.with(|s|s.files.backup.as_ref().map(|draft|if draft.path.is_empty(){"/".into()}else{draft.path.clone()}).unwrap_or_default())}</p>
        <Show when=move ||state.with(|s|s.files.backup.as_ref().is_some_and(|draft|draft.loading))><LoadingState/></Show>
        <Show when=move ||state.with(|s|s.files.backup.as_ref().is_some_and(|draft|!draft.loading&&!draft.configuration.is_null()))>
            <p class="zxs-row">{t("为每个备份硬盘分别设置逻辑前缀；文件使用前缀加文件名，文件夹使用前缀加目录内相对路径。主盘路径保持原样。","Set a separate logical prefix for each backup disk. Files append their name; folders append paths relative to the selected folder. Primary paths stay unchanged.")}</p>
            <Show when=move ||state.with(|s|s.files.backup.as_ref().is_some_and(|draft|draft.configuration["inherited"]==true))><p class="zxs-row">{t("当前继承上级规则；保存后将为此范围单独设置。","This scope currently inherits its parent’s rule. Saving creates a rule for this scope.")}</p></Show>
            <div class="zxs-file-toolbar zxs-flow"><Select label=t("目标份数","Target copy count") options=options value=target disabled=disabled on_change=Callback::new(move |value:String|state.update(|s|if let Some(draft)=s.files.backup.as_mut(){draft.target=value.parse().unwrap_or(0);}))/></div>
            {move ||slots.get().into_iter().map(move |slot|{let read=slot.clone();let write=slot.clone();let label=format!("{slot} {}",t("备份前缀","backup prefix"));view!{<label class="zxs-file-toolbar zxs-flow"><span class="zxs-row">{label.clone()}</span><input class="zxs-input zxs-input--compact zxs-input--underline" aria-label=label data-max-bytes="768" disabled=move ||disabled.get() prop:value=move ||state.with(|s|s.files.backup.as_ref().and_then(|draft|draft.prefixes.get(&read)).cloned().unwrap_or_default()) on:input=move |event|state.update(|s|if let Some(draft)=s.files.backup.as_mut(){draft.prefixes.insert(write.clone(),event_target_value(&event));})/></label>}}).collect_view()}
            <div class="zxs-stack">{move ||paths.get().into_iter().map(|(slot,path)|view!{<p class="zxs-row">{format!("{slot} / {path}")}</p>}).collect_view()}</div>
            <Show when=move ||state.with(|s|s.files.backup.as_ref().is_some_and(|draft|!draft.can_manage))><p class="zxs-row">{t("此备份策略仅可查看。","This backup configuration is read only.")}</p></Show>
            <button class="zxs-button" disabled=move ||disabled.get() on:click=move |_|submit(state)>{t("保存备份设置","Save backup settings")}</button>
        </Show>
        <p class="zxs-row zxs-inline-error" role="status">{move ||state.with(|s|review::error_text(s,review::Kind::Backup))}</p>
    </section></Show>}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multi_primary_routes_use_each_selected_prefix_and_the_shortest_capacity() {
        let configuration = json!({"route_slot_ids":["P01","B01","C01"],"backup_routes":[["P01","B01","C01"],["P02","B02"]],"backup_slot_ids":["B01","C01","B02"],"max_copies":2});
        assert_eq!(max_copies(&configuration), 2);
        assert_eq!(target_slots(&configuration, 2), vec!["B01", "B02"]);
        assert_eq!(target_slots(&configuration, 1), Vec::<String>::new());
        let draft = Draft {
            id: "draft".into(),
            repository: "repo".into(),
            path: "docs".into(),
            configuration,
            generation: "0".into(),
            scope_id: "scope".into(),
            target: 2,
            prefixes: BTreeMap::from([
                ("B01".into(), "first-chain".into()),
                ("B02".into(), "other-chain".into()),
            ]),
            loading: false,
            can_manage: true,
        };
        assert_eq!(
            request(&draft).unwrap()["backup_prefixes"],
            json!({"B01":"first-chain","B02":"other-chain"})
        );
        let mut invalid = draft;
        invalid.target = 3;
        assert!(request(&invalid).is_err());
        assert_eq!(
            target_slots(&json!({"available_route_slot_ids":["P01","B01"]}), 2),
            vec!["B01"]
        );
        assert_eq!(
            max_copies(
                &json!({"route_slot_ids":["P01","B01"],"available_route_slot_ids":["P01","B01","C01"]})
            ),
            3,
            "legacy available route describes capacity rather than the currently configured copy count"
        );
    }
    #[test]
    fn independent_prefixes_are_relative_to_a_stable_scope_and_route() {
        let draft = Draft {
            id: "draft".into(),
            repository: "repo".into(),
            path: "docs".into(),
            configuration: json!({"route_slot_ids":["P01","B01","C01"]}),
            generation: "0".into(),
            scope_id: "stable-directory".into(),
            target: 3,
            prefixes: BTreeMap::from([
                ("B01".into(), "我的备份".into()),
                ("C01".into(), "长期保存".into()),
            ]),
            loading: false,
            can_manage: true,
        };
        let body = request(&draft).unwrap();
        assert_eq!(body["backup_prefixes"]["B01"], "我的备份");
        assert_eq!(body["backup_prefixes"]["C01"], "长期保存");
        assert_eq!(body["expected_scope_id"], "stable-directory");
        assert_eq!(body["expected_generation"], "0");
        assert!(valid_prefix(&"a".repeat(200)));
        assert!(!valid_prefix("../escape"));
        assert!(!valid_prefix("/absolute"));
        let mut invalid = draft;
        invalid.target = 4;
        assert!(request(&invalid).is_err());
    }
}
