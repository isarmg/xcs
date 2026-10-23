//! Runtime components for use with [`tracing`].
//!
//! This module provides [`Executor`] implementations that configure
//! instrumentation of spawned futures. These [`Executor`]s can propagate
//! tracing [`Span`]s to futures spawned onto the async runtime. See the
//! crate-level documentation of [`tracing`] for [more information] about spans.
//!
//! # Choosing an [`Executor`].
//!
//! Hyper spawns [`Future`]s onto an [`Executor`], to avoid tightly coupling
//! APIs to any particular async runtime. This includes background tasks that
//! might help service I/O for the lifetime of a connection, for example.
//!
//! Some [`Subscriber`][tracing::subscriber] implementations have different
//! semantics regarding the lifecycle of [`Span`]s. Integrations with
//! OpenTelemetry collectors, for example, might not emit the events within
//! the context of a span until it is closed. Conversely, subscribers that
//! print traces to the terminal may not have to contend with these details when
//! instrumenting long-lived tasks that run in the background.
//!
//! This module provides different executors to help pass tracing context in
//! the manner appropriate for your application. For most typical applications,
//! [`CurrentSpanExecutor<E>`] should suffice.
//!
//! # Examples
//!
//! Run spawned tasks within a provided span.
//!
//! ```
//! # #[cfg(feature = "tokio")]
//! # {
//! use http_transport::adapters::rt::{TokioExecutor, WithSpanExecutor};
//!
//! let span = tracing::info_span!("example");
//! let executor = WithSpanExecutor::new(TokioExecutor::new(), span);
//! # }
//! ```
//!
//! Run spawned tasks within the current span when [`Executor::execute()`] is
//! called.
//!
//! ```
//! # #[cfg(feature = "tokio")]
//! # {
//! use http_transport::adapters::rt::{TokioExecutor, CurrentSpanExecutor};
//!
//! let executor = CurrentSpanExecutor::new(TokioExecutor::new());
//! # }
//! ```
//!
//! Run spawned tasks within distinct spans that are marked as following from
//! the active span when [`Executor::execute()`] is called.
//!
//! ```
//! # #[cfg(feature = "tokio")]
//! # {
//! use http_transport::adapters::rt::{MkSpanExecutor, TokioExecutor};
//! use tracing::{info_span, Span};
//!
//! let mk = || {
//!     let span = info_span!("example");
//!     span.follows_from(Span::current());
//!     span
//! };
//! let executor = MkSpanExecutor::new(TokioExecutor::new(), mk);
//! # }
//! ```
//!
//! [more information]: tracing#spans-1

use crate::rt::Executor;
use tracing::{
    instrument::{Instrument, Instrumented},
    Span,
};

/// An executor that propagates the current tracing span to its futures.
///
/// The span is captured when [`execute`](Executor::execute) is called, and is
/// entered each time the future is polled or dropped. Execution is delegated to
/// the wrapped executor, without requiring a particular runtime.
///
/// Requires the `tracing` feature.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "tokio")]
/// # {
/// use http_transport::adapters::rt::{TokioExecutor, CurrentSpanExecutor};
///
/// let executor = CurrentSpanExecutor::new(TokioExecutor::new());
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct CurrentSpanExecutor<E> {
    inner: E,
}

/// An executor that propagates a provided tracing span to its futures.
///
/// The span provided to this executor is entered each time the future is
/// polled or dropped. Execution is delegated to the wrapped executor, without
/// requiring a particular runtime.
///
/// Requires the `tracing` feature.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "tokio")]
/// # {
/// use http_transport::adapters::rt::{TokioExecutor, WithSpanExecutor};
///
/// let span = tracing::info_span!("example");
/// let executor = WithSpanExecutor::new(TokioExecutor::new(), span);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct WithSpanExecutor<E> {
    inner: E,
    span: Span,
}

/// An executor that uses a callback to propagate tracing span to its futures.
///
/// The callback is invoked each time a future is spawned, creating a span that
/// will be entered each time that future is polled or dropped. Execution is
/// delegated to the wrapped executor, without requiring a particular runtime.
///
/// Requires the `tracing` feature.
///
/// # Example
///
/// Spawned tasks can be marked as "following from" the execution context.
///
/// See [`tracing::Span::follows_from()`] for more information about
/// indicating causal relationships between spans.
///
/// ```
/// # #[cfg(feature = "tokio")]
/// # {
/// use http_transport::adapters::rt::{MkSpanExecutor, TokioExecutor};
/// use tracing::{info_span, Span};
///
/// let mk = || {
///     let span = info_span!("example");
///     span.follows_from(Span::current());
///     span
/// };
/// let executor = MkSpanExecutor::new(TokioExecutor::new(), mk);
/// # }
/// ```
///
/// Spawned tasks can be marked as children of the execution context.
///
/// ```
/// # #[cfg(feature = "tokio")]
/// # {
/// use http_transport::adapters::rt::{MkSpanExecutor, TokioExecutor};
/// use tracing::{info_span, Span};
///
/// let mk = || info_span!(parent: Span::current(), "example");
/// let executor = MkSpanExecutor::new(TokioExecutor::new(), mk);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct MkSpanExecutor<E, F> {
    inner: E,
    mk: F,
}

// ===== impl CurrentSpanExecutor =====

impl<E> CurrentSpanExecutor<E> {
    /// Wrap an executor to propagate the current tracing span to its futures.
    pub fn new(inner: E) -> Self {
        Self { inner }
    }
}

impl<E, F> Executor<F> for CurrentSpanExecutor<E>
where
    E: Executor<Instrumented<F>>,
    F: Future,
{
    fn execute(&self, future: F) {
        self.inner.execute(future.in_current_span());
    }
}

// ===== impl WithSpanExecutor =====

impl<E> WithSpanExecutor<E> {
    /// Wrap an executor to propagate the provided tracing span to its futures.
    pub fn new(inner: E, span: Span) -> Self {
        Self { inner, span }
    }

    /// Wrap an executor to propagate the current tracing span to its futures.
    ///
    /// This will instrument futures with the span that is active at the call-site of _this_
    /// function. Use [`CurrentSpanExecutor<E>`] if you would prefer to propagate the current span
    /// when [`Executor::execute()`] is called, rather than span that is active when initializating
    /// the executor.
    pub fn current(inner: E) -> Self {
        Self {
            inner,
            span: Span::current(),
        }
    }
}

impl<E, F> Executor<F> for WithSpanExecutor<E>
where
    E: Executor<Instrumented<F>>,
    F: Future,
{
    fn execute(&self, future: F) {
        self.inner.execute(future.instrument(self.span.clone()));
    }
}

// ===== impl MkSpanExecutor =====

impl<E, F> MkSpanExecutor<E, F> {
    /// Wrap an executor that creates new spans to instrument spawned futures.
    pub fn new(inner: E, mk: F) -> Self {
        Self { inner, mk }
    }
}

impl<E, F, Fut> Executor<Fut> for MkSpanExecutor<E, F>
where
    E: Executor<Instrumented<Fut>>,
    F: Fn() -> Span,
    Fut: Future,
{
    fn execute(&self, future: Fut) {
        let span = (self.mk)();
        self.inner.execute(future.instrument(span));
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/http-transport/adapters/unit/src/rt/tracing/tests_245.rs"
));

use std::future::Future;
