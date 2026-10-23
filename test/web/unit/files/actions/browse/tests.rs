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
        state.update(|s| s.session_transition = crate::ui::SessionTransition::LoggingOut);
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
        for (authenticated, session_transition) in [
            (false, crate::ui::SessionTransition::Idle),
            (true, crate::ui::SessionTransition::Restoring),
            (true, crate::ui::SessionTransition::LoggingOut),
            (true, crate::ui::SessionTransition::LogoutUnconfirmed),
        ] {
            let state = RwSignal::new(State {
                authenticated,
                session_transition,
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

#[test]
fn a_failed_disk_refresh_invalidates_readiness_without_discarding_cached_rows() {
    Owner::new().with(|| {
        for disks in [
            vec![],
            vec![crate::files::model::Disk {
                slot_id: "P01".into(),
                online: true,
                writable: true,
                available_bytes: 4096,
            }],
        ] {
            let state = RwSignal::new(State {
                authenticated: true,
                ..Default::default()
            });
            state.update(|s| {
                s.files.disks = disks.clone();
                s.files.disks_loaded = true;
            });
            fail_disk_load(
                state,
                api::Error {
                    status: 503,
                    code: "UNAVAILABLE".into(),
                    message: "Transient disk listing failure".into(),
                    retryable: true,
                    request_id: None,
                },
            );
            state.with_untracked(|s| {
                assert!(
                    !s.files.disks_loaded,
                    "a later health poll must retry this read"
                );
                assert_eq!(s.files.disks, disks);
                assert!(s.session_active());
                assert_eq!(s.errors.len(), 1);
            });
        }
    });
}
