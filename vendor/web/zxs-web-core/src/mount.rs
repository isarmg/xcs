//! CSR mounting and reactive ownership. This product has no hydration mode.
use crate::IntoView;
use any_spawner::Executor;
use reactive_graph::owner::Owner;
use tachys::{
    dom::body,
    view::{Mountable, Render},
};
use web_sys::HtmlElement;

/// Build and mount a browser CSR application to the document body.
pub fn mount_to_body<F, N>(app: F)
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    mount_to(body(), app).forget();
}

/// Mount one CSR tree and retain its reactive owner until explicitly dropped.
pub fn mount_to<F, N>(parent: HtmlElement, app: F) -> UnmountHandle<N::State>
where
    F: FnOnce() -> N + 'static,
    N: IntoView,
{
    let _ = Executor::init_wasm_bindgen();
    let owner = Owner::new();
    let mountable = owner.with(move || {
        let mut view = app().into_view().build();
        view.mount(&parent, None);
        view
    });
    UnmountHandle { owner, mountable }
}

/// Build and mount a CSR view into the engine's DOM renderer.
pub fn mount_to_renderer<F, N>(
    parent: &tachys::renderer::types::Element,
    app: F,
) -> UnmountHandle<N::State>
where
    F: FnOnce() -> N + 'static,
    N: Render,
{
    let _ = Executor::init_wasm_bindgen();
    let owner = Owner::new();
    let mountable = owner.with(move || {
        let mut view = app().build();
        view.mount(parent, None);
        view
    });
    UnmountHandle { owner, mountable }
}

/// Dropping the handle unmounts its DOM tree and then releases scoped resources.
#[must_use = "Keep this handle alive, or call forget for an application root"]
pub struct UnmountHandle<M: Mountable> {
    owner: Owner,
    mountable: M,
}
impl<M: Mountable> UnmountHandle<M> {
    /// Keep an application root mounted until its document is destroyed.
    pub fn forget(self) {
        std::mem::forget(self);
    }
}
impl<M: Mountable> Drop for UnmountHandle<M> {
    fn drop(&mut self) {
        self.mountable.unmount();
        // A view, context, or external caller may retain an Owner clone.
        // Unmount is a disposal boundary, not merely an Arc decrement.
        self.owner.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct TestMount(Arc<Mutex<Vec<&'static str>>>);

    impl Mountable for TestMount {
        fn unmount(&mut self) {
            self.0.lock().unwrap().push("unmount");
        }
        fn mount(
            &mut self,
            _: &tachys::renderer::types::Element,
            _: Option<&tachys::renderer::types::Node>,
        ) {
            unreachable!("this lifecycle test does not mount browser DOM")
        }
        fn insert_before_this(&self, _: &mut dyn Mountable) -> bool {
            false
        }
        fn elements(&self) -> Vec<tachys::renderer::types::Element> {
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
}
