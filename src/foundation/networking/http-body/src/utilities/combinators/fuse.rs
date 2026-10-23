use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Body, Frame, SizeHint};

/// A "fused" [`Body`].
///
/// This [`Body`] yields `Poll::Ready(None)` forever after the underlying body yields
/// `Poll::Ready(None)`, or an error `Poll::Ready(Some(Err(_)))`, once.
///
/// Bodies should ideally continue to return `Poll::Ready(None)` indefinitely after the end of
/// the stream is reached. [`Fuse<B>`] avoids polling its underlying body `B` further after the
/// underlying stream as ended, which can be useful for implementation that cannot uphold this
/// guarantee.
///
/// This is akin to the functionality that [`std::iter::Iterator::fuse()`] provides for
/// [`Iterator`][std::iter::Iterator]s.
#[derive(Debug)]
pub struct Fuse<B> {
    inner: Option<B>,
}

impl<B> Fuse<B>
where
    B: Body,
{
    /// Returns a fused body.
    pub fn new(body: B) -> Self {
        Self {
            inner: if body.is_end_stream() {
                None
            } else {
                Some(body)
            },
        }
    }
}

impl<B> Body for Fuse<B>
where
    B: Body + Unpin,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<B::Data>, B::Error>>> {
        let Self { inner } = self.get_mut();

        let poll = inner
            .as_mut()
            .map(|mut inner| match Pin::new(&mut inner).poll_frame(cx) {
                frame @ Poll::Ready(Some(Ok(_))) => (frame, inner.is_end_stream()),
                end @ Poll::Ready(Some(Err(_)) | None) => (end, true),
                poll @ Poll::Pending => (poll, false),
            });

        if let Some((frame, eos)) = poll {
            eos.then(|| inner.take());
            frame
        } else {
            Poll::Ready(None)
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner
            .as_ref()
            .map(B::size_hint)
            .unwrap_or_else(|| SizeHint::with_exact(0))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/http-body/utilities/unit/src/combinators/fuse/tests_82.rs"));
