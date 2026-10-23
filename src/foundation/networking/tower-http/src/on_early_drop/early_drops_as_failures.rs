//! Adapter that bridges early-drop events to [`trace::OnFailure`].
//!
//! [`trace::OnFailure`]: crate::trace::OnFailure

use crate::on_early_drop::failure::{BodyDropped, DroppedFailure, FutureDropped};
use crate::on_early_drop::traits::{OnBodyDrop, OnDropCallback, OnFutureDrop};
use crate::trace::OnFailure;
use http::{response, Request, StatusCode};
use std::time::Instant;
use tracing::Span;

/// Bridges early-drop events to [`trace::OnFailure`](crate::trace::OnFailure).
///
/// Each event is reported by invoking the wrapped hook with a
/// [`DroppedFailure`]: `Future` for future drops, `Body` for body drops
/// (carrying the emitted response status).
///
/// Latency is computed from the moment the hook is produced (either at
/// `Service::call` or at response-ready time). The captured span is
/// [`Span::current()`] at that same moment. To report events against the
/// request span used by [`TraceLayer`](crate::trace::TraceLayer), place
/// [`OnEarlyDropLayer`] inside `TraceLayer`.
///
/// See the [module docs](super) for the example.
///
/// [`OnEarlyDropLayer`]: super::OnEarlyDropLayer
#[derive(Debug, Clone, Copy)]
pub struct EarlyDropsAsFailures<F> {
    on_failure: F,
}

impl<F> EarlyDropsAsFailures<F> {
    /// Wrap an [`OnFailure`] implementation.
    pub fn new(on_failure: F) -> Self {
        Self { on_failure }
    }
}

/// Future-drop callback produced by [`EarlyDropsAsFailures`].
pub struct FutureDropFailureCallback<F> {
    start: Instant,
    on_failure: F,
    span: Span,
}

impl<F> OnDropCallback for FutureDropFailureCallback<F>
where
    F: OnFailure<DroppedFailure> + Send + 'static,
{
    fn on_drop(mut self) {
        let latency = self.start.elapsed();
        let _entered = self.span.enter();
        self.on_failure
            .on_failure(DroppedFailure::Future(FutureDropped), latency, &self.span);
    }
}

/// Intermediate produced by [`OnBodyDrop::make_at_call`] for
/// [`EarlyDropsAsFailures`], carrying state forward to
/// [`OnBodyDrop::make_at_response`].
pub struct PreResponseBodyDropCallback<F> {
    start: Instant,
    on_failure: F,
    span: Span,
}

/// Body-drop callback produced by [`EarlyDropsAsFailures`].
pub struct BodyDropFailureCallback<F> {
    start: Instant,
    on_failure: F,
    span: Span,
    status: StatusCode,
}

impl<F> OnDropCallback for BodyDropFailureCallback<F>
where
    F: OnFailure<DroppedFailure> + Send + 'static,
{
    fn on_drop(mut self) {
        let latency = self.start.elapsed();
        let _entered = self.span.enter();
        self.on_failure.on_failure(
            DroppedFailure::Body(BodyDropped {
                status: self.status,
            }),
            latency,
            &self.span,
        );
    }
}

impl<F, ReqB> OnFutureDrop<ReqB> for EarlyDropsAsFailures<F>
where
    F: OnFailure<DroppedFailure> + Clone + Send + 'static,
{
    type Callback = FutureDropFailureCallback<F>;

    fn make(&mut self, _request: &Request<ReqB>) -> Self::Callback {
        FutureDropFailureCallback {
            start: Instant::now(),
            on_failure: self.on_failure.clone(),
            span: Span::current(),
        }
    }
}

impl<F, ReqB> OnBodyDrop<ReqB> for EarlyDropsAsFailures<F>
where
    F: OnFailure<DroppedFailure> + Clone + Send + 'static,
{
    type Intermediate = PreResponseBodyDropCallback<F>;
    type Callback = BodyDropFailureCallback<F>;

    fn make_at_call(&mut self, _request: &Request<ReqB>) -> Self::Intermediate {
        PreResponseBodyDropCallback {
            start: Instant::now(),
            on_failure: self.on_failure.clone(),
            span: Span::current(),
        }
    }

    fn make_at_response(
        &mut self,
        intermediate: Self::Intermediate,
        response_parts: &response::Parts,
    ) -> Self::Callback {
        BodyDropFailureCallback {
            start: intermediate.start,
            on_failure: intermediate.on_failure,
            span: intermediate.span,
            status: response_parts.status,
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tower-http/unit/src/on_early_drop/early_drops_as_failures/tests_136.rs"));
