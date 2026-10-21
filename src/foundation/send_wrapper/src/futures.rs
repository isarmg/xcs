//! [`Future`] and [`Stream`] support for [`SendWrapper`].
use std::{
	future::Future,
	ops::{Deref as _, DerefMut as _},
	pin::Pin,
	task,
};

use futures_core::Stream;

use crate::SendWrapper;

impl<F: Future> Future for SendWrapper<F> {
	type Output = F::Output;

	/// Polls this [`SendWrapper`] [`Future`].
	///
	/// # Panics
	///
	/// Polling panics if it is done from a different thread than the one the [`SendWrapper`]
	/// instance has been created with.
	#[track_caller]
	fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
		self.assert_valid_for_poll();
		// This is safe as `SendWrapper` itself points to the inner `Future`.
		// So, as long as `SendWrapper` is pinned, the inner `Future` is pinned too.
		unsafe { self.map_unchecked_mut(Self::deref_mut) }.poll(cx)
	}
}

impl<S: Stream> Stream for SendWrapper<S> {
	type Item = S::Item;

	/// Polls this [`SendWrapper`] [`Stream`].
	///
	/// # Panics
	///
	/// Polling panics if it is done from a different thread than the one the [`SendWrapper`]
	/// instance has been created with.
	#[track_caller]
	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut task::Context<'_>,
	) -> task::Poll<Option<Self::Item>> {
		self.assert_valid_for_poll();
		// This is safe as `SendWrapper` itself points to the inner `Stream`.
		// So, as long as `SendWrapper` is pinned, the inner `Stream` is pinned too.
		unsafe { self.map_unchecked_mut(Self::deref_mut) }.poll_next(cx)
	}

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.deref().size_hint()
	}
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/send_wrapper/unit/src/futures/tests_57.rs"));
