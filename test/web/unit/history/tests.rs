use super::*;

#[test]
fn branch_registration_is_distinct_from_legacy_content_creation() {
    assert_eq!(action_label("file_branch_pending"), "Registered branch");
    assert_eq!(action_label("branch_pending"), "Registered branch");
    assert_eq!(action_label("file_branch"), "Created file branch");
    assert_eq!(
        action_label("directory.branch.rename"),
        "Renamed folder branch"
    );
    assert_eq!(
        action_label("directory.default.select"),
        "Switched SMB default folder branch"
    );
    assert_eq!(
        action_label("directory.branches.create"),
        "Registered folder branch"
    );
    assert_eq!(
        action_label("directory.branches.select"),
        "Switched folder branch"
    );
    assert_eq!(action_label("directory.tree.write"), "Wrote folder subtree");
}

thread_local! {
    static PROJECTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(super) fn record_projection() {
    PROJECTIONS.set(PROJECTIONS.get() + 1);
}

#[test]
fn unrelated_updates_do_not_reproject_history_and_filters_preserve_order() {
    Owner::new().with(|| {
        PROJECTIONS.set(0);
        let state = RwSignal::new(State::default());
        state.update(|s| {
            s.activity.events = (0..200)
                .map(|i| json_codec::json!({"slot_id": if i % 2 == 0 {"P01"} else {"P02"}, "path": i.to_string()}))
                .collect::<Vec<_>>()
                .into();
        });
        let events = visible_events(state);
        assert_eq!(events.with_untracked(Vec::len), 200);
        for i in 0..100 {
            state.update(|s| {
                s.management.loading = i % 2 == 0;
                s.notice = i.to_string();
                s.activity.loading = i % 2 == 0;
            });
            assert_eq!(events.with_untracked(Vec::len), 200);
        }
        assert_eq!(PROJECTIONS.get(), 1);

        state.update(|s| s.activity.disk = "P02".into());
        assert_eq!(events.with_untracked(|events| events.iter().map(|event| text(event, &["path"])).collect::<Vec<_>>()),
            (0..200).filter(|i| i % 2 == 1).map(|i| i.to_string()).collect::<Vec<_>>());
        assert_eq!(PROJECTIONS.get(), 2);

        state.update(|s| {
            s.activity.events = vec![
                json_codec::json!({"slot_id":"P02", "path":"z"}),
                json_codec::json!({"slot_id":"P01", "path":"hidden"}),
                json_codec::json!({"slot_id":"P02", "path":"a"}),
            ].into();
        });
        assert_eq!(events.with_untracked(|events| events.iter().map(|event| text(event, &["path"])).collect::<Vec<_>>()), ["z", "a"]);
        assert_eq!(PROJECTIONS.get(), 3);

        state.update(|s| s.activity.disk = "missing".into());
        assert!(events.with_untracked(Vec::is_empty));
        assert_eq!(PROJECTIONS.get(), 4);
        state.update(|s| s.activity.disk.clear());
        assert_eq!(events.with_untracked(Vec::len), 3);
        assert_eq!(PROJECTIONS.get(), 5);
        state.update(|s| s.activity.events = Default::default());
        assert!(events.with_untracked(Vec::is_empty));
        assert_eq!(PROJECTIONS.get(), 6);
    });
}
