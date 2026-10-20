//! Single-line metadata edits share the branch editor's commit/cancel behavior.
use super::{actions, model::NamespaceAction};
use crate::{
    icons::FileIcon,
    ui::{State, list, text},
};
use serde_json::Value;
use zxs_web::{html, i18n::t, prelude::*};

// The UI applies the shared metadata byte bound to newly entered names too.
pub(super) const MAX_TEXT_BYTES: usize = zxs_contract::MAX_NOTE_BYTES;

#[derive(Clone)]
pub(super) enum Target {
    CreateDirectory,
    RenameEntry(Value),
    RenameDirectoryBranch {
        repository: String,
        branch_id: String,
        name: String,
    },
    RenameFileBranch {
        file: String,
        branch_id: String,
        name: String,
    },
    FileNote,
    VersionNote(String),
    NodeNote(String),
}

impl Target {
    fn is_note(&self) -> bool {
        matches!(
            self,
            Self::FileNote | Self::VersionNote(_) | Self::NodeNote(_)
        )
    }

    fn initial(&self, state: &State) -> String {
        match self {
            Self::CreateDirectory => String::new(),
            Self::RenameEntry(entry) => text(entry, &["name"]),
            Self::RenameDirectoryBranch { name, .. } | Self::RenameFileBranch { name, .. } => {
                name.clone()
            }
            Self::FileNote => state.files.file_history["note"]
                .as_str()
                .or_else(|| {
                    state
                        .files
                        .selected
                        .as_ref()
                        .and_then(|entry| entry["note"].as_str())
                })
                .unwrap_or_default()
                .to_owned(),
            Self::NodeNote(id) => super::model::history_nodes(&state.files.file_history)
                .iter()
                .find(|node| super::model::history_node_id(node) == *id)
                .map(|node| text(node, &["note"]))
                .unwrap_or_default(),
            Self::VersionNote(id) => list(&state.files.file_history, "versions")
                .iter()
                .find(|version| text(version, &["version_id"]) == *id)
                .map(|version| text(version, &["note"]))
                .unwrap_or_default(),
        }
    }

    fn display(&self, state: &State) -> String {
        let value = self.initial(state);
        if !value.is_empty() {
            return value;
        }
        if let Self::NodeNote(id) = self {
            if let Some(node) = super::model::history_nodes(&state.files.file_history)
                .iter()
                .find(|node| super::model::history_node_id(node) == *id)
            {
                match node["kind"].as_str().unwrap_or_default() {
                    "tags" => return t("标签变化", "Tags changed"),
                    "merge" => return t("合并分支", "Merged branches"),
                    _ => {}
                }
                let previous = text(node, &["previous_path"]);
                let path = text(node, &["path"]);
                let action = if previous.rsplit_once('/').map(|(parent, _)| parent)
                    == path.rsplit_once('/').map(|(parent, _)| parent)
                {
                    t("改名", "Renamed")
                } else {
                    t("路径变化", "Path changed")
                };
                return format!("{action} {previous} → {path}");
            }
        }
        t("无备注", "No note")
    }

