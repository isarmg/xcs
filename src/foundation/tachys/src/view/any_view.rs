#![allow(clippy::type_complexity)]

use super::{add_attr::AddAnyAttr, Mountable, PositionState, Render, RenderHtml};
use crate::{
    erased::{Erased, ErasedLocal},
    html::attribute::{
        any_attribute::{AnyAttribute, AnyAttributeState, IntoAnyAttribute},
        Attribute,
    },
    hydration::Cursor,
    renderer::Rndr,
};
use std::{any::TypeId, fmt::Debug};

/// A type-erased view. This can be used if control flow requires that multiple different types of
/// view must be received, and it is either impossible or too cumbersome to use the `EitherOf___`
/// enums.
///
/// It can also be used to create recursive components, which otherwise cannot return themselves
/// due to the static typing of the view tree.
///
/// Generally speaking, using `AnyView` restricts the amount of information available to the
/// compiler and should be limited to situations in which it is necessary to preserve the maximum
/// amount of type information possible.
pub struct AnyView {
    type_id: TypeId,
    value: Erased,
    build: fn(Erased) -> AnyViewState,
    rebuild: fn(Erased, &mut AnyViewState),
    // The fields below are cfg-gated so they will not be included in WASM bundles if not needed.
    // Ordinarily, the compiler can simply omit this dead code because the methods are not called.
    // With this type-erased wrapper, however, the compiler is not *always* able to correctly
    // eliminate that code.
}

impl AnyView {
    #[doc(hidden)]
    pub fn as_type_id(&self) -> TypeId {
        self.type_id
    }
}

impl Debug for AnyView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyView")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}
/// Retained view state for [`AnyView`].
pub struct AnyViewState {
    type_id: TypeId,
    state: ErasedLocal,
    unmount: fn(&mut ErasedLocal),
    mount: fn(
        &mut ErasedLocal,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ),
    insert_before_this: fn(&ErasedLocal, child: &mut dyn Mountable) -> bool,
    elements: fn(&ErasedLocal) -> Vec<crate::renderer::types::Element>,
    placeholder: Option<crate::renderer::types::Placeholder>,
}

impl Debug for AnyViewState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyViewState")
            .field("type_id", &self.type_id)
            .field("state", &"")
            .field("unmount", &self.unmount)
            .field("mount", &self.mount)
            .field("insert_before_this", &self.insert_before_this)
            .finish()
    }
}

/// Allows converting some view into [`AnyView`].
pub trait IntoAny {
    /// Converts the view into a type-erased [`AnyView`].
    fn into_any(self) -> AnyView;
}

/// A more general version of [`IntoAny`] that allows into [`AnyView`],
/// but also erasing other types that don't implement [`RenderHtml`] like routing.
pub trait IntoMaybeErased {
    /// The type of the output.
    type Output: IntoMaybeErased;

    /// Converts the view into a type-erased view if in erased mode.
    fn into_maybe_erased(self) -> Self::Output;
}

impl<T> IntoMaybeErased for T
where
    T: RenderHtml,
{
    #[cfg(not(erase_components))]
    type Output = Self;

    #[cfg(erase_components)]
    type Output = AnyView;

    fn into_maybe_erased(self) -> Self::Output {
        #[cfg(not(erase_components))]
        {
            self
        }
        #[cfg(erase_components)]
        {
            self.into_owned().into_any()
        }
    }
}

fn mount_any<T>(
    state: &mut ErasedLocal,
    parent: &crate::renderer::types::Element,
    marker: Option<&crate::renderer::types::Node>,
) where
    T: Render,
    T::State: 'static,
{
    state.get_mut::<T::State>().mount(parent, marker)
}

fn unmount_any<T>(state: &mut ErasedLocal)
where
    T: Render,
    T::State: 'static,
{
    state.get_mut::<T::State>().unmount();
}

fn insert_before_this<T>(state: &ErasedLocal, child: &mut dyn Mountable) -> bool
where
    T: Render,
    T::State: 'static,
{
    state.get_ref::<T::State>().insert_before_this(child)
}

fn elements<T>(state: &ErasedLocal) -> Vec<crate::renderer::types::Element>
where
    T: Render,
    T::State: 'static,
{
    state.get_ref::<T::State>().elements()
}

