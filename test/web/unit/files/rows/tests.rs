use super::*;
use json_codec::json;
use std::cell::Cell;

thread_local! {
    static SORTS: Cell<usize> = const { Cell::new(0) };
    static PROJECTED: Cell<usize> = const { Cell::new(0) };
    static FILTERS: Cell<usize> = const { Cell::new(0) };
}
pub(super) fn record_sort(entries: usize) {
    SORTS.set(SORTS.get() + 1);
    PROJECTED.set(PROJECTED.get() + entries);
}
pub(super) fn record_filter() {
    FILTERS.set(FILTERS.get() + 1);
}
fn reset_counts() {
    SORTS.set(0);
    PROJECTED.set(0);
    FILTERS.set(0);
}
fn state(entries: Vec<Value>) -> State {
    let mut state = State {
        authenticated: true,
        session_epoch: 7,
        ..Default::default()
    };
    state.files.slot = "disk".into();
    state.files.directory = "docs".into();
    state.files.entries = entries.into();
    state
}
fn entry(id: &str, path: &str, name: &str, kind: &str) -> Value {
    json!({"entry_id":id,"path":path,"name":name,"kind":kind,"version_id":"old","size":1})
}

#[test]
fn unrelated_updates_do_not_project_sort_or_filter_entries() {
    for size in [100, 1_000, 10_000] {
        let owner = Owner::new();
        owner.with(|| {
            reset_counts();
            let state = RwSignal::new(state((0..size).map(|i| entry(&i.to_string(), &format!("docs/{i}"), &i.to_string(), "file")).collect()));
            let rows = visible_rows(state);
            assert_eq!(rows.with_untracked(Vec::len), size);
            for i in 0..100 {
                state.update(|s| {
                    s.notice = i.to_string();
                    s.files.selected = Some(s.files.entries[i % size].clone());
                    s.files.checked_entries.insert(i.to_string());
                    s.files.preview = Some(("preview".into(), i.to_string()));
                    s.files.file_history = json!({"history":i});
                    s.transfers.filter = Some(crate::files::SaveStatus::Uploading);
                });
                assert_eq!(rows.with_untracked(Vec::len), size);
            }
            assert_eq!((SORTS.get(), PROJECTED.get(), FILTERS.get()), (1, size, 1));
            state.update(|s|s.files.query = "1".into());
            assert!(!rows.get_untracked().is_empty());
            assert_eq!((SORTS.get(), PROJECTED.get(), FILTERS.get()), (1, size, 2));
            println!("entries={size}; unrelated_updates=100; sorts=1; projected={size}; filters_before_query=1; filters_after_query=2");
        });
    }
}

#[test]
fn ordering_filtering_and_equal_name_server_order_are_preserved() {
    let owner = Owner::new();
    owner.with(|| {
        let state = RwSignal::new(state(vec![
            entry("f1", "docs/z", "z", "file"),
            entry("f2", "docs/a", "A", "file"),
            entry("d1", "docs/Z", "Z", "directory"),
            entry("f3", "docs/a2", "a", "file"),
            entry("d2", "docs/b", "b", "directory"),
        ]));
        let rows = visible_rows(state);
        let ids = || {
            rows.with_untracked(|rows| {
                rows.iter()
                    .map(|row| row.selection_id.clone())
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(ids(), ["d2", "d1", "f2", "f3", "f1"]);
        let retained = rows.get_untracked()[2].clone();
        state.update(|s| s.files.query = "A".into());
        assert_eq!(ids(), ["f2", "f3"]);
        assert!(Arc::ptr_eq(&retained, &rows.get_untracked()[0]));
        state.update(|s| s.files.query = "absent".into());
        assert!(ids().is_empty());
    });
}

#[test]
fn retained_key_refreshes_payload_and_pending_handlers_resolve_current_data() {
    let owner = Owner::new();
    owner.with(|| {
        let state = RwSignal::new(state(vec![entry("f", "docs/a", "old name", "file")]));
        let rows = visible_rows(state);
        let old = rows.get_untracked().remove(0);
        state.update(|s| {
            let mut refreshed = s.files.entries[0].clone();
            refreshed["name"] = "new name".into();
            refreshed["version_id"] = "new".into();
            refreshed["size"] = 42.into();
            s.files.entries = vec![refreshed].into();
        });
        // Even before recomputing the row signal, queued handlers cannot use
        // the old version. Current server metadata is authoritative.
        assert_eq!(
            state.with_untracked(|s| old.current_entry(s).unwrap()["version_id"].clone()),
            "new"
        );
        let new = rows.get_untracked().remove(0);
        assert_eq!(old.key, new.key);
        assert_ne!(old, new);
        assert_eq!(new.name, "new name");
        assert_eq!(new.size, Some(42));
        state.update(|s| s.files.entries = Default::default());
        assert!(state.with_untracked(|s| old.current_entry(s).is_none()));
    });
}

#[test]
fn aliases_sessions_and_navigation_do_not_reuse_wrong_identity() {
    let mut state = state(vec![
        entry("f", "docs/a", "a", "file"),
        entry("f", "docs/b", "b", "file"),
    ]);
    let old = Listing::capture(&state).project();
    assert_ne!(old[0].key, old[1].key);
    state.files.directory = "another".into();
    assert!(old[0].current_entry(&state).is_none());
    assert_ne!(old[0].key, Listing::capture(&state).project()[0].key);
    state.files.directory = "docs".into();
    state.files.slot = "other-disk".into();
    assert!(old[0].current_entry(&state).is_none());
    state.files.slot = "disk".into();
    state.session_epoch += 1;
    assert!(old[0].current_entry(&state).is_none());
    assert_ne!(old[0].key, Listing::capture(&state).project()[0].key);
    state.session_epoch -= 1;
    state.logout_pending = true;
    assert!(old[0].current_entry(&state).is_none());
}

#[test]
fn copy_on_write_cannot_hide_changed_listing_from_memo() {
    let owner = Owner::new();
    owner.with(|| {
        let state = RwSignal::new(state(vec![entry("f", "docs/a", "before", "file")]));
        let rows = visible_rows(state);
        let before = rows.get_untracked().remove(0);
        state.update(|s| Arc::make_mut(&mut s.files.entries)[0]["name"] = "after".into());
        let after = rows.get_untracked().remove(0);
        assert_eq!(before.key, after.key);
        assert_ne!(before, after);
        assert_eq!(after.name, "after");
        assert_eq!(before.name, "before");
    });
}

#[test]
fn display_rows_do_not_retain_old_full_snapshots_or_stale_hidden_metadata() {
    let mut state = state(vec![entry("f", "docs/a", "a", "file")]);
    let old_snapshot = Arc::downgrade(&state.files.entries);
    let old = Listing::capture(&state).project().remove(0);
    let mut updated = state.files.entries[0].clone();
    updated["version_id"] = "new-version".into();
    state.files.entries = vec![updated].into();
    let new = Listing::capture(&state).project().remove(0);
    assert_eq!(
        old, new,
        "hidden metadata does not invalidate unchanged display data"
    );
    assert!(
        old_snapshot.upgrade().is_none(),
        "retained rows must not pin whole old listings"
    );
    assert_eq!(
        old.current_entry(&state).unwrap()["version_id"],
        "new-version"
    );
}
