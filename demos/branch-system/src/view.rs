use super::{
    graph,
    model::{Action, Entry, Id, Workspace},
    scenarios,
};
use std::collections::BTreeSet;
use wasm_bindgen::JsCast;
use xcs_web::{
    components::{Select, SelectOption},
    prelude::*,
};

#[derive(Clone)]
struct State {
    model: Workspace,
    previous: Option<Workspace>,
    undo: Vec<Workspace>,
    redo: Vec<Workspace>,
    scene: usize,
    cursor: usize,
    playing: bool,
    path: String,
    selected: Id,
    message: String,
    error: String,
    log: Vec<String>,
}
impl State {
    fn load(&mut self, scene: usize, cursor: usize) {
        match scenarios::replay(scene, cursor) {
            Ok((model, message, selected, path)) => {
                self.previous = cursor
                    .checked_sub(1)
                    .and_then(|i| scenarios::replay(scene, i).ok())
                    .map(|v| v.0);
                self.model = model;
                self.message = message;
                self.selected = selected;
                self.path = path;
                self.scene = scene;
                self.cursor = cursor;
                self.error.clear();
                self.undo.clear();
                self.redo.clear();
            }
            Err(error) => {
                self.error = error;
                self.playing = false;
            }
        }
    }
    fn perform(&mut self, action: Action) {
        let keep_source = matches!(&action, Action::Note { .. });
        let source = self.selected;
        let previous = self.model.clone();
        match self.model.apply(action) {
            Ok(message) => {
                self.previous = Some(previous.clone());
                self.undo.push(previous);
                if self.undo.len() > 80 {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.error.clear();
                self.message = message.clone();
                self.log.push(message);
                if self.log.len() > 300 {
                    self.log.remove(0);
                }
                self.select_current();
                if keep_source && self.model.files.contains_key(&source) {
                    self.selected = source;
                }
            }
            Err(error) => self.error = error,
        }
    }
    fn select_current(&mut self) {
        if !self.path.is_empty() && self.model.entry(&self.path).is_err() {
            self.path = self
                .model
                .paths()
                .into_iter()
                .find(|(_, e)| !e.folder())
                .map(|v| v.0)
                .unwrap_or_default();
        }
        self.selected = self.model.file_at(&self.path).map(|v| v.2).unwrap_or(0);
    }
}
fn advance(state: RwSignal<State>) {
    state.update(|s| {
        let length = scenarios::all()[s.scene].steps.len();
        if s.cursor < length {
            s.load(s.scene, s.cursor + 1);
        }
        if s.cursor >= length {
            s.playing = false;
        }
    });
}
fn select(
    label: &'static str,
    options: Signal<Vec<SelectOption>>,
    value: RwSignal<String>,
) -> impl IntoView {
    view! { <Select label=label.to_owned() options=options value=Signal::derive(move ||value.get()) on_change=Callback::new(move |v|value.set(v))/> }
}
fn field(label: &'static str, value: RwSignal<String>, placeholder: &'static str) -> impl IntoView {
    view! { <label class="xcs-demo-field"><span class="xcs-row">{label}</span><input class="xcs-input xcs-input--compact xcs-input--underline" aria-label=label placeholder=placeholder prop:value=move ||value.get() on:input=move |ev|value.set(event_target_value(&ev))/></label> }
}
fn command_input(operation: RwSignal<String>, value: RwSignal<String>) -> impl IntoView {
    let label = move || match operation.get().as_str() {
        "write" => "文件正文（留空提交空版本）",
        "create-file" | "create-directory" => "新分支名称（最多 128 字节）",
        "rename" => "新名称（最多 128 字节）",
        "tags" => "完整标签集合（用逗号分隔，留空清除）",
        "note" => "节点备注（最多 128 字节，留空清除）",
        "backup" => "备份前缀（B01 | C01）",
        _ => "输入内容",
    };
    view! {<label class="xcs-demo-field"><span class="xcs-row">{label}</span><input class="xcs-input xcs-input--compact xcs-input--underline" data-xcs-demo-input="value" aria-label=label prop:value=move ||value.get() on:input=move |ev|value.set(event_target_value(&ev))/></label>}
}
fn icon(folder: bool) -> impl IntoView {
    view! { <svg class="xcs-demo-icon" aria-hidden="true" viewBox="0 0 20 20"><path d=if folder {"M2 5h6l2 2h8v9H2Z M2 7h16"}else{"M5 2h6l4 4v12H5Z M11 2v5h4"} fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/></svg> }
}
pub fn app() -> impl IntoView {
    let doc = web_sys::window().and_then(|w| w.document());
    let free = doc
        .and_then(|d| d.document_element())
        .and_then(|e| e.get_attribute("data-xcs-demo"))
        .is_some_and(|v| v == "free");
    let query = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok());
    let mut initial = State {
        model: Workspace::seed(),
        previous: None,
        undo: vec![],
        redo: vec![],
        scene: 0,
        cursor: 0,
        playing: false,
        path: "A/报告.txt".into(),
        selected: 0,
        message: String::new(),
        error: String::new(),
        log: vec![],
    };
    let scene = query
        .as_ref()
        .and_then(|q| q.get("scenario"))
        .and_then(|v| scenarios::all().iter().position(|s| s.id == v))
        .unwrap_or(0);
    let cursor = query
        .as_ref()
        .and_then(|q| q.get("step"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
        .min(scenarios::all()[scene].steps.len());
    initial.load(scene, cursor);
    let state = RwSignal::new(initial);
    let timer = gloo_timers::callback::Interval::new(2400, move || {
        if state.with_untracked(|s| s.playing) {
            advance(state);
        }
    });
    let timer = send_wrapper::SendWrapper::new(timer);
    on_cleanup(move || drop(timer));
    view! {
        <div class="xcs-app-shell xcs-demo" data-demo-mode=if free {"free"} else {"guided"}>
            <header class="xcs-demo-nav xcs-flow"><span class="xcs-row">"XCS"</span><nav class="xcs-flow" aria-label="演示导航">
                <a class="xcs-button" href="./index.html" aria-current=(!free).then_some("page")>"过程演示"</a>
                <a class="xcs-button" href="./playground.html" aria-current=free.then_some("page")>"自由操作"</a>
                <a class="xcs-button" href="./reference/index.html">"静态图示"</a>
            </nav><span class="xcs-row xcs-status">"独立内存演示 · 刷新重置 · 不读写真实文件"</span></header>
            <main class="xcs-demo-main xcs-stack">
                {if free { controls(state).into_any() } else { process(state).into_any() }}
                <div class="xcs-demo-feedback" aria-live="polite"><p class="xcs-row" role="status">{move ||state.with(|s|s.message.clone())}</p><Show when=move ||state.with(|s|!s.error.is_empty())><p class="xcs-row xcs-danger" role="alert">{move ||state.with(|s|s.error.clone())}</p></Show></div>
                {overview(state,free)}
                {trees(state)}
                <Show when=move ||free><details class="xcs-frame xcs-demo-log"><summary class="xcs-row">"操作记录"</summary>{move ||state.with(|s|s.log.clone()).into_iter().rev().map(|item|view!{<p class="xcs-row">{item}</p>}).collect_view()}</details></Show>
                <p class="xcs-row xcs-status">"父节点在下，子节点在上；绿色为本步新增，蓝框为当前选择。目录合并待确认。演示不执行磁盘复制、权限校验或存储保护证明。"</p>
            </main>
        </div>
    }
}
fn process(state: RwSignal<State>) -> impl IntoView {
    let scene_options = Signal::derive(move || {
        scenarios::all()
            .iter()
            .enumerate()
            .map(|(i, s)| SelectOption::new(i.to_string(), s.title))
            .collect()
    });
    let scene_value = Signal::derive(move || state.with(|s| s.scene.to_string()));
    view! { <section class="xcs-stack" aria-label="过程播放控制">
        <div class="xcs-demo-process-toolbar xcs-flow xcs-surface-end"><Select label="演示过程".to_owned() options=scene_options value=scene_value on_change=Callback::new(move |v:String|{if let Ok(index) = v.parse::<usize>() {state.update(|s|{s.playing=false;s.load(index,0);});}})/>
            <div class="xcs-flow"><button class="xcs-button" disabled=move ||state.with(|s|s.cursor == 0) on:click=move |_|state.update(|s|{s.playing=false;s.load(s.scene,s.cursor-1);})>"上一步"</button>
            <button class="xcs-button" on:click=move |_|state.update(|s|{if s.cursor == scenarios::all()[s.scene].steps.len(){s.load(s.scene,0);}s.playing=!s.playing;})>{move ||if state.with(|s|s.playing){"暂停"}else{"播放"}}</button>
            <button class="xcs-button" disabled=move ||state.with(|s|s.cursor == scenarios::all()[s.scene].steps.len()) on:click=move |_|{state.update(|s|s.playing=false);advance(state);}>"下一步"</button>
            <button class="xcs-button" on:click=move |_|state.update(|s|{s.playing=false;s.load(s.scene,0);})>"重播"</button></div>
            <a class="xcs-button" href=move ||state.with(|s|format!("./playground.html?scenario={}&step={}",scenarios::all()[s.scene].id,s.cursor))>"从这一步自行操作 →"</a>
        </div>
        <div class="xcs-demo-steps xcs-frame" role="group" aria-label="过程步骤">
            <button class="xcs-button" aria-pressed=move ||state.with(|s|s.cursor == 0).to_string() on:click=move |_|state.update(|s|{s.playing=false;s.load(s.scene,0);})>"0 · 初始状态"</button>
            {move ||{let index=state.with(|s|s.scene);scenarios::all()[index].steps.iter().enumerate().map(|(i,step)|view!{<button class="xcs-button" aria-pressed=move ||state.with(|s|s.cursor == i+1).to_string() on:click=move |_|state.update(|s|{s.playing=false;s.load(index,i+1);})>{format!("{} · {}",i+1,step.title)}</button>}).collect_view()}}
        </div>
        <p class="xcs-row" data-demo-step="description">{move ||state.with(|s|{let scenes=scenarios::all();if s.cursor == 0 {format!("0 / {} · 从下方的父节点开始，点击下一步观察变化",scenes[s.scene].steps.len())}else{let step=&scenes[s.scene].steps[s.cursor-1];format!("{} / {} · {}",s.cursor,scenes[s.scene].steps.len(),step.detail)}})}</p>
    </section> }
}
fn controls(state: RwSignal<State>) -> impl IntoView {
    let operation = RwSignal::new("write".to_owned());
    let value = RwSignal::new(String::new());
    let branch = RwSignal::new(String::new());
    let target = RwSignal::new("B".to_owned());
    let owner = RwSignal::new(String::new());
    let merge_inputs = RwSignal::new(BTreeSet::<String>::new());
    let branch_options = Signal::derive(move || {
        state.with(|s| {
            if let Ok((file, _, _)) = s.model.file_at(&s.path) {
                s.model
                    .file_branches
                    .iter()
                    .filter(|((id, _), _)| *id == file)
                    .map(|((_, n), b)| {
                        SelectOption::new(
                            n,
                            format!(
                                "{n}{}",
                                if b.alias.is_some() {
                                    "（只读别名）"
                                } else if b.pending {
                                    "（待首写）"
                                } else {
                                    ""
                                }
                            ),
                        )
                    })
                    .collect()
            } else if let Ok(ctx) = s.model.context(&s.path) {
                s.model
                    .directory_branches
                    .keys()
                    .filter(|(id, _)| *id == ctx.directory)
                    .map(|(_, n)| SelectOption::new(n, n))
                    .collect()
            } else {
                vec![]
            }
        })
    });
    let node_options = Signal::derive(move || {
        state.with(|s| {
            s.model
                .file_at(&s.path)
                .ok()
                .map(|(file, _, _)| {
                    s.model
                        .files
                        .values()
                        .filter(|n| n.file == file)
                        .rev()
                        .map(|n| {
                            SelectOption::new(
                                n.id.to_string(),
                                format!(
                                    "F{} · {} · {}",
                                    n.id,
                                    n.kind.label(),
                                    s.model.content(n.id)
                                ),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    });
    let op_options = Signal::derive(|| {
        [
            ("write", "写入文件"),
            ("create-file", "登记文件分支"),
            ("switch-file", "切换文件分支"),
            ("create-directory", "登记目录分支"),
            ("switch-directory", "切换目录分支"),
            ("rename", "重命名"),
            ("move", "移动"),
            ("copy", "复制"),
            ("delete", "删除引用"),
            ("tags", "修改节点标签"),
            ("note", "编辑节点备注"),
            ("merge", "合并文件分支"),
            ("default", "设置 SMB 默认组合"),
            ("backup", "设置备份前缀"),
        ]
        .into_iter()
        .map(|(v, n)| SelectOption::new(v, n))
        .collect()
    });
    let node_value = Signal::derive(move || state.with(|s| s.selected.to_string()));
    let root_options = Signal::derive(move || {
        state.with(|s| {
            s.model
                .root_branches()
                .iter()
                .map(|n| SelectOption::new(n, n))
                .collect()
        })
    });
    let root_value = Signal::derive(move || state.with(|s| s.model.active.clone()));
    view! { <section class="xcs-stack" aria-label="自由操作控制">
        <div class="xcs-demo-controls xcs-surface-end">
            <Select label="当前根目录分支".to_owned() options=root_options value=root_value on_change=Callback::new(move |name|state.update(|s|s.perform(Action::SwitchDirectoryBranch{path:String::new(),name})))/>
            {select("操作",op_options,operation)}
            <div class="xcs-demo-command-buttons xcs-flow">
                <button class="xcs-button" disabled=move ||state.with(|s|s.undo.is_empty()) on:click=move |_|state.update(|s|{if let Some(previous)=s.undo.pop(){s.redo.push(s.model.clone());s.previous=Some(s.model.clone());s.model=previous;s.select_current();s.message="已撤销上一步".into();s.error.clear();}})>"撤销"</button>
                <button class="xcs-button" disabled=move ||state.with(|s|s.redo.is_empty()) on:click=move |_|state.update(|s|{if let Some(next)=s.redo.pop(){s.undo.push(s.model.clone());s.previous=Some(s.model.clone());s.model=next;s.select_current();s.message="已重做".into();s.error.clear();}})>"重做"</button>
                <button class="xcs-button" on:click=move |_|{state.update(|s|{s.load(0,0);s.log.clear();});value.set(String::new());branch.set(String::new());owner.set(String::new());merge_inputs.set(BTreeSet::new());}>"重置演示"</button>
            </div>
        </div>
        <form class="xcs-frame xcs-demo-operation" on:submit=move |event: web_sys::SubmitEvent|{
            event.prevent_default();
            let op=operation.get_untracked();let text=value.get_untracked();let b=branch.get_untracked();let dest=target.get_untracked();
            state.update(|s|{
                let path=s.path.clone();
                let action=match op.as_str(){
                    "write"=>Action::Write{path,content:text},"create-file"=>Action::CreateFileBranch{path,name:text},"switch-file"=>Action::SwitchFileBranch{path,name:b},
                    "create-directory"=>Action::CreateDirectoryBranch{path,name:text},"switch-directory"=>Action::SwitchDirectoryBranch{path,name:b},
                    "rename"=>Action::Rename{path,name:text},"move"=>Action::Move{path,target:dest},"copy"=>Action::Copy{path,target:dest},"delete"=>Action::Delete{path},
                    "tags"=>Action::Tags{path,source:s.selected,tags:text.split([',','，']).map(str::trim).filter(|v|!v.is_empty()).map(str::to_owned).collect()},
                    "note"=>Action::Note{path,source:s.selected,text},
                    "merge"=>Action::Merge{path,branches:merge_inputs.get_untracked().into_iter().collect(),chosen:s.selected,owner:owner.get_untracked()},
                    "default"=>Action::Default{name:b},"backup"=>Action::Protect{path,copies:3,prefixes:text.split('|').map(str::trim).map(str::to_owned).collect()},
                    _=>{s.error="请选择操作".into();return;}
                };s.perform(action);
            });
        }>
            <p class="xcs-row">{move ||state.with(|s|format!("操作对象：{}",if s.path.is_empty(){"P01（根目录）"}else{&s.path}))}</p>
            <Show when=move ||matches!(operation.get().as_str(),"write"|"create-file"|"create-directory"|"rename"|"tags"|"note"|"backup")>
                {command_input(operation,value)}
            </Show>
            <Show when=move ||matches!(operation.get().as_str(),"switch-file"|"switch-directory")>{select("目标分支",branch_options,branch)}</Show>
            <Show when=move ||operation.get() == "default">{select("SMB 默认根目录分支",root_options,branch)}</Show>
            <Show when=move ||matches!(operation.get().as_str(),"move"|"copy")>{field("目标目录",target,"例如 B；留空表示根目录")}</Show>
            <Show when=move ||matches!(operation.get().as_str(),"tags"|"merge"|"note")>
                <Select label="来源 / 合并结果节点".to_owned() options=node_options value=node_value on_change=Callback::new(move |v:String|if let Ok(node)=v.parse(){state.update(|s|s.selected=node);})/>
            </Show>
            <Show when=move ||operation.get() == "merge">
                <fieldset class="xcs-demo-merge-choices"><legend class="xcs-row">"参与合并的分支"</legend>{move ||state.with(|s|s.model.file_at(&s.path).ok().map(|(file,_,_)|s.model.file_branches.iter().filter(|((f,_),b)|*f == file && b.alias.is_none()).map(|((_,n),_)|n.clone()).collect::<Vec<_>>()).unwrap_or_default()).into_iter().map(move |name|{let selected=name.clone();let changed=name.clone();view!{<label class="xcs-row"><input type="checkbox" prop:checked=move ||merge_inputs.with(|v|v.contains(&selected)) on:change=move |ev|merge_inputs.update(|v|{if event_target_checked(&ev){v.insert(changed.clone());}else{v.remove(&changed);}})/>{name}</label>}}).collect_view()}</fieldset>
                {select("合并后归属分支",Signal::derive(move ||merge_inputs.get().iter().map(|n|SelectOption::new(n,n)).collect()),owner)}
            </Show>
            <Show when=move ||operation.get() == "delete"><p class="xcs-row">"移除当前组成的引用，历史和其他分支保留；可撤销。"</p></Show>
            <Show when=move ||operation.get() == "backup"><p class="xcs-row">"演示配置三份：P01 + B01 + C01；只演示逻辑路径映射。"</p></Show>
            <button class="xcs-button" type="submit">{move ||if operation.get() == "merge" {"确认合并并执行"}else{"执行操作"}}</button>
        </form>
    </section> }
}
fn overview(state: RwSignal<State>, free: bool) -> impl IntoView {
    view! { <section class="xcs-stack" aria-label="当前可见文件与统计">
        <div class="xcs-flow"><p class="xcs-row" data-demo-count="bodies">{move ||state.with(|s|format!("正文 {} 份",s.model.bodies.len()))}</p><p class="xcs-row" data-demo-count="files">{move ||state.with(|s|format!("文件节点 {} 个",s.model.files.len()))}</p><p class="xcs-row" data-demo-count="directories">{move ||state.with(|s|format!("目录子树 {} 个",s.model.directories.len()))}</p><p class="xcs-row">{move ||state.with(|s|format!("当前组合 {} · SMB 默认 {}",s.model.active,s.model.default_branch))}</p></div>
        <div class="xcs-frame xcs-demo-file-list"><div class="xcs-demo-file-row xcs-demo-file-heading"><span class="xcs-row">"名称 / 路径"</span><span class="xcs-row">"所引用分支及节点"</span><span class="xcs-row">"当前读取内容"</span></div>
            <button class="xcs-demo-file-row" data-demo-path="" aria-pressed=move ||state.with(|s|s.path.is_empty()).to_string() on:click=move |_|state.update(|s|{s.path.clear();s.selected=0;})><span class="xcs-row">{icon(true)}"P01（根目录）"</span><span class="xcs-row">{move ||state.with(|s|format!("{} → D{}",s.model.active,s.model.context("").map(|v|v.node).unwrap_or(0)))}</span><span class="xcs-row">"目录组成"</span></button>
            {move ||state.with(|s|s.model.paths()).into_iter().map(move |(path,entry)|{
                let chosen=path.clone();let object=path.clone();let folder=entry.folder();let node=entry.node();
                let label=match &entry{Entry::File{branch,lazy,..}|Entry::Folder{branch,lazy,..}=>format!("{branch} → {}{node}{}",if folder{"D"}else{"F"},lazy.as_ref().map(|n|format!(" · {n} 待首写")).unwrap_or_default())};
                let content=if folder{"目录组成".to_owned()}else{state.with_untracked(|s|s.model.content(node).to_owned())};
                view!{<button class="xcs-demo-file-row" data-demo-path=path.clone() aria-pressed=move ||state.with(|s|s.path == chosen).to_string() on:click=move |_|state.update(|s|{s.path=object.clone();s.selected=if folder{0}else{node};})><span class="xcs-row">{icon(folder)}{path.clone()}</span><span class="xcs-row">{label}</span><span class="xcs-row">{content}</span></button>}
            }).collect_view()}
        </div>
        <Show when=move ||free><p class="xcs-row xcs-status">"点击列表选择对象；目录操作选择文件夹或根目录。点击文件树节点可选择历史来源。"</p></Show>
        {move ||state.with(|s|s.model.protection.clone()).into_iter().map(|p|view!{<div class="xcs-frame"><p class="xcs-row">{format!("{} · {} 份（前缀固定，后续主盘改名不改写它）",p.path,p.copies)}</p>{p.prefixes.iter().enumerate().map(|(i,prefix)|view!{<p class="xcs-row">{format!("{} → {prefix}/〈相对于策略目录的路径〉",if i == 0{"B01"}else{"C01"})}</p>}).collect_view()}</div>}).collect_view()}
    </section> }
}
fn trees(state: RwSignal<State>) -> impl IntoView {
    let directory_path = RwSignal::new(String::new());
    let file_options = Signal::derive(move || {
        state.with(|s| {
            s.model
                .paths()
                .iter()
                .filter(|(_, e)| !e.folder())
                .map(|(p, _)| SelectOption::new(p, p))
                .collect()
        })
    });
    let file_value = Signal::derive(move || state.with(|s| s.path.clone()));
    let directory_options = Signal::derive(move || {
        state.with(|s| {
            std::iter::once(SelectOption::new("", "P01（根目录）"))
                .chain(
                    s.model
                        .paths()
                        .into_iter()
                        .filter(|(_, e)| e.folder())
                        .map(|(p, _)| SelectOption::new(&p, &p)),
                )
                .collect()
        })
    });
    view! { <div class="xcs-demo-trees">
        <section class="xcs-stack"><div class="xcs-surface-end"><Select label="文件节点树 · 正文与元数据".to_owned() options=file_options value=file_value on_change=Callback::new(move |path:String|state.update(|s|{s.selected=s.model.file_at(&path).map(|v|v.2).unwrap_or(0);s.path=path;}))/></div>
            <div class="xcs-frame xcs-demo-canvas" on:click=move |ev:web_sys::MouseEvent|{if let Some(element)=ev.target().and_then(|v|v.dyn_into::<web_sys::Element>().ok()).and_then(|e|e.closest("[data-demo-kind='F']").ok().flatten()){if let Some(id)=element.get_attribute("data-demo-node").and_then(|v|v.parse().ok()){state.update(|s|s.selected=id);}}}
                inner_html=move ||state.with(|s|s.model.file_at(&s.path).ok().map(|(file,_,_)|graph::file(&s.model,file,s.selected,s.previous.as_ref()).html).unwrap_or_else(||"<p class='xcs-row'>选择一个文件查看节点树</p>".into()))>
            </div>
            <p class="xcs-row" data-demo-node="selected-detail">{move ||state.with(|s|s.model.files.get(&s.selected).map(|n|format!("F{} · {} · {} · 标签：{}",n.id,n.path,n.note,n.tags.iter().cloned().collect::<Vec<_>>().join(" / "))).unwrap_or_default())}</p>
        </section>
        <section class="xcs-stack"><div class="xcs-surface-end">{select("目录子树 · 固定组成",directory_options,directory_path)}</div>
            <div class="xcs-frame xcs-demo-canvas" inner_html=move ||state.with(|s|{
                let path=directory_path.get();let ctx=s.model.context(&path).or_else(|_|s.model.context(""));
                ctx.ok().map(|ctx|graph::directory(&s.model,ctx.directory,ctx.node,s.previous.as_ref()).html).unwrap_or_default()
            })></div>
        </section>
    </div> }
}