impl<T> IntoAny for T
where
    T: Send,
    T: RenderHtml,
{
    fn into_any(self) -> AnyView {
        fn build<T: RenderHtml + 'static>(value: Erased) -> AnyViewState {
            let state = ErasedLocal::new(value.into_inner::<T>().build());
            let placeholder = (!T::EXISTS).then(Rndr::create_placeholder);
            AnyViewState {
                type_id: TypeId::of::<T>(),
                state,
                mount: mount_any::<T>,
                unmount: unmount_any::<T>,
                insert_before_this: insert_before_this::<T>,
                elements: elements::<T>,
                placeholder,
            }
        }

        fn rebuild<T: RenderHtml + 'static>(value: Erased, state: &mut AnyViewState) {
            let state = state.state.get_mut::<<T as Render>::State>();
            value.into_inner::<T>().rebuild(state);
        }

        let value = self.into_owned();
        AnyView {
            type_id: TypeId::of::<T::Owned>(),
            build: build::<T::Owned>,
            rebuild: rebuild::<T::Owned>,

            value: Erased::new(value),
        }
    }
}

impl Render for AnyView {
    type State = AnyViewState;

    fn build(self) -> Self::State {
        (self.build)(self.value)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self.type_id == state.type_id {
            (self.rebuild)(self.value, state)
        } else {
            let mut new = self.build();
            if let Some(placeholder) = &mut state.placeholder {
                placeholder.insert_before_this(&mut new);
                placeholder.unmount();
            } else {
                state.insert_before_this(&mut new);
            }
            state.unmount();
            *state = new;
        }
    }
}

impl AddAnyAttr for AnyView {
    type Output<SomeNewAttr: Attribute> = AnyViewWithAttrs;

    #[allow(unused_variables)]
    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        AnyViewWithAttrs {
            view: self,
            attrs: vec![attr.into_cloneable_owned().into_any_attr()],
        }
    }
}

impl RenderHtml for AnyView {
    type Owned = Self;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        {
            _ = cursor;
            _ = position;
            panic!("Type-erased views cannot be used in a static browser template.");
        }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl Mountable for AnyViewState {
    fn unmount(&mut self) {
        (self.unmount)(&mut self.state);
        if let Some(placeholder) = &mut self.placeholder {
            placeholder.unmount();
        }
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        (self.mount)(&mut self.state, parent, marker);
        if let Some(placeholder) = &mut self.placeholder {
            placeholder.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        let before_view = (self.insert_before_this)(&self.state, child);
        if before_view {
            return true;
        }

        if let Some(placeholder) = &self.placeholder {
            placeholder.insert_before_this(child)
        } else {
            false
        }
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        (self.elements)(&self.state)
    }
}

/// wip
pub struct AnyViewWithAttrs {
    view: AnyView,
    attrs: Vec<AnyAttribute>,
}

impl Render for AnyViewWithAttrs {
    type State = AnyViewWithAttrsState;

    fn build(self) -> Self::State {
        let view = self.view.build();
        let elements = view.elements();
        let mut attrs = Vec::with_capacity(elements.len() * self.attrs.len());
        for attr in self.attrs {
            for el in &elements {
                attrs.push(attr.clone().build(el))
            }
        }
        AnyViewWithAttrsState { view, attrs }
    }

    fn rebuild(self, state: &mut Self::State) {
        self.view.rebuild(&mut state.view);

        // at this point, we have rebuilt the inner view
        // now we need to update attributes that were spread onto this
        // this approach is not ideal, but it avoids two edge cases:
        // 1) merging attributes from two unrelated views (https://github.com/leptos-rs/leptos/issues/4268)
        // 2) failing to re-create attributes from the same view (https://github.com/leptos-rs/leptos/issues/4512)
        for element in state.elements() {
            // first, remove the previous set of attributes
            self.attrs
                .clone()
                .rebuild(&mut (element.clone(), Vec::new()));
            // then, add the new set of attributes
            self.attrs.clone().build(&element);
        }
    }
}

impl RenderHtml for AnyViewWithAttrs {
    type Owned = Self;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let view = self.view.hydrate(cursor, position);
        let elements = view.elements();
        let mut attrs = Vec::with_capacity(elements.len() * self.attrs.len());
        for attr in self.attrs {
            for el in &elements {
                attrs.push(attr.clone().hydrate(el));
            }
        }
        AnyViewWithAttrsState { view, attrs }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl AddAnyAttr for AnyViewWithAttrs {
    type Output<SomeNewAttr: Attribute> = AnyViewWithAttrs;

    fn add_any_attr<NewAttr: Attribute>(mut self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self.attrs.push(attr.into_cloneable_owned().into_any_attr());
        self
    }
}

/// State for any view with attributes spread onto it.
pub struct AnyViewWithAttrsState {
    view: AnyViewState,
    #[allow(dead_code)] // keeps attribute states alive until dropped
    attrs: Vec<AnyAttributeState>,
}

impl Mountable for AnyViewWithAttrsState {
    fn unmount(&mut self) {
        self.view.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.view.mount(parent, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.view.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.view.elements()
    }
}
