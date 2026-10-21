use crate::{
    html::attribute::Attribute,
    hydration::Cursor,
    prelude::Mountable,
    view::{add_attr::AddAnyAttr, PositionState, Render, RenderHtml},
};
use reactive_graph::owner::Owner;
use std::mem;

/// A view wrapper that sets the reactive [`Owner`] to a particular owner whenever it is rendered.
#[derive(Debug, Clone)]
pub struct OwnedView<T> {
    owner: Owner,
    view: T,
}

impl<T> OwnedView<T> {
    /// Wraps a view with the current owner.
    pub fn new(view: T) -> Self {
        let owner = Owner::current().expect("no reactive owner");
        Self { owner, view }
    }

    /// Wraps a view with the given owner.
    pub fn new_with_owner(view: T, owner: Owner) -> Self {
        Self { owner, view }
    }
}

/// Retained view state for an [`OwnedView`].
#[derive(Debug, Clone)]
pub struct OwnedViewState<T>
where
    T: Mountable,
{
    // note: the drop order of the fields matters here
    // dropping `state` before `owner` ensures that cleanups happen
    // from the bottom up: i.e., the child state is dropped before
    // any other cleanups attached to this owner are fired
    state: T,
    owner: Owner,
}

impl<T> OwnedViewState<T>
where
    T: Mountable,
{
    /// Wraps a state with the given owner.
    fn new(state: T, owner: Owner) -> Self {
        Self { owner, state }
    }
}

impl<T> Render for OwnedView<T>
where
    T: Render,
{
    type State = OwnedViewState<T::State>;

    fn build(self) -> Self::State {
        let state = self.owner.with(|| self.view.build());
        OwnedViewState::new(state, self.owner)
    }

    fn rebuild(self, state: &mut Self::State) {
        let OwnedView { owner, view, .. } = self;
        owner.with(|| view.rebuild(&mut state.state));
        // Defer dropping the previous owner until after effects have run:
        // render effects that do things like reading from context it provides
        // may have already been triggered and queued to run. `spawn_local` will
        // defer the drop to the end of the queue, while still deterministically
        // cleaning up the memory to prevent leaks.
        let old_owner = mem::replace(&mut state.owner, owner);
        reactive_graph::spawn_local(async move { drop(old_owner) });
    }
}

impl<T> AddAnyAttr for OwnedView<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> = OwnedView<T::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let OwnedView { owner, view } = self;
        OwnedView {
            owner,
            view: view.add_any_attr(attr),
        }
    }
}

impl<T> RenderHtml for OwnedView<T>
where
    T: RenderHtml,
{
    // TODO

    type Owned = OwnedView<T::Owned>;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let state = self.owner.with(|| self.view.hydrate(cursor, position));
        OwnedViewState::new(state, self.owner)
    }

    fn into_owned(self) -> Self::Owned {
        OwnedView {
            owner: self.owner,
            view: self.view.into_owned(),
        }
    }
}

impl<T> Mountable for OwnedViewState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        self.state.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.state.mount(parent, marker);
    }

    fn try_mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) -> bool {
        self.state.try_mount(parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.state.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.state.elements()
    }
}
