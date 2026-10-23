use super::*;
use crate::BackendAction;
use storage_contract::{Config, Event, new_id};

fn event(sequence: u64) -> Event {
    Event {
        sequence,
        operation_id: format!("operation-{sequence}"),
        kind: "test".into(),
        object_id: "object".into(),
    }
}

#[test]
fn event_cursor_seeks_past_boundaries_and_gaps_without_copying() {
    assert!(events_after(&[], 0).is_empty());
    let events = [1, 3, 3, 8, u64::MAX].map(event);
    for after in [0, 1, 2, 3, 4, 8, 9, u64::MAX] {
        let expected = events
            .iter()
            .filter(|event| event.sequence > after)
            .collect::<Vec<_>>();
        let actual = events_after(&events, after);
        assert_eq!(actual.iter().collect::<Vec<_>>(), expected);
        assert!(std::ptr::eq(
            actual.as_ptr(),
            events[events.len() - expected.len()..].as_ptr()
        ));
    }
}

#[test]
fn event_cursor_at_large_history_tail_returns_only_unseen_events() {
    let events = (1..=100_000).map(event).collect::<Vec<_>>();
    assert_eq!(events_after(&events, 99_999), &events[99_999..]);
    assert!(events_after(&events, 100_000).is_empty());
}

fn poll(backend: &EngineBackend, identity: &Identity, after: u64) -> Result<Value, ApiError> {
    backend.call(
        BackendAction::Events,
        identity,
        json!({"query":{"after":after.to_string()}}),
    )
}

#[test]
fn event_cursor_preserves_visibility_progress_and_recovery_order() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("system");
    let engine =
        Engine::initialize(&root, Config::development([temporary.path().join("disk")])).unwrap();
    let backend = EngineBackend::new(engine);
    let mut identity = Identity {
        admission_session: None,
        smb_binding: None,
        principal: "owner".into(),
        admin: false,
        repositories: Default::default(),
    };
    assert_eq!(
        poll(&backend, &identity, 0).unwrap(),
        json!({"events":[],"next_cursor":"0"})
    );
    assert_eq!(
        poll(&backend, &identity, 1).unwrap_err().code,
        "INVALID_CURSOR"
    );
    let (first, last) = {
        let mut engine = backend.engine.lock().unwrap();
        let repo = engine
            .create_repository("Visible", "P01", 1, "owner", &new_id("op"))
            .unwrap();
        identity.repositories.insert(repo.repository_id);
        let first = engine.state().sequence;
        engine
            .create_repository("Hidden", "P01", 1, "other", &new_id("op"))
            .unwrap();
        (first, engine.state().sequence)
    };
    let expected = poll(&backend, &identity, 0).unwrap();
    assert_eq!(expected["events"].as_array().unwrap().len(), 1);
    assert_eq!(expected["events"][0]["sequence"], first.to_string());
    assert_eq!(expected["next_cursor"], last.to_string());
    for after in [first, last] {
        assert_eq!(
            poll(&backend, &identity, after).unwrap(),
            json!({"events":[],"next_cursor":last.to_string()})
        );
    }
    assert_eq!(
        poll(&backend, &identity, last + 1).unwrap_err().code,
        "INVALID_CURSOR"
    );
    let admin = Identity {
        admin: true,
        ..identity.clone()
    };
    assert_eq!(
        poll(&backend, &admin, 0).unwrap()["events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    drop(backend);
    let backend = EngineBackend::new(Engine::open(&root).unwrap());
    assert_eq!(poll(&backend, &identity, 0).unwrap(), expected);
    assert_eq!(
        poll(&backend, &admin, first).unwrap()["events"][0]["sequence"],
        last.to_string()
    );
}
