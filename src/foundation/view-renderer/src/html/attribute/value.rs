use crate::renderer::Rndr;
use std::{
    borrow::Cow,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    sync::Arc,
};

/// Declares that this type can be converted into some other type, which is a valid attribute value.
pub trait IntoAttributeValue {
    /// The attribute value into which this type can be converted.
    type Output;

    /// Consumes this value, transforming it into an attribute value.
    fn into_attribute_value(self) -> Self::Output;
}

impl<T> IntoAttributeValue for T
where
    T: AttributeValue,
{
    type Output = Self;

    fn into_attribute_value(self) -> Self::Output {
        self
    }
}

/// A possible value for an HTML attribute.
pub trait AttributeValue: Send {
    /// The state that should be retained between building and rebuilding.
    type State;

    /// A version of the value that can be cloned. This can be the same type, or a
    /// reference-counted type. Generally speaking, this does *not* need to refer to the same data,
    /// but should behave in the same way. So for example, making an event handler cloneable should
    /// probably make it reference-counted (so that a `FnMut()` continues mutating the same
    /// closure), but making a `String` cloneable does not necessarily need to make it an
    /// `Arc<str>`, as two different clones of a `String` will still have the same value.
    type Cloneable: AttributeValue + Clone;

    /// A cloneable type that is also `'static`. This is used for spreading across types when the
    /// spreadable attribute needs to be owned. In some cases (`&'a str` to `Arc<str>`, etc.) the owned
    /// cloneable type has worse performance than the cloneable type, so they are separate.
    type CloneableOwned: AttributeValue + Clone + 'static;

    /// Renders the attribute value to HTML for a `<template>`.
    fn to_template(key: &str, buf: &mut String);

    /// Adds interactivity as necessary, given DOM nodes that were created from HTML that has
    /// either been rendered on the server, or cloned for a `<template>`.
    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State;

    /// Adds this attribute to the element during client-side rendering.
    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State;

    /// Applies a new value for the attribute.
    fn rebuild(self, key: &str, state: &mut Self::State);

    /// Converts this attribute into an equivalent that can be cloned.
    fn into_cloneable(self) -> Self::Cloneable;

    /// Converts this attributes into an equivalent that can be cloned and is `'static`.
    fn into_cloneable_owned(self) -> Self::CloneableOwned;
}

impl AttributeValue for () {
    type State = ();

    type Cloneable = ();
    type CloneableOwned = ();

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, _key: &str, _el: &crate::renderer::types::Element) {}

    fn build(self, _el: &crate::renderer::types::Element, _key: &str) -> Self::State {}

    fn rebuild(self, _key: &str, _state: &mut Self::State) {}

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<'a> AttributeValue for &'a str {
    type State = (crate::renderer::types::Element, &'a str);

    type Cloneable = &'a str;
    type CloneableOwned = Arc<str>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        // Restore the dynamic attribute on the browser template clone.
        {
            Rndr::set_attribute(el, key, self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        Rndr::set_attribute(el, key, self);
        (el.to_owned(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<'a> AttributeValue for Cow<'a, str> {
    type State = (crate::renderer::types::Element, Self);

    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        // Restore the dynamic attribute on the browser template clone.
        {
            Rndr::set_attribute(el, key, &self);
        }
        (el.clone(), self)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        Rndr::set_attribute(el, key, &self);
        (el.to_owned(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, &self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl<'a> AttributeValue for &'a String {
    type State = (crate::renderer::types::Element, &'a String);
    type Cloneable = Self;
    type CloneableOwned = Arc<str>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate(self.as_str(), key, el);
        (el, self)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        Rndr::set_attribute(el, key, self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.as_str().into()
    }
}

impl AttributeValue for String {
    type State = (crate::renderer::types::Element, String);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate(self.as_str(), key, el);
        (el, self)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        Rndr::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, &self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into()
    }
}

impl AttributeValue for Arc<str> {
    type State = (crate::renderer::types::Element, Arc<str>);
    type Cloneable = Arc<str>;
    type CloneableOwned = Arc<str>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        let (el, _) = <&str as AttributeValue>::hydrate(self.as_ref(), key, el);
        (el, self)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        Rndr::set_attribute(el, key, &self);
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            Rndr::set_attribute(el, key, &self);
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}
// TODO impl AttributeValue for Rc<str> and Arc<str> too

impl AttributeValue for bool {
    type State = (crate::renderer::types::Element, bool);
    type Cloneable = Self;
    type CloneableOwned = Self;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        self.build(el, key)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        if self {
            Rndr::set_attribute(el, key, "");
        }
        (el.clone(), self)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev_value) = state;
        if self != *prev_value {
            if self {
                Rndr::set_attribute(el, key, "");
            } else {
                Rndr::remove_attribute(el, key);
            }
        }
        *prev_value = self;
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }
}

impl<V> AttributeValue for Option<V>
where
    V: AttributeValue,
{
    type State = (crate::renderer::types::Element, Option<V::State>);
    type Cloneable = Option<V::Cloneable>;
    type CloneableOwned = Option<V::CloneableOwned>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        let state = self.map(|v| v.hydrate(key, el));
        (el.clone(), state)
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        let el = el.clone();
        let v = self.map(|v| v.build(&el, key));
        (el, v)
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let (el, prev) = state;
        match (self, prev.as_mut()) {
            (None, None) => {}
            (None, Some(_)) => {
                Rndr::remove_attribute(el, key);
                *prev = None;
            }
            (Some(value), None) => {
                *prev = Some(value.build(el, key));
            }
            (Some(new), Some(old)) => {
                new.rebuild(key, old);
            }
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.map(|value| value.into_cloneable())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.map(|value| value.into_cloneable_owned())
    }
}

macro_rules! render_primitive {
  ($($child_type:ty),* $(,)?) => {
      $(
        impl AttributeValue for $child_type
        where

        {

            type State = (crate::renderer::types::Element, $child_type);
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn to_template(_key: &str, _buf: &mut String) {}

            fn hydrate(
                self,
                key: &str,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                // Restore dynamic attribute values on the cloned template.
                {
                    Rndr::set_attribute(el, key, &self.to_string());
                }
                (el.clone(), self)
            }

            fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
                Rndr::set_attribute(el, key, &self.to_string());
                (el.to_owned(), self)
            }

            fn rebuild(self, key: &str, state: &mut Self::State) {
                let (el, prev_value) = state;
                if self != *prev_value {
                    Rndr::set_attribute(el, key, &self.to_string());
                }
                *prev_value = self;
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }

        }
      )*
  }
}

render_primitive![
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    isize,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    char,
    IpAddr,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    Ipv4Addr,
    Ipv6Addr,
    NonZeroI8,
    NonZeroU8,
    NonZeroI16,
    NonZeroU16,
    NonZeroI32,
    NonZeroU32,
    NonZeroI64,
    NonZeroU64,
    NonZeroI128,
    NonZeroU128,
    NonZeroIsize,
    NonZeroUsize,
];
