use super::*;
fn authorized() -> Model {
    Model {
        key: "a".repeat(64),
        csrf: "csrf".into(),
        username: "admin".into(),
        password: "password".into(),
        disks: vec!["A".into(), "B".into(), "C".into(), "D".into()],
        selected: vec!["A".into(), "B".into(), "C".into()],
        empty: true,
        confirmed: true,
        ..Default::default()
    }
}
#[test]
fn initialization_requires_explicit_empty_confirmation_and_settled_outcome() {
    assert!(!can_initialize(&Model::default()));
    let mut m = authorized();
    assert!(can_initialize(&m));
    m.confirmed = false;
    assert!(!can_initialize(&m));
    m.confirmed = true;
    m.empty = false;
    assert!(!can_initialize(&m));
    m.empty = true;
    m.pending = true;
    assert!(!can_initialize(&m));
    m.pending = false;
    m.unknown = true;
    assert!(!can_initialize(&m));
    m.unknown = false;
    m.complete = true;
    assert!(!can_initialize(&m));
    m.complete = false;
    m.csrf.clear();
    assert!(!can_initialize(&m));
}
#[test]
fn one_or_more_unique_allowlisted_disk_ids_are_required() {
    let mut m = authorized();
    assert!(valid_selection(&m));
    m.selected.push("D".into());
    assert!(valid_selection(&m));
    m.selected = vec!["A".into()];
    assert!(valid_selection(&m));
    m.selected.clear();
    assert!(!valid_selection(&m));
    m.selected = vec!["A".into(), "B".into(), "/tmp/unsafe".into()];
    assert!(!valid_selection(&m));
    m.selected = vec!["A".into(), "A".into(), "B".into()];
    assert!(!valid_selection(&m));
    m.disks = (0..33).map(|n| format!("disk-{n}")).collect();
    m.selected = m.disks.clone();
    assert!(!valid_selection(&m));
    m.selected.pop();
    assert!(valid_selection(&m));
}

#[test]
fn completion_requires_authorized_proof_and_never_infers_account_reset() {
    let mut m = authorized();
    m.action = Action::Complete;
    m.empty = false;
    m.status = "recovery_required".into();
    m.unknown = true;
    assert!(!can_initialize(&m));
    accept_status(
        &mut m,
        &json!({"state":"recovery_required","csrf_token":"csrf"}),
    );
    assert!(!m.completion_available);
    assert!(!can_initialize(&m));
    accept_status(
        &mut m,
        &json!({"state":"existing","csrf_token":"csrf","completion_available":true}),
    );
    assert!(
        can_initialize(&m),
        "new explicit proof admits only bounded completion"
    );
    m.action = Action::Wizard;
    assert!(!can_initialize(&m), "proof does not authorize fresh init");
    m.action = Action::Complete;
    m.confirmed = false;
    assert!(!can_initialize(&m));
    accept_status(
        &mut m,
        &json!({"state":"recovery_required","completion_available":true}),
    );
    assert!(
        !m.completion_available,
        "proof without CSRF authorization is incomplete"
    );
}

#[test]
fn preset_requires_exact_review_and_does_not_rewrite_configuration() {
    let mut m = authorized();
    m.action = Action::Preset;
    m.presets = vec!["local-four-disk".into()];
    m.preset_id = "local-four-disk".into();
    assert!(!can_initialize(&m));
    m.preset_preview = json!({"disks":[{"candidate_id":"A"},{"candidate_id":"B"},{"candidate_id":"C"},{"candidate_id":"D"}],"resource_limits":{"workers":7}});
    assert!(!can_initialize(&m));
    m.preset_digest = "review-bound-digest".into();
    assert!(can_initialize(&m));
    m.selected.clear();
    assert!(
        can_initialize(&m),
        "preset disk count does not depend on wizard selection"
    );
    let (path, body) = submission(&m, "secret".into());
    assert_eq!(path, "/api/setup/initialize-preset");
    assert_eq!(
        body,
        json!({"preset_id":"local-four-disk","config_digest":"review-bound-digest","username":"admin","password":"secret"})
    );
    assert!(body.get("configuration").is_none());
    assert!(body.get("mode").is_none());
    assert!(body.get("disk_ids").is_none());
    m.preset_id = "unapproved".into();
    assert!(!can_initialize(&m));
}

#[test]
fn changing_setup_action_invalidates_confirmation_and_completion_is_bounded() {
    let mut m = authorized();
    choose(&mut m, Action::Complete);
    assert!(!m.confirmed);
    let (path, body) = submission(&m, "secret".into());
    assert_eq!(path, "/api/setup/complete-initialization");
    assert_eq!(body, json!({"username":"admin","password":"secret"}));
    choose(&mut m, Action::Preset);
    assert!(!m.confirmed);
}

#[test]
fn single_disk_submission_contains_only_selected_authorized_id() {
    let mut m = authorized();
    m.disks = vec!["A".into()];
    m.selected = vec!["A".into()];
    assert!(can_initialize(&m));
    let (path, body) = submission(&m, "secret".into());
    assert_eq!(path, "/api/setup/initialize");
    assert_eq!(
        body,
        json!({"disk_ids":["A"],"mode":"production","username":"admin","password":"secret"})
    );
    m.development = true;
    assert_eq!(submission(&m, "secret".into()).1["mode"], "production");
    m.allow_development = true;
    assert_eq!(submission(&m, "secret".into()).1["mode"], "development");
}

#[test]
fn setup_outcome_blocks_app_owned_reload_until_confirmed() {
    let mut model = Model::default();
    assert!(!model.reload_blocked());
    model.pending = true;
    assert!(model.reload_blocked());
    model.pending = false;
    model.unknown = true;
    assert!(model.reload_blocked());
    model.unknown = false;
    model.complete = true;
    assert!(!model.reload_blocked());
}

#[test]
fn verified_completion_releases_reload_guard_without_readmitting_initialization() {
    for success in [
        json!({"accepted":true}),
        json!({"initialized":true,"restart_required":true}),
    ] {
        let mut model = authorized();
        finish_initialization(&mut model, Ok(json!({})));
        assert!(model.unknown);
        assert!(model.reload_blocked());
        assert!(!can_initialize(&model));
        accept_status(
            &mut model,
            &json!({"state":"existing","csrf_token":"csrf","completion_available":true}),
        );
        choose(&mut model, Action::Complete);
        model.confirmed = true;
        assert!(can_initialize(&model));
        assert!(
            model.reload_blocked(),
            "proof alone cannot settle the outcome"
        );
        model.pending = true;
        finish_initialization(&mut model, Ok(success));
        assert!(model.complete);
        assert!(!model.unknown);
        assert!(!model.reload_blocked());
        assert!(!can_initialize(&model));
    }
}

#[test]
fn failed_completion_preserves_the_original_unknown_reload_fence() {
    let mut model = authorized();
    model.unknown = true;
    model.pending = true;
    finish_initialization(
        &mut model,
        Err(api::Error {
            code: "INVALID_REQUEST".into(),
            message: "rejected".into(),
            retryable: false,
            status: 400,
            request_id: None,
        }),
    );
    assert!(!model.pending);
    assert!(model.unknown);
    assert!(!model.complete);
    assert!(model.reload_blocked());
}
