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
