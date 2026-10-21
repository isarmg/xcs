use super::{ElementWithChildren, HtmlElement};
use crate::{
    html::attribute::{
        maybe_next_attr_erasure_macros::{next_attr_combine, next_attr_output_type},
        Attribute, NamedAttributeKey, NextAttribute,
    },
    renderer::Rndr,
    view::add_attr::AddAnyAttr,
};
use std::sync::Arc;

/// Returns an [`Attribute`] that sets the inner HTML of an element.
///
/// No children should be given to this element, as this HTML will be used instead.
///
/// # Security
/// Be very careful when using this method. Always remember to
/// sanitize the input to avoid a cross-site scripting (XSS)
/// vulnerability.
#[inline(always)]
pub fn inner_html<T>(value: T) -> InnerHtml<T>
where
    T: InnerHtmlValue,
{
    InnerHtml { value }
}

/// Sets the inner HTML of an element.
#[derive(Debug)]
pub struct InnerHtml<T> {
    value: T,
}

impl<T> Clone for InnerHtml<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}

impl<T> Attribute for InnerHtml<T>
where
    T: InnerHtmlValue,
{
    type State = T::State;
    type Cloneable = InnerHtml<T::Cloneable>;
    type CloneableOwned = InnerHtml<T::CloneableOwned>;

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        self.value.hydrate(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.value.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.value.rebuild(state);
    }

    fn into_cloneable(self) -> Self::Cloneable {
        InnerHtml {
            value: self.value.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        InnerHtml {
            value: self.value.into_cloneable_owned(),
        }
    }

    fn keys(&self) -> Vec<NamedAttributeKey> {
        vec![NamedAttributeKey::InnerHtml]
    }
}

impl<T> NextAttribute for InnerHtml<T>
where
    T: InnerHtmlValue,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(self, new_attr: NewAttr) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

/// Sets the inner HTML of an element.
pub trait InnerHtmlAttribute<T>
where
    T: InnerHtmlValue,

    Self: Sized + AddAnyAttr,
{
    /// Sets the inner HTML of this element.
    ///
    /// No children should be given to this element, as this HTML will be used instead.
    ///
    /// # Security
    /// Be very careful when using this method. Always remember to
    /// sanitize the input to avoid a cross-site scripting (XSS)
    /// vulnerability.
    fn inner_html(self, value: T) -> <Self as AddAnyAttr>::Output<InnerHtml<T>> {
        self.add_any_attr(inner_html(value))
    }
}

impl<T, E, At> InnerHtmlAttribute<T> for HtmlElement<E, At, ()>
where
    Self: AddAnyAttr,
    E: ElementWithChildren,
    At: Attribute,
    T: InnerHtmlValue,
{
    fn inner_html(self, value: T) -> <Self as AddAnyAttr>::Output<InnerHtml<T>> {
        self.add_any_attr(inner_html(value))
    }
}

/// A possible value for [`InnerHtml`].
pub trait InnerHtmlValue: Send {
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: InnerHtmlValue + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: InnerHtmlValue + Clone + 'static;

    /// Renders the class to HTML for a `<template>`.
    fn to_template(buf: &mut String);

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
}

impl InnerHtmlValue for String {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn to_template(_buf: &mut String) {}

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            Rndr::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self.into()
    }
}

impl InnerHtmlValue for Arc<str> {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_template(_buf: &mut String) {}

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_inner_html(el, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_inner_html(el, &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            Rndr::set_inner_html(&state.0, &self);
            state.1 = self;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::Cloneable {
        self
    }
}

impl InnerHtmlValue for &str {
    type State = (crate::renderer::types::Element, Self);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn to_template(_buf: &mut String) {}

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            Rndr::set_inner_html(el, self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_inner_html(el, self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        if self != state.1 {
            Rndr::set_inner_html(&state.0, self);
            state.1 = self;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<T> InnerHtmlValue for Option<T>
where
    T: InnerHtmlValue,
{
    type State = (crate::renderer::types::Element, Option<T::State>);
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

    fn to_template(_buf: &mut String) {}

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        (el.clone(), self.map(|n| n.hydrate(el)))
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        (el.clone(), self.map(|n| n.build(el)))
    }

    fn rebuild(self, state: &mut Self::State) {
        let new_state = match (self, &mut state.1) {
            (None, None) => None,
            (None, Some(_)) => {
                Rndr::set_inner_html(&state.0, "");
                Some(None)
            }
            (Some(new), None) => Some(Some(new.build(&state.0))),
            (Some(new), Some(state)) => {
                new.rebuild(state);
                None
            }
        };
        if let Some(new_state) = new_state {
            state.1 = new_state;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|inner| inner.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|inner| inner.into_cloneable_owned())
    }
}
