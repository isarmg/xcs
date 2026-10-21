//! `Span` and `Event` key-value data.
//!
//! Spans and events may be annotated with key-value data, referred to as _fields_.
//! These fields consist of a mapping from a key (corresponding to
//! a `&str` but represented internally as an array index) to a [`Value`].
//!
//! # `Value`s and `Subscriber`s
//!
//! `Subscriber`s consume `Value`s as fields attached to [span]s or [`Event`]s.
//! The set of field keys on a given span or event is defined on its [`Metadata`].
//! When a span is created, it provides [`Attributes`] to the `Subscriber`'s
//! [`new_span`] method, containing any fields whose values were provided when
//! the span was created; and may call the `Subscriber`'s [`record`] method
//! with additional [`Record`]s if values are added for more of its fields.
//! Similarly, the [`Event`] type passed to the subscriber's [`event`] method
//! will contain any fields attached to each event.
//!
//! `tracing` represents values as either one of a set of Rust primitives
//! (`i64`, `u64`, `f64`, `bool`, and `&str`) or using a `fmt::Display` or
//! `fmt::Debug` implementation. `Subscriber`s are provided these primitive
//! value types as `dyn Value` trait objects.
//!
//! These trait objects can be formatted using `fmt::Debug`, but may also be
//! recorded as typed data by calling the [`Value::record`] method on these
//! trait objects with a _visitor_ implementing the [`Visit`] trait. This trait
//! represents the behavior used to record values of various types. For example,
//! an implementation of `Visit` might record integers by incrementing counters
//! for their field names rather than printing them.
//!
//!
//! [span]: mod@crate::span
//! [`Event`]: crate::event::Event
//! [`Metadata`]: crate::Metadata
//! [`Attributes`]: crate::span::Attributes
//! [`Record`]: crate::span::Record
//! [`new_span`]: crate::Subscriber::new_span
//! [`record`]: crate::Subscriber::record
//! [`event`]: crate::Subscriber::event
pub use tracing::primitives::field::*;

use crate::Metadata;

/// Trait implemented to allow a type to be used as a field key.
///
/// <pre class="ignore" style="white-space:normal;font:inherit;">
/// <strong>Note</strong>: Although this is implemented for both the
/// <a href="./struct.Field.html"><code>Field</code></a> type <em>and</em> any
/// type that can be borrowed as an <code>&str</code>, only <code>Field</code>
/// allows <em>O</em>(1) access.
/// Indexing a field with a string results in an iterative search that performs
/// string comparisons. Thus, if possible, once the key for a field is known, it
/// should be used whenever possible.
/// </pre>
pub trait AsField: crate::sealed::Sealed {
    /// Attempts to convert `&self` into a `Field` with the specified `metadata`.
    ///
    /// If `metadata` defines this field, then the field is returned. Otherwise,
    /// this returns `None`.
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field>;
}

// ===== impl AsField =====

impl AsField for Field {
    #[inline]
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field> {
        if self.callsite() == metadata.callsite() {
            Some(self.clone())
        } else {
            None
        }
    }
}

impl AsField for &Field {
    #[inline]
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field> {
        if self.callsite() == metadata.callsite() {
            Some((*self).clone())
        } else {
            None
        }
    }
}

impl AsField for str {
    #[inline]
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field> {
        metadata.fields().field(&self)
    }
}

impl crate::sealed::Sealed for Field {}
impl crate::sealed::Sealed for &Field {}
impl crate::sealed::Sealed for str {}