    fn submit(self, state: RwSignal<State>, value: String) {
        match self {
            Self::CreateDirectory => {
                actions::namespace(state, NamespaceAction::CreateDirectory, value)
            }
            Self::RenameEntry(entry) => {
                actions::namespace(state, NamespaceAction::Rename(entry), value)
            }
            Self::RenameDirectoryBranch {
                repository,
                branch_id,
                ..
            } => actions::rename_directory_branch(state, repository, branch_id, value),
            Self::RenameFileBranch {
                file, branch_id, ..
            } => actions::rename_file_branch(state, file, branch_id, value),
            Self::FileNote => actions::update_note(state, None, value),
            Self::VersionNote(id) => actions::update_note(state, Some(id), value),
            Self::NodeNote(id) => actions::update_node_note(state, id, value),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Edit {
    Cancel,
    Invalid,
    Save(String),
}

pub(super) fn valid_note(value: &str) -> bool {
    zxs_contract::validate_note(value).is_ok()
}

fn edit_value(value: &str, note: bool) -> Edit {
    if note {
        if valid_note(value) {
            Edit::Save(value.to_owned())
        } else {
            Edit::Invalid
        }
    } else {
        let name = value.trim();
        if name.is_empty() {
            Edit::Cancel
        } else if !value.chars().any(char::is_control)
            && name.len() <= MAX_TEXT_BYTES
            && super::model::valid_name(name)
        {
            Edit::Save(name.to_owned())
        } else {
            Edit::Invalid
        }
    }
}

/// The same editor state is used by toolbar edits and by the two separate rows
/// of a version node. Persisted metadata is only read when editing starts.
#[derive(Clone, Copy)]
struct Editor {
    state: RwSignal<State>,
    editing: RwSignal<bool>,
    value: RwSignal<String>,
    error: RwSignal<bool>,
    target: StoredValue<Target>,
    label: StoredValue<String>,
    disabled: Signal<bool>,
    note: bool,
}

impl Editor {
    fn new(state: RwSignal<State>, label: String, target: Target, disabled: Signal<bool>) -> Self {
        let editor = Self {
            state,
            editing: RwSignal::new(false),
            value: RwSignal::new(String::new()),
            error: RwSignal::new(false),
            note: target.is_note(),
            target: StoredValue::new(target),
            label: StoredValue::new(label),
            disabled,
        };
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
                        .map(|entry| text(entry, &["file_id", "entry_id", "path"])),
                )
            })
        });
        Effect::new(move |_| {
            let _ = context.get();
            editor.cancel();
        });
        editor
    }

    fn cancel(self) {
        self.editing.set(false);
        self.value.set(String::new());
        self.error.set(false);
    }

    fn start(self) {
        self.value.set(
            self.state
                .with_untracked(|s| self.target.get_value().initial(s)),
        );
        self.error.set(false);
        self.editing.set(true);
    }

    fn commit(self) {
        let edit = edit_value(&self.value.get_untracked(), self.note);
        if edit == Edit::Invalid {
            self.error.set(true);
            return;
        }
        self.cancel();
        if !self.disabled.get_untracked() {
            if let Edit::Save(value) = edit {
                self.target.get_value().submit(self.state, value);
            }
        }
    }
}

fn editor_input(editor: Editor) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    input.on_load(|input| {
        let _ = input.focus();
        input.select();
    });
    view! {
        <input class="zxs-inline-input zxs-input zxs-input--compact zxs-input--underline" node_ref=input type="text"
            aria-label=editor.label.get_value() aria-invalid=move ||editor.error.get().to_string()
            title=t("最多 128 个 UTF-8 字节；Enter 保存，Esc 或点击其他位置取消。","Up to 128 UTF-8 bytes. Enter saves; Escape or clicking elsewhere cancels.")
            data-max-bytes="128" autocomplete="off" autocapitalize="none" spellcheck="false"
            prop:value=move ||editor.value.get() disabled=move ||editor.disabled.get()
            on:input=move |event|{editor.value.set(event_target_value(&event));editor.error.set(false);}
            on:blur=move |_|editor.cancel()
            on:keydown=move |event:web_sys::KeyboardEvent| {
                if event.is_composing(){return;}
                match event.key().as_str() {
                    "Escape"=>{event.prevent_default();event.stop_propagation();editor.cancel();}
                    "Enter"=>{event.prevent_default();event.stop_propagation();editor.commit();}
                    _=>{}
                }
            }/>
        <span class="zxs-inline-hint zxs-row" aria-live="polite">{move ||format!("{}/128 {}",editor.value.with(|value|value.len()),t("字节","bytes"))}</span>
        <Show when=move ||editor.error.get()>
            <span class="zxs-inline-error zxs-row" role="status">{if editor.note{t("备注最多 128 个 UTF-8 字节，不能包含换行或控制字符。","Notes must be at most 128 UTF-8 bytes without line breaks or control characters.")}else{t("名称无效，最多 128 个 UTF-8 字节。","Invalid name. Use at most 128 UTF-8 bytes.")}}</span>
        </Show>
    }
}

pub(super) fn control(
    state: RwSignal<State>,
    label: String,
    icon: Option<&'static str>,
    target: Target,
    disabled: Signal<bool>,
) -> impl IntoView {
    let editor = Editor::new(state, label, target, disabled);
    view! {
        <span class="zxs-inline-edit zxs-flow zxs-flow--inline" class:zxs-inline-note=editor.note>
            {move ||if editor.editing.get() {
                editor_input(editor).into_any()
            } else {
                view! {
                    <button class="zxs-button" type="button" aria-label=editor.label.get_value()
                        disabled=move ||editor.disabled.get() on:click=move |_|editor.start()>
                        {icon.map(|name|view!{<FileIcon name=name/>})}
                        {move ||if editor.note {
                            let note=state.with(|s|editor.target.get_value().initial(s));
                            if note.is_empty(){editor.label.get_value()}else{note}
                        }else{editor.label.get_value()}}
                    </button>
                }.into_any()
            }}
        </span>
    }
}

