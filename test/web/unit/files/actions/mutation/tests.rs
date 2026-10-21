use super::*;

#[test]
fn business_mutations_require_an_active_session_before_browser_work() {
    for (authenticated, logout_pending) in [(true, true), (false, false)] {
        Owner::new().with(|| {
            let state = RwSignal::new(State {
                authenticated,
                logout_pending,
                ..Default::default()
            });
            state.update(|s| {
                s.files.writable = true;
                s.files.slot = "P01".into();
                s.files.selected = Some(json!({"file_id":"file"}));
            });
            // No browser ID generation or server request may start after
            // sign-out admission closes, even while credentials are kept
            // for a failed revocation retry.
            namespace(state, NamespaceAction::CreateDirectory, "new".into());
            directory_branch(state, "folder".into(), "draft".into(), false);
            update_note(state, None, "note".into());
            select_branch(state, "other".into());
            record_mutation(state, "PATCH", "/api/test".into(), json!({}));
            crate::files::review::submit(
                state,
                crate::files::review::Kind::Backup,
                "PUT",
                "/api/test".into(),
                json!({}),
            );
            state.update(|s| s.files.repository_id = "repository".into());
            crate::files::batch::start(state, crate::files::batch::Kind::Delete, None);
            assert!(!state.with_untracked(|s| s.files.mutation_pending));
            assert!(state.with_untracked(|s| s.files.batch.is_none()));
            assert!(state.with_untracked(|s| s.files.publications.is_empty()));
        });
    }
}

#[test]
fn refreshed_or_navigating_directories_block_writes_at_the_action_boundary() {
    Owner::new().with(|| {
        let state = RwSignal::new(State::default());
        state.update(|s| {
            s.files.writable = true;
            s.files.loading = true;
            s.files.selected = Some(json!({"file_id":"file"}));
        });
        // These must return before generating IDs or spawning browser work.
        namespace(state, NamespaceAction::CreateDirectory, "new".into());
        directory_branch(state, "folder".into(), "draft".into(), false);
        update_note(state, None, "note".into());
        select_branch(state, "other".into());
        assert!(!state.with_untracked(|s| s.files.mutation_pending));
    });
}
