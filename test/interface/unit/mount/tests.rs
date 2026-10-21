use super::*;
use std::sync::{Arc, Mutex};

struct TestMount(Arc<Mutex<Vec<&'static str>>>);

impl Mountable for TestMount {
    fn unmount(&mut self) {
        self.0.lock().unwrap().push("unmount");
    }
    fn mount(
        &mut self,
        _: &view_renderer::renderer::types::Element,
        _: Option<&view_renderer::renderer::types::Node>,
    ) {
        unreachable!("this lifecycle test does not mount browser DOM")
    }
    fn insert_before_this(&self, _: &mut dyn Mountable) -> bool {
        false
    }
    fn elements(&self) -> Vec<view_renderer::renderer::types::Element> {
        Vec::new()
    }
}

impl Drop for TestMount {
    fn drop(&mut self) {
        self.0.lock().unwrap().push("drop state");
    }
}

#[test]
fn unmount_disposes_resources_even_when_owner_is_retained() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let owner = Owner::new();
    let retained = owner.clone();
    owner.with(|| {
        let events = Arc::clone(&events);
        Owner::on_cleanup(move || events.lock().unwrap().push("cleanup"));
    });
    drop(UnmountHandle {
        owner,
        mountable: TestMount(Arc::clone(&events)),
    });
    assert_eq!(
        *events.lock().unwrap(),
        ["unmount", "cleanup", "drop state"]
    );
    // Final owner release must not execute cleanup a second time.
    drop(retained);
    assert_eq!(
        *events.lock().unwrap(),
        ["unmount", "cleanup", "drop state"]
    );
}

#[test]
fn unmount_also_disposes_child_owners_with_retained_handles() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let owner = Owner::new();
    let child = owner.child();
    child.with(|| {
        let events = Arc::clone(&events);
        Owner::on_cleanup(move || events.lock().unwrap().push("child cleanup"));
    });
    drop(UnmountHandle {
        owner,
        mountable: TestMount(Arc::clone(&events)),
    });
    assert_eq!(
        *events.lock().unwrap(),
        ["unmount", "child cleanup", "drop state"]
    );
    drop(child);
    assert_eq!(
        *events.lock().unwrap(),
        ["unmount", "child cleanup", "drop state"]
    );
}
