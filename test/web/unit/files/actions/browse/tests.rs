use super::*;

#[test]
fn listing_responses_require_the_original_location_generation_and_active_session() {
    Owner::new().with(|| {
        let state = RwSignal::new(State {
            authenticated: true,
            session_epoch: 1,
            ..Default::default()
        });
        state.update(|s| {
            s.files.slot = "P01".into();
            s.files.directory = "docs".into();
            s.files.generation = 3;
        });
        assert!(current_listing(state, 1, "P01", "docs", 3));
        assert!(!current_listing(state, 1, "P01", "docs", 2));
        assert!(!current_listing(state, 1, "P02", "docs", 3));
        assert!(!current_listing(state, 1, "P01", "other", 3));
        assert!(!current_listing(state, 0, "P01", "docs", 3));
        state.update(|s| s.logout_pending = true);
        assert!(!current_listing(state, 1, "P01", "docs", 3));
    });
}

#[test]
fn directory_navigation_drops_the_previous_write_permission_before_loading() {
    Owner::new().with(|| {
        let state = RwSignal::new(State::default());
        state.update(|s| {
            s.files.writable = true;
            s.files.loaded = true;
        });
        // No disk is selected, so this exercises the synchronous transition
        // without starting a browser request on the native test runner.
        go_directory(state, "another".into());
        state.with_untracked(|s| {
            assert!(!s.files.writable);
            assert!(!s.files.loaded);
            assert_eq!(s.files.directory, "another");
        });
    });
}

#[test]
fn selected_file_history_read_requires_an_active_session() {
    Owner::new().with(|| {
        for (authenticated, restoring, logout_pending, logout_unconfirmed) in [
            (false, false, false, false),
            (true, true, false, false),
            (true, false, true, false),
            (true, false, false, true),
        ] {
            let state = RwSignal::new(State {
                authenticated,
                restoring,
                logout_pending,
                logout_unconfirmed,
                ..Default::default()
            });
            state.update(|s| {
                s.files.selected = Some(json_codec::json!({"file_id":"file-1"}));
            });
            load_file_history(state);
            state.with_untracked(|s| {
                assert!(!s.files.history_loading);
                assert_eq!(s.files.history_generation, 0);
            });
        }
    });
}
