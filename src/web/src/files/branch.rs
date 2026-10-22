use super::actions;
use crate::ui::State;
use web_interface::{html, i18n::t, prelude::*};

#[derive(Clone)]
pub(super) enum Target {
    CurrentDirectory,
    Directory { path: String, select: bool },
    CurrentFileVersion,
    HistoryNode(String),
}

impl Target {
    fn submit(self, state: RwSignal<State>, name: String) {
        match self {
            Self::CurrentDirectory => {
                let path = state.with_untracked(|s| s.files.directory.clone());
                actions::directory_branch(state, path, name, false);
            }
            Self::Directory { path, select } => {
                actions::directory_branch(state, path, name, select);
            }
            Self::CurrentFileVersion | Self::HistoryNode(_) => {
                if let Some((version, node)) =
                    state.with_untracked(|s| file_source(&s.files, &self))
                {
                    actions::create_branch(state, version, node, name);
                }
            }
        }
    }
}

fn file_source(model: &super::model::Model, target: &Target) -> Option<(String, Option<String>)> {
    let record = match target {
        Target::CurrentFileVersion => model
            .selected
            .as_ref()
            .filter(|entry| super::model::visible_content(entry))?
            .clone(),
        Target::HistoryNode(id) => super::model::history_nodes_ref(&model.file_history)
            .iter()
            .find(|node| super::model::history_node_id(node) == *id)?
            .clone(),
        _ => return None,
    };
    let version = record["version_id"]
        .as_str()
        .filter(|id| !id.is_empty())?
        .to_owned();
    let node = record["node_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            if matches!(target, Target::CurrentFileVersion) {
                model.file_history["node_id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .map(str::to_owned)
            } else {
                None
            }
        });
    Some((version, node))
}

#[derive(Debug, PartialEq, Eq)]
enum Name {
    Empty,
    Invalid,
    Valid(String),
}

fn branch_name(value: &str) -> Name {
    let controls = value.chars().any(char::is_control);
    let value = value.trim();
    if value.is_empty() {
        Name::Empty
    } else if !controls
        && value.len() <= super::inline::MAX_TEXT_BYTES
        && name_policy::validate_name(value).is_ok()
    {
        Name::Valid(value.to_owned())
    } else {
        Name::Invalid
    }
}

/// The editor owns only local focus and text. Metadata refreshes leave it mounted;
/// navigating to a different disk, folder or file cancels the unfinished name.
pub(super) fn control(
    state: RwSignal<State>,
    label: String,
    target: Target,
    disabled: Signal<bool>,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let value = RwSignal::new(String::new());
    let error = RwSignal::new(false);
    let label = StoredValue::new(label);
    let target = StoredValue::new(target);
    let context = Memo::new(move |_| {
        state.with(|s| {
            (
                s.session_epoch,
                s.page.clone(),
                s.files.slot.clone(),
                s.files.directory.clone(),
                s.files
                    .selected
                    .as_ref()
                    .map(|entry| crate::ui::text(entry, &["file_id", "entry_id", "path"])),
            )
        })
    });
    Effect::new(move |_| {
        let _ = context.get();
        editing.set(false);
        value.set(String::new());
        error.set(false);
    });

    view! {
        <span class="zxs-inline-branch zxs-flow zxs-flow--inline">
            {move || if editing.get() {
                // Each edit receives its own ref so focus also works when a
                // cancelled editor is opened again.
                let input = NodeRef::<html::Input>::new();
                input.on_load(|input| { let _ = input.focus(); });
                view! {
                    <input class="zxs-branch-name zxs-input zxs-input--compact zxs-input--underline" node_ref=input type="text"
                        aria-label=t("分支名称","Branch name")
                        aria-invalid=move || error.get().to_string()
                        title=t("最多 128 个 UTF-8 字节。","Up to 128 UTF-8 bytes.") data-max-bytes="128" autocomplete="off"
                        autocapitalize="none" spellcheck="false"
                        prop:value=move || value.get()
                        disabled=move || disabled.get()
                        on:input=move |event| { value.set(event_target_value(&event)); error.set(false); }
                        on:blur=move |_| { editing.set(false); value.set(String::new()); error.set(false); }
                        on:keydown=move |event: web_sys::KeyboardEvent| {
                            if event.is_composing() { return; }
                            match event.key().as_str() {
                                "Escape" => {
                                    event.prevent_default(); event.stop_propagation();
                                    editing.set(false); value.set(String::new()); error.set(false);
                                }
                                "Enter" => {
                                    event.prevent_default(); event.stop_propagation();
                                    let name = branch_name(&value.get_untracked());
                                    if name == Name::Invalid {
                                        error.set(true);
                                        return;
                                    }
                                    // Capture the value before closing: removing the
                                    // input can fire blur and clear its local text.
                                    editing.set(false); value.set(String::new()); error.set(false);
                                    if !disabled.get_untracked()
                                        && let Name::Valid(name) = name {
                                            target.get_value().submit(state, name);
                                        }
                                }
                                _ => {}
                            }
                        }/>
                }.into_any()
            } else {
                view! {
                    <button class="zxs-button" type="button" disabled=move || disabled.get()
                        on:click=move |_| { value.set(String::new()); error.set(false); editing.set(true); }>
                        {label.get_value()}
                    </button>
                }.into_any()
            }}
            <Show when=move ||error.get()>
                <span class="zxs-branch-error zxs-row" role="status">{t("分支名称无效，最多 128 个 UTF-8 字节。","Invalid branch name. Use at most 128 UTF-8 bytes.")}</span>
            </Show>
        </span>
    }
}

#[cfg(test)]
#[path = "../../../../test/web/unit/files/branch/tests.rs"]
mod tests;
