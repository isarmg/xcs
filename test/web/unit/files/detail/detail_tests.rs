use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn an_unknown_or_loading_note_cannot_be_submitted_as_an_empty_note() {
    let mut model = Model::default();
    assert!(!file_note_ready(&model));
    model.file_history = serde_json::json!({"note":"existing note"});
    model.history_loading = true;
    assert!(!file_note_ready(&model));
    model.history_loading = false;
    assert!(file_note_ready(&model));
    model.file_history = serde_json::json!({"note":""});
    assert!(
        file_note_ready(&model),
        "a known empty note remains editable"
    );
    model.file_history = Value::Null;
    assert!(
        !file_note_ready(&model),
        "a failed first request never implies an empty note"
    );
}

#[test]
fn metadata_completion_keeps_the_current_detail_controls_mounted() {
    Owner::new().with(|| {
        let state = RwSignal::new(State::default());
        state.update(|s| {
            s.files.selected = Some(
                serde_json::json!({"entry_id":"a","file_id":"a","name":"a.txt","version_id":"v1"}),
            )
        });
        let selected = detail_selection(state);
        let mounts = Arc::new(AtomicUsize::new(0));
        let count = mounts.clone();
        let detail = Memo::new(move |_| {
            let entry = selected.get();
            count.fetch_add(1, Ordering::SeqCst);
            entry
        });
        assert!(detail.get().is_some());
        state.update(|s| {
            s.files.history_loading = true;
            s.files.mutation_pending = true;
        });
        state.update(|s| {
            s.files.file_history =
                serde_json::json!({"branches":[{"name":"main"},{"name":"draft"}]});
            s.files.history_loading = false;
            s.files.mutation_pending = false;
        });
        state.update(|s| {
            s.files.selected = Some(
                serde_json::json!({"entry_id":"a","file_id":"a","name":"a.txt","version_id":"v1"}),
            )
        });
        assert!(detail.get().is_some());
        assert_eq!(
            mounts.load(Ordering::SeqCst),
            1,
            "completion of either metadata request must retain detail ownership"
        );
        state.update(|s| s.files.selected.as_mut().unwrap()["name"] = "renamed.txt".into());
        assert_eq!(detail.get().unwrap()["name"], "renamed.txt");
        assert_eq!(
            mounts.load(Ordering::SeqCst),
            2,
            "renaming the container must refresh version download names"
        );
    });
}
