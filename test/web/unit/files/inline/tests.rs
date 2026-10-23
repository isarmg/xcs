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
    state.files.file_history = json_codec::json!({"nodes":[{"node_id":"fnode-rename","version_id":"ver-one","kind":"rename","previous_path":"old/report.txt","path":"new/report.txt","note":""}]}).into();
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
    state.files.file_history = json_codec::json!({
        "note": original,
        "versions": [{"version_id": "old", "note": original}],
    })
    .into();
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

fn active_state() -> State {
    State {
        authenticated: true,
        session_epoch: 7,
        page: crate::ui::Page::Details,
        ..Default::default()
    }
}

#[test]
fn an_editor_keeps_text_until_its_own_success_and_locks_unknown_outcomes() {
    Owner::new().with(|| {
        let state = RwSignal::new(active_state());
        let draft = Draft::unobserved(state);
        draft.start("user's exact draft".into());
        draft.submit(|| Some("first".into()));
        draft.cancel_idle(); // A disabled input's blur is not cancellation.
        assert_eq!(draft.value.get_untracked(), "user's exact draft");
        assert!(draft.editing.get_untracked());
        draft.settle("unrelated", super::super::model::SaveStatus::Published);
        draft.settle("first", super::super::model::SaveStatus::Unknown);
        assert!(draft.pending());
        draft.submit(|| panic!("unknown outcomes cannot submit another operation"));
        draft.settle("first", super::super::model::SaveStatus::Rejected);
        assert!(!draft.pending());
        assert_eq!(draft.value.get_untracked(), "user's exact draft");
        assert!(draft.editing.get_untracked());
        draft.submit(|| Some("retry".into()));
        draft.settle("first", super::super::model::SaveStatus::Published);
        assert_eq!(draft.value.get_untracked(), "user's exact draft");
        draft.settle("retry", super::super::model::SaveStatus::Degraded);
        assert!(!draft.editing.get_untracked());
        assert_eq!(draft.value.get_untracked(), "");
    });
}

#[test]
fn rejected_admission_keeps_the_draft_and_unsubmitted_edits_can_cancel() {
    Owner::new().with(|| {
        let state = RwSignal::new(active_state());
        let draft = Draft::unobserved(state);
        draft.start("keep me".into());
        draft.submit(|| None);
        assert_eq!(draft.value.get_untracked(), "keep me");
        assert!(!draft.pending());
        draft.cancel_idle();
        assert!(!draft.editing.get_untracked());
        assert_eq!(draft.value.get_untracked(), "");
    });
}

#[test]
fn queued_edits_cannot_submit_after_session_navigation_alias_or_branch_changes() {
    Owner::new().with(|| {
        for change in ["epoch", "logout", "page", "slot", "directory", "repository", "directory_branch", "path", "branch", "version"] {
            let mut initial = active_state();
            initial.files.selected = Some(json_codec::json!({"entry_id":"same","file_id":"same","path":"docs/a","selected_branch_id":"main","version_id":"v1"}));
            let state = RwSignal::new(initial);
            let draft = Draft::unobserved(state);
            draft.start("old context".into());
            state.update(|s| match change {
                "epoch" => s.session_epoch += 1,
                "logout" => s.session_transition = crate::ui::SessionTransition::LoggingOut,
                "page" => s.page = crate::ui::Page::Files,
                "slot" => s.files.slot = "other".into(),
                "directory" => s.files.directory = "other".into(),
                "repository" => s.files.repository_id = "other".into(),
                "directory_branch" => s.files.directory_branches = vec![json_codec::json!({"current":true,"directory_branch_id":"other"})],
                "path" => s.files.selected.as_mut().unwrap()["path"] = "docs/b".into(),
                "branch" => s.files.selected.as_mut().unwrap()["selected_branch_id"] = "other".into(),
                "version" => s.files.selected.as_mut().unwrap()["version_id"] = "v2".into(),
                _ => unreachable!(),
            });
            draft.submit(|| panic!("old editor must not submit into a new context"));
            assert!(!draft.editing.get_untracked());
            assert!(!draft.pending());
        }
    });
}
