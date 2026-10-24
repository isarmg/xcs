use super::*;
#[test]
fn multi_primary_routes_use_each_selected_prefix_and_the_shortest_capacity() {
    let configuration = json!({"route_slot_ids":["P01","B01","C01"],"backup_routes":[["P01","B01","C01"],["P02","B02"]],"backup_slot_ids":["B01","C01","B02"],"max_copies":2});
    assert_eq!(max_copies(&configuration), 2);
    assert_eq!(target_slots(&configuration, 2), vec!["B01", "B02"]);
    assert_eq!(target_slots(&configuration, 1), Vec::<String>::new());
    let draft = Draft {
        id: "draft".into(),
        repository: "repo".into(),
        path: "docs".into(),
        configuration,
        generation: "0".into(),
        scope_id: "scope".into(),
        target: 2,
        prefixes: BTreeMap::from([
            ("B01".into(), "first-chain".into()),
            ("B02".into(), "other-chain".into()),
        ]),
        loading: false,
    };
    assert_eq!(
        request(&draft).unwrap()["backup_prefixes"],
        json!({"B01":"first-chain","B02":"other-chain"})
    );
    let mut invalid = draft;
    invalid.target = 3;
    assert!(request(&invalid).is_err());
    assert!(target_slots(&json!({"available_route_slot_ids":["P01","B01"]}), 2).is_empty());
    assert_eq!(max_copies(&json!({"route_slot_ids":["P01","B01"]})), 0);
    assert_eq!(max_copies(&json!({"backup_routes":[["P01","B01"]]})), 0);
    assert!(routes(&json!({"backup_routes":[["P01",42]]})).is_empty());
    assert!(
        backup_paths(&json!({"backup_paths":[{"slot_id":"B01","logical_path":"old"}]})).is_empty()
    );
}
#[test]
fn independent_prefixes_are_relative_to_a_stable_scope_and_route() {
    let draft = Draft {
        id: "draft".into(),
        repository: "repo".into(),
        path: "docs".into(),
        configuration: json!({"backup_routes":[["P01","B01","C01"]],"max_copies":3}),
        generation: "0".into(),
        scope_id: "stable-directory".into(),
        target: 3,
        prefixes: BTreeMap::from([
            ("B01".into(), "我的备份".into()),
            ("C01".into(), "长期保存".into()),
        ]),
        loading: false,
    };
    let body = request(&draft).unwrap();
    assert_eq!(body["backup_prefixes"]["B01"], "我的备份");
    assert_eq!(body["backup_prefixes"]["C01"], "长期保存");
    assert_eq!(body["expected_scope_id"], "stable-directory");
    assert_eq!(body["expected_generation"], "0");
    assert!(valid_prefix(&"a".repeat(200)));
    assert!(!valid_prefix("../escape"));
    assert!(!valid_prefix("/absolute"));
    let mut invalid = draft;
    invalid.target = 4;
    assert!(request(&invalid).is_err());
}

#[test]
fn backup_read_does_not_allocate_a_draft_for_an_inactive_session() {
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
            start(state, "repo".into(), "docs".into());
            assert!(state.with_untracked(|s| s.files.backup.is_none()));
        }
    });
}

#[test]
fn backup_submission_keeps_session_and_storage_guards_without_role_fields() {
    Owner::new().with(|| {
        for (authenticated, transition, storage, writable) in [
            (
                false,
                crate::ui::SessionTransition::Idle,
                crate::management::Storage::Ready,
                true,
            ),
            (
                true,
                crate::ui::SessionTransition::Restoring,
                crate::management::Storage::Ready,
                true,
            ),
            (
                true,
                crate::ui::SessionTransition::LoggingOut,
                crate::management::Storage::Ready,
                true,
            ),
            (
                true,
                crate::ui::SessionTransition::LogoutUnconfirmed,
                crate::management::Storage::Ready,
                true,
            ),
            (
                true,
                crate::ui::SessionTransition::Idle,
                crate::management::Storage::ReadOnly,
                true,
            ),
            (
                true,
                crate::ui::SessionTransition::Idle,
                crate::management::Storage::Unavailable,
                true,
            ),
            (
                true,
                crate::ui::SessionTransition::Idle,
                crate::management::Storage::Ready,
                false,
            ),
        ] {
            let state = RwSignal::new(State {
                authenticated,
                session_transition: transition,
                ..Default::default()
            });
            state.update(|s| {
                s.management.storage = storage;
                s.files.writable = writable;
                s.files.backup = Some(Draft {
                    id: "draft".into(),
                    repository: "repo".into(),
                    path: "docs".into(),
                    configuration: json!({"backup_routes":[["P01"]],"max_copies":1}),
                    generation: "0".into(),
                    scope_id: "scope".into(),
                    target: 1,
                    prefixes: BTreeMap::new(),
                    loading: false,
                });
            });
            submit(state);
            state.with_untracked(|s| {
                assert!(s.files.backup.is_some());
                assert!(!s.files.mutation_pending);
                assert!(s.files.publications.is_empty());
            });
        }
    });
}
