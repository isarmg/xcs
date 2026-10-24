use super::BackendAction;
#[test]
fn every_action_round_trips_and_unknown_actions_are_rejected() {
    let mut names = std::collections::BTreeSet::new();
    for &action in BackendAction::ALL {
        assert!(names.insert(action.as_str()));
        assert_eq!(BackendAction::try_from(action.as_str()).unwrap(), action);
    }
    for unknown in [
        "",
        "file_manager_unknown",
        "repositories ",
        "Repositories",
        "set_grants",
    ] {
        assert!(BackendAction::try_from(unknown).is_err());
    }
}
