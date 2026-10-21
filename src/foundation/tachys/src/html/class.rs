use super::attribute::{
    maybe_next_attr_erasure_macros::next_attr_output_type, Attribute, NamedAttributeKey,
    NextAttribute,
};
use crate::{
    html::attribute::maybe_next_attr_erasure_macros::next_attr_combine,
    renderer::Rndr,
    view::{Position, ToTemplate},
};
use std::{borrow::Cow, sync::Arc};

/// Adds a CSS class.
#[inline(always)]
pub fn class<C>(class: C) -> Class<C>
where
    C: IntoClass,
{
    Class { class }
}

/// A CSS class.
#[derive(Debug)]
pub struct Class<C> {
    class: C,
}

impl<C> Clone for Class<C>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            class: self.class.clone(),
        }
    }
}

impl<C> Attribute for Class<C>
where
    C: IntoClass,
{
    type State = C::State;
    type Cloneable = Class<C::Cloneable>;
    type CloneableOwned = Class<C::CloneableOwned>;

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        self.class.hydrate(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.class.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.class.rebuild(state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Class {
            class: self.class.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Class {
            class: self.class.into_cloneable_owned(),
        }
    }

    fn keys(&self) -> Vec<NamedAttributeKey> {
        vec![NamedAttributeKey::Attribute("class".into())]
    }
}

impl<C> NextAttribute for Class<C>
where
    C: IntoClass,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(self, new_attr: NewAttr) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

impl<C> ToTemplate for Class<C>
where
    C: IntoClass,
{
    const CLASS: &'static str = C::TEMPLATE;

    fn to_template(
        _buf: &mut String,
        class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        C::to_template(class);
    }
}

/// A possible value for a CSS class.
pub trait IntoClass: Send {
    /// The HTML that should be included in a `<template>`.
    const TEMPLATE: &'static str = "";

    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: IntoClass + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: IntoClass + Clone + 'static;

    /// Whether this class attribute should overwrite previous class values.
    /// Returns `true` for `class="..."` attributes, `false` for `class:name=value` directives.
    fn should_overwrite(&self) -> bool {
        false
    }

    /// Renders the class to HTML for a `<template>`.
    #[allow(unused)] // it's used with `nightly` feature
    fn to_template(class: &mut String) {}

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State;

    /// Adds this class to the element during client-side rendering.
    fn build(self, el: &crate::renderer::types::Element) -> Self::State;

    /// Updates the value.
    fn rebuild(self, state: &mut Self::State);

    /// Converts this to a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this to a cloneable, owned type.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;

    /// Reset the class list to the state before this class was added.
    fn reset(state: &mut Self::State);
}

impl<T: IntoClass> IntoClass for Option<T> {
    type State = (crate::renderer::types::Element, Option<T::State>);
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        if let Some(t) = self {
            (el.clone(), Some(t.hydrate(el)))
        } else {
            (el.clone(), None)
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        if let Some(t) = self {
            (el.clone(), Some(t.build(el)))
        } else {
            (el.clone(), None)
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let el = &state.0;
        let prev_state = &mut state.1;
        let maybe_next_t_state = match (prev_state.take(), self) {
            (Some(mut prev_t_state), None) => {
                T::reset(&mut prev_t_state);
                Some(None)
            }
            (None, Some(t)) => Some(Some(t.build(el))),
            (Some(mut prev_t_state), Some(t)) => {
                t.rebuild(&mut prev_t_state);
                Some(Some(prev_t_state))
            }
            (None, None) => Some(None),
        };
        if let Some(next_t_state) = maybe_next_t_state {
            state.1 = next_t_state;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|t| t.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|t| t.into_cloneable_owned())
    }

    fn reset(state: &mut Self::State) {
        if let Some(prev_t_state) = &mut state.1 {
            T::reset(prev_t_state);
        }
    }
}

impl IntoClass for &str {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn should_overwrite(&self) -> bool {
        true
    }

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_attribute(el, "class", self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for Cow<'_, str> {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn should_overwrite(&self) -> bool {
        true
    }

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for String {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn should_overwrite(&self) -> bool {
        true
    }

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for Arc<str> {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn should_overwrite(&self) -> bool {
        true
    }

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_attribute(el, "class", &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "class", &self);
        }
        *prev = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn reset(state: &mut Self::State) {
        let (el, _prev) = state;
        Rndr::remove_attribute(el, "class");
    }
}

impl IntoClass for (&'static str, bool) {
    type State = (crate::renderer::types::ClassList, bool, &'static str);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, include) = self;
        let class_list = Rndr::class_list(el);
        if !false && include {
            Rndr::add_class(&class_list, name);
        }
        (class_list, self.1, name)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, include) = self;
        let class_list = Rndr::class_list(el);
        if include {
            Rndr::add_class(&class_list, name);
        }
        (class_list, self.1, name)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, include) = self;
        let (class_list, prev_include, prev_name) = state;
        if name == *prev_name {
            if include != *prev_include {
                if include {
                    Rndr::add_class(class_list, name);
                } else {
                    Rndr::remove_class(class_list, name);
                }
            }
        } else {
            if *prev_include {
                Rndr::remove_class(class_list, prev_name);
            }
            if include {
                Rndr::add_class(class_list, name);
            }
        }
        *prev_include = include;
        *prev_name = name;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }

    fn reset(state: &mut Self::State) {
        let (class_list, _, name) = state;
        Rndr::remove_class(class_list, name);
    }
}

#[cfg(all(feature = "nightly", rustc_nightly))]
impl<const V: &'static str> IntoClass for crate::view::static_types::Static<V> {
    const TEMPLATE: &'static str = V;

    type State = ();
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_template(class: &mut String) {
        class.push_str(V);
    }

    fn hydrate(self, _el: &crate::renderer::types::Element) -> Self::State {}

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "class", V);
    }

    fn rebuild(self, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn reset(_state: &mut Self::State) {}
}
