use super::attribute::{
    maybe_next_attr_erasure_macros::next_attr_output_type, Attribute, NextAttribute,
};

use crate::{
    html::attribute::{maybe_next_attr_erasure_macros::next_attr_combine, NamedAttributeKey},
    renderer::{dom::CssStyleDeclaration, Rndr},
    view::{Position, ToTemplate},
};
use std::sync::Arc;

/// Returns an [`Attribute`] that will add to an element's CSS styles.
#[inline(always)]
pub fn style<S>(style: S) -> Style<S>
where
    S: IntoStyle,
{
    Style { style }
}

/// An [`Attribute`] that will add to an element's CSS styles.
#[derive(Debug)]
pub struct Style<S> {
    style: S,
}

impl<S> Clone for Style<S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            style: self.style.clone(),
        }
    }
}

impl<S> Attribute for Style<S>
where
    S: IntoStyle,
{
    type State = S::State;
    type Cloneable = Style<S::Cloneable>;
    type CloneableOwned = Style<S::CloneableOwned>;

    // TODO

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        self.style.hydrate(el)
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        self.style.build(el)
    }

    fn rebuild(self, state: &mut Self::State) {
        self.style.rebuild(state)
    }

    fn into_cloneable(self) -> Self::Cloneable {
        Style {
            style: self.style.into_cloneable(),
        }
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        Style {
            style: self.style.into_cloneable_owned(),
        }
    }

    fn keys(&self) -> Vec<NamedAttributeKey> {
        vec![NamedAttributeKey::Attribute("style".into())]
    }
}

impl<S> NextAttribute for Style<S>
where
    S: IntoStyle,
{
    next_attr_output_type!(Self, NewAttr);

    fn add_any_attr<NewAttr: Attribute>(self, new_attr: NewAttr) -> Self::Output<NewAttr> {
        next_attr_combine!(self, new_attr)
    }
}

impl<S> ToTemplate for Style<S>
where
    S: IntoStyle,
{
    fn to_template(
        _buf: &mut String,
        _style: &mut String,
        _class: &mut String,
        _inner_html: &mut String,
        _position: &mut Position,
    ) {
        // TODO: should there be some templating for static styles?
    }
}

/// Any type that can be added to the `style` attribute or set as a style in
/// the [`CssStyleDeclaration`](web_sys::CssStyleDeclaration).
///
/// This could be a plain string, or a property name-value pair.
pub trait IntoStyle: Send {
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: IntoStyle + Clone;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: IntoStyle + Clone + 'static;

    /// Restores a dynamic style on a clone of a browser template.
    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State
    where
        Self: Sized,
    {
        self.build(el)
    }

    /// Adds this style to the element during client-side rendering.
    fn build(self, el: &crate::renderer::types::Element) -> Self::State;

    /// Updates the value.
    fn rebuild(self, state: &mut Self::State);

    /// Converts this to a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this to a cloneable, owned type.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;

    /// Reset the styling to the state before this style was added.
    fn reset(state: &mut Self::State);
}

impl<T: IntoStyle> IntoStyle for Option<T> {
    type State = (crate::renderer::types::Element, Option<T::State>);
    type Cloneable = Option<T::Cloneable>;
    type CloneableOwned = Option<T::CloneableOwned>;

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

impl<'a> IntoStyle for &'a str {
    type State = (crate::renderer::types::Element, &'a str);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "style", self);
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
        Rndr::remove_attribute(el, "style");
    }
}

impl IntoStyle for Arc<str> {
    type State = (crate::renderer::types::Element, Arc<str>);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "style", &self);
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
        Rndr::remove_attribute(el, "style");
    }
}

impl IntoStyle for String {
    type State = (crate::renderer::types::Element, String);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        Rndr::set_attribute(el, "style", &self);
        (el.clone(), self)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, prev) = state;
        if self != *prev {
            Rndr::set_attribute(el, "style", &self);
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
        Rndr::remove_attribute(el, "style");
    }
}

/// Any type that can be used to set an individual style in the
/// [`CssStyleDeclaration`](web_sys::CssStyleDeclaration).
///
/// This is the value in a `(name, value)` tuple that implements [`IntoStyle`].
pub trait IntoStyleValue: Send {
    /// The view state retained between building and rebuilding.
    type State;
    /// An equivalent value that can be cloned.
    type Cloneable: Clone + IntoStyleValue;
    /// An equivalent value that can be cloned and is `'static`.
    type CloneableOwned: Clone + IntoStyleValue + 'static;

    /// Adds this style to the element during client-side rendering.
    fn build(self, style: &CssStyleDeclaration, name: &str) -> Self::State;

    /// Updates the value.
    fn rebuild(self, style: &CssStyleDeclaration, name: &str, state: &mut Self::State);

    /// Restores a dynamic style on a clone of a browser template.
    fn hydrate(self, style: &CssStyleDeclaration, name: &str) -> Self::State
    where
        Self: Sized,
    {
        self.build(style, name)
    }

    /// Converts this to a cloneable type.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this to a cloneable, owned type.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

impl<K, V> IntoStyle for (K, V)
where
    K: AsRef<str> + Clone + Send + 'static,
    V: IntoStyleValue,
{
    type State = (crate::renderer::types::CssStyleDeclaration, K, V::State);
    type Cloneable = (K, V::Cloneable);
    type CloneableOwned = (K, V::CloneableOwned);

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, value) = self;
        let style = Rndr::style(el);
        let state = value.build(&style, name.as_ref());
        (style, name, state)
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, value) = self;
        // state.1 was the previous name, theoretically the css name could be changed:
        if name.as_ref() != state.1.as_ref() {
            <Self as IntoStyle>::reset(state);
            state.2 = value.build(&state.0, name.as_ref());
        } else {
            value.rebuild(&state.0, name.as_ref(), &mut state.2);
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1.into_cloneable_owned())
    }

    /// Reset the renderer to the state before this style was added.
    fn reset(state: &mut Self::State) {
        let (style, name, _value) = state;
        Rndr::remove_css_property(style, name.as_ref());
    }
}

macro_rules! impl_style_value {
    ($ty:ty) => {
        impl IntoStyleValue for $ty {
            type State = Self;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn build(self, style: &CssStyleDeclaration, name: &str) -> Self::State {
                Rndr::set_css_property(style, name, &self);
                self
            }

            fn rebuild(self, style: &CssStyleDeclaration, name: &str, state: &mut Self::State) {
                if &self != &*state {
                    Rndr::set_css_property(style, name, &self);
                }
                *state = self;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }
        }

        impl IntoStyleValue for Option<$ty> {
            type State = Self;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn build(self, style: &CssStyleDeclaration, name: &str) -> Self::State {
                if let Some(value) = &self {
                    Rndr::set_css_property(style, name, &value);
                }
                self
            }

            fn rebuild(self, style: &CssStyleDeclaration, name: &str, state: &mut Self::State) {
                match (&state, &self) {
                    (None, None) => {}
                    (Some(_), None) => Rndr::remove_css_property(style, name),
                    (None, Some(value)) => Rndr::set_css_property(style, name, &value),
                    (Some(old), Some(new)) => {
                        if new != &*old {
                            Rndr::set_css_property(style, name, &new);
                        }
                    }
                }
                *state = self;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }
        }
    };
}

impl_style_value!(&'static str);
impl_style_value!(Arc<str>);
impl_style_value!(String);
#[cfg(feature = "oco")]
impl_style_value!(oco_ref::Oco<'static, str>);
