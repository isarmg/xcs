use crate::io_traits::AsyncWrite;
use crate::io_traits::IoSlice;
use crate::traits::future::Future;
use crate::traits::ready;
use core::task::{Context, Poll};
use std::io;
use std::pin::Pin;

/// Future for the
/// [`write_all_vectored`](super::AsyncWriteExt::write_all_vectored) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WriteAllVectored<'a, W: ?Sized + Unpin> {
    writer: &'a mut W,
    bufs: &'a mut [IoSlice<'a>],
}

impl<W: ?Sized + Unpin> Unpin for WriteAllVectored<'_, W> {}

impl<'a, W: AsyncWrite + ?Sized + Unpin> WriteAllVectored<'a, W> {
    pub(super) fn new(writer: &'a mut W, mut bufs: &'a mut [IoSlice<'a>]) -> Self {
        IoSlice::advance_slices(&mut bufs, 0);
        Self { writer, bufs }
    }
}

impl<W: AsyncWrite + ?Sized + Unpin> Future for WriteAllVectored<'_, W> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = &mut *self;
        while !this.bufs.is_empty() {
            let n = ready!(Pin::new(&mut this.writer).poll_write_vectored(cx, this.bufs))?;
            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            } else {
                IoSlice::advance_slices(&mut this.bufs, n);
            }
        }

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/futures/util/unit/src/io/write_all_vectored/tests_45.rs"
));