/// A version keeps its note on the second logical row and its explicit edit
/// action on the third. Only the note text is replaced by an underline input.
pub(super) fn version_note(
    state: RwSignal<State>,
    target: Target,
    node_id: String,
    disabled: Signal<bool>,
) -> (impl IntoView, impl IntoView) {
    let editor = Editor::new(state, t("编辑备注", "Edit note"), target, disabled);
    let note = view! {
        <div class="zxs-version-note-row zxs-row" class:zxs-inline-note=move ||editor.editing.get()>
            {move ||if editor.editing.get() {
                editor_input(editor).into_any()
            } else {
                view!{<span class="zxs-version-note">{move ||{
                    state.with(|s|editor.target.get_value().display(s))
                }}</span>{super::tags::chips(state,node_id.clone())}}.into_any()
            }}
        </div>
    };
    let can_modify = Signal::derive(move || {
        state.with(|s| s.files.writable && !super::model::selected_entry_read_only(&s.files))
    });
    let action = view! {
        <Show when=move ||can_modify.get()><button class="zxs-button zxs-version-note-action" type="button"
            disabled=move ||editor.disabled.get()||editor.editing.get()
            on:click=move |_|editor.start()>{t("编辑备注","Edit note")}</button></Show>
    };
    (note, action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_limits_are_bytes_without_splitting_utf8() {
        let ascii = "a".repeat(128);
        let chinese = format!("{}ab", "文".repeat(42));
        assert_eq!(ascii.len(), 128);
        assert_eq!(chinese.len(), 128);
        for value in [&ascii, &chinese] {
            assert_eq!(edit_value(value, true), Edit::Save(value.clone()));
            assert_eq!(edit_value(value, false), Edit::Save(value.clone()));
            assert_eq!(edit_value(&format!("{value}a"), true), Edit::Invalid);
            assert_eq!(edit_value(&format!("{value}a"), false), Edit::Invalid);
        }
        assert_eq!(edit_value(&"文".repeat(43), true), Edit::Invalid);
    }

    #[test]
    fn clearing_a_note_is_distinct_from_cancelling_a_name() {
        assert_eq!(edit_value("", true), Edit::Save(String::new()));
        assert_eq!(edit_value("  ", false), Edit::Cancel);
        assert_eq!(edit_value(" 草稿 ", false), Edit::Save("草稿".into()));
        for value in ["a\nb", "a\rb", "a\tb", "a\0b", "a\u{7f}b", "a\n"] {
            assert_eq!(edit_value(value, true), Edit::Invalid);
            assert_eq!(edit_value(value, false), Edit::Invalid);
        }
    }

    #[test]
    fn a_path_nodes_generated_caption_is_display_only_and_note_input_starts_from_metadata() {
        let mut state = State::default();
        state.files.file_history = serde_json::json!({"nodes":[{"node_id":"fnode-rename","version_id":"ver-one","kind":"rename","previous_path":"old/report.txt","path":"new/report.txt","note":""}]});
        let target = Target::NodeNote("fnode-rename".into());
        assert_eq!(target.initial(&state), "");
        assert_eq!(
            target.display(&state),
            "Path changed old/report.txt → new/report.txt"
        );
        assert!(target.is_note());
        state.files.file_history["nodes"][0]["note"] = "Reviewed rename".into();
        assert_eq!(target.display(&state), "Reviewed rename");
        assert_eq!(edit_value("", target.is_note()), Edit::Save(String::new()));
    }
    #[test]
    fn legacy_notes_are_displayed_without_silent_truncation() {
        let original = format!("{}\nlegacy", "文".repeat(60));
        let mut state = State::default();
        state.files.file_history = serde_json::json!({
            "note": original,
            "versions": [{"version_id": "old", "note": original}],
        });
        assert_eq!(Target::FileNote.initial(&state), original);
        assert_eq!(Target::VersionNote("old".into()).initial(&state), original);
        assert_eq!(edit_value(&original, true), Edit::Invalid);
        assert_eq!(state.files.file_history["note"], original);
    }
    #[test]
    fn directory_branch_rename_keeps_the_record_identity_and_shares_cancel_and_byte_rules() {
        let target = Target::RenameDirectoryBranch {
            repository: "repository-a".into(),
            branch_id: "directory-a".into(),
            name: "auto-generated".into(),
        };
        let state = State::default();
        assert_eq!(target.initial(&state), "auto-generated");
        assert!(!target.is_note());
        assert_eq!(edit_value("", target.is_note()), Edit::Cancel);
        assert_eq!(
            edit_value(&"a".repeat(129), target.is_note()),
            Edit::Invalid
        );
        assert_eq!(
            edit_value(" renamed ", target.is_note()),
            Edit::Save("renamed".into())
        );
    }
}
