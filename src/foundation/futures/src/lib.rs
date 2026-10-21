//! Combinators and utilities for working with `Future`s, `Stream`s, `Sink`s,
//! and the `AsyncRead` and `AsyncWrite` traits.

#![no_std]
#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_assignments, unused_variables)
    )
))]
#![warn(missing_docs, unsafe_op_in_unsafe_fn)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "bilock", not(feature = "unstable")))]
compile_error!("The `bilock` feature requires the `unstable` feature as an explicit opt-in to unstable features");

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

// Macro re-exports

#[cfg(feature = "async-await")]
#[macro_use]
mod async_await;
#[cfg(feature = "async-await")]
#[doc(hidden)]
pub use self::async_await::*;

// Not public API.
#[doc(hidden)]
pub mod __private {
    pub use crate::*;
    pub use core::{
        option::Option::{self, None, Some},
        pin::Pin,
        result::Result::{Err, Ok},
    };

    #[cfg(feature = "async-await")]
    pub mod async_await {
        pub use crate::async_await::*;
    }
}

#[cfg(feature = "sink")]
#[cfg(feature = "utilities")]
macro_rules! delegate_sink {
    ($field:ident, $item:ty) => {
        fn poll_ready(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Result<(), Self::Error>> {
            self.project().$field.poll_ready(cx)
        }

        fn start_send(self: core::pin::Pin<&mut Self>, item: $item) -> Result<(), Self::Error> {
            self.project().$field.start_send(item)
        }

        fn poll_flush(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Result<(), Self::Error>> {
            self.project().$field.poll_flush(cx)
        }

        fn poll_close(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Result<(), Self::Error>> {
            self.project().$field.poll_close(cx)
        }
    };
}

#[cfg(feature = "utilities")]
macro_rules! delegate_future {
    ($field:ident) => {
        fn poll(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Self::Output> {
            self.project().$field.poll(cx)
        }
    };
}

#[cfg(feature = "utilities")]
macro_rules! delegate_stream {
    ($field:ident) => {
        fn poll_next(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Option<Self::Item>> {
            self.project().$field.poll_next(cx)
        }
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.$field.size_hint()
        }
    };
}

#[cfg(feature = "io")]
#[cfg(feature = "std")]
#[cfg(feature = "utilities")]
macro_rules! delegate_async_write {
    ($field:ident) => {
        fn poll_write(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
            buf: &[u8],
        ) -> core::task::Poll<std::io::Result<usize>> {
            self.project().$field.poll_write(cx, buf)
        }
        fn poll_write_vectored(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
            bufs: &[std::io::IoSlice<'_>],
        ) -> core::task::Poll<std::io::Result<usize>> {
            self.project().$field.poll_write_vectored(cx, bufs)
        }
        fn poll_flush(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<std::io::Result<()>> {
            self.project().$field.poll_flush(cx)
        }
        fn poll_close(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<std::io::Result<()>> {
            self.project().$field.poll_close(cx)
        }
    };
}

#[cfg(feature = "io")]
#[cfg(feature = "std")]
#[cfg(feature = "utilities")]
macro_rules! delegate_async_read {
    ($field:ident) => {
        fn poll_read(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
            buf: &mut [u8],
        ) -> core::task::Poll<std::io::Result<usize>> {
            self.project().$field.poll_read(cx, buf)
        }

        fn poll_read_vectored(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
            bufs: &mut [std::io::IoSliceMut<'_>],
        ) -> core::task::Poll<std::io::Result<usize>> {
            self.project().$field.poll_read_vectored(cx, bufs)
        }
    };
}

#[cfg(feature = "io")]
#[cfg(feature = "std")]
#[cfg(feature = "utilities")]
macro_rules! delegate_async_buf_read {
    ($field:ident) => {
        fn poll_fill_buf(
            self: core::pin::Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<std::io::Result<&[u8]>> {
            self.project().$field.poll_fill_buf(cx)
        }

        fn consume(self: core::pin::Pin<&mut Self>, amt: usize) {
            self.project().$field.consume(amt)
        }
    };
}

#[cfg(feature = "utilities")]
macro_rules! delegate_access_inner {
    ($field:ident, $inner:ty, ($($ind:tt)*)) => {
        /// Acquires a reference to the underlying sink or stream that this combinator is
        /// pulling from.
        pub fn get_ref(&self) -> &$inner {
            (&self.$field) $($ind get_ref())*
        }

        /// Acquires a mutable reference to the underlying sink or stream that this
        /// combinator is pulling from.
        ///
        /// Note that care must be taken to avoid tampering with the state of the
        /// sink or stream which may otherwise confuse this combinator.
        pub fn get_mut(&mut self) -> &mut $inner {
            (&mut self.$field) $($ind get_mut())*
        }

        /// Acquires a pinned mutable reference to the underlying sink or stream that this
        /// combinator is pulling from.
        ///
        /// Note that care must be taken to avoid tampering with the state of the
        /// sink or stream which may otherwise confuse this combinator.
        pub fn get_pin_mut(self: core::pin::Pin<&mut Self>) -> core::pin::Pin<&mut $inner> {
            self.project().$field $($ind get_pin_mut())*
        }

        /// Consumes this combinator, returning the underlying sink or stream.
        ///
        /// Note that this may discard intermediate state of this combinator, so
        /// care should be taken to avoid losing resources when this is called.
        pub fn into_inner(self) -> $inner {
            self.$field $($ind into_inner())*
        }
    }
}

#[cfg(feature = "utilities")]
macro_rules! delegate_all {
    (@trait Future $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> crate::traits::future::Future for $name<$($arg),*> where $t: crate::traits::future::Future $(, $($bound)*)* {
            type Output = <$t as crate::traits::future::Future>::Output;

            delegate_future!(inner);
        }
    };
    (@trait FusedFuture $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> crate::traits::future::FusedFuture for $name<$($arg),*> where $t: crate::traits::future::FusedFuture $(, $($bound)*)* {
            fn is_terminated(&self) -> bool {
                self.inner.is_terminated()
            }
        }
    };
    (@trait Stream $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> crate::traits::stream::Stream for $name<$($arg),*> where $t: crate::traits::stream::Stream $(, $($bound)*)* {
            type Item = <$t as crate::traits::stream::Stream>::Item;

            delegate_stream!(inner);
        }
    };
    (@trait FusedStream $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> crate::traits::stream::FusedStream for $name<$($arg),*> where $t: crate::traits::stream::FusedStream $(, $($bound)*)* {
            fn is_terminated(&self) -> bool {
                self.inner.is_terminated()
            }
        }
    };
    (@trait Sink $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        #[cfg(feature = "sink")]
        impl<_Item, $($arg),*> crate::sink_trait::Sink<_Item> for $name<$($arg),*> where $t: crate::sink_trait::Sink<_Item> $(, $($bound)*)* {
            type Error = <$t as crate::sink_trait::Sink<_Item>>::Error;

            delegate_sink!(inner, _Item);
        }
    };
    (@trait Debug $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> core::fmt::Debug for $name<$($arg),*> where $t: core::fmt::Debug $(, $($bound)*)* {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Debug::fmt(&self.inner, f)
            }
        }
    };
    (@trait AccessInner[$inner:ty, ($($ind:tt)*)] $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> $name<$($arg),*> $(where $($bound)*)* {
            delegate_access_inner!(inner, $inner, ($($ind)*));
        }
    };
    (@trait New[|$($param:ident: $paramt:ty),*| $cons:expr] $name:ident < $($arg:ident),* > ($t:ty) $(where $($bound:tt)*)*) => {
        impl<$($arg),*> $name<$($arg),*> $(where $($bound)*)* {
            pub(crate) fn new($($param: $paramt),*) -> Self {
                Self { inner: $cons }
            }
        }
    };
    ($(#[$attr:meta])* $name:ident<$($arg:ident),*>($t:ty) : $ftrait:ident $([$($targs:tt)*])* $({$($item:tt)*})* $(where $($bound:tt)*)*) => {
        pin_project_lite::pin_project! {
            #[must_use = "futures/streams/sinks do nothing unless you `.await` or poll them"]
            $(#[$attr])*
            pub struct $name< $($arg),* > $(where $($bound)*)* { #[pin] inner: $t }
        }

        impl<$($arg),*> $name< $($arg),* > $(where $($bound)*)* {
            $($($item)*)*
        }

        delegate_all!(@trait $ftrait $([$($targs)*])* $name<$($arg),*>($t) $(where $($bound)*)*);
    };
    ($(#[$attr:meta])* $name:ident<$($arg:ident),*>($t:ty) : $ftrait:ident $([$($ftargs:tt)*])* + $strait:ident $([$($stargs:tt)*])* $(+ $trait:ident $([$($targs:tt)*])*)* $({$($item:tt)*})* $(where $($bound:tt)*)*) => {
        delegate_all!($(#[$attr])* $name<$($arg),*>($t) : $strait $([$($stargs)*])* $(+ $trait $([$($targs)*])*)* $({$($item)*})* $(where $($bound)*)*);

        delegate_all!(@trait $ftrait $([$($ftargs)*])* $name<$($arg),*>($t) $(where $($bound)*)*);
    };
}

#[cfg(feature = "utilities")]
pub mod future;
#[doc(no_inline)]
#[cfg(feature = "utilities")]
pub use crate::future::{Future, FutureExt, TryFuture, TryFutureExt};

#[cfg(feature = "utilities")]
pub mod stream;
#[doc(no_inline)]
#[cfg(feature = "utilities")]
pub use crate::stream::{Stream, StreamExt, TryStream, TryStreamExt};

#[cfg(feature = "sink")]
#[cfg_attr(docsrs, doc(cfg(feature = "sink")))]
#[cfg(feature = "utilities")]
pub mod sink;
#[cfg(feature = "sink")]
#[doc(no_inline)]
#[cfg(feature = "utilities")]
pub use crate::sink::{Sink, SinkExt};

#[cfg(feature = "utilities")]
pub mod task;

#[cfg(feature = "utilities")]
pub mod never;

#[cfg(feature = "io")]
#[cfg_attr(docsrs, doc(cfg(feature = "io")))]
#[cfg(feature = "std")]
pub mod io;
#[cfg(feature = "io")]
#[cfg(feature = "std")]
#[doc(no_inline)]
pub use crate::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite,
    AsyncWriteExt,
};

#[cfg(feature = "alloc")]
#[cfg(feature = "utilities")]
pub mod lock;

#[cfg(target_has_atomic = "ptr")]
#[cfg(feature = "alloc")]
#[cfg(feature = "utilities")]
mod abortable;

#[cfg(feature = "utilities")]
mod fns;
#[cfg(feature = "utilities")]
mod macros;
#[cfg(feature = "utilities")]
mod unfold_state;

// Core traits stay available without alloc, std, combinators, or procedural macros.
#[cfg(feature = "channel")]
pub mod channel;
#[cfg(feature = "executor")]
pub mod executor;
#[cfg(feature = "std")]
mod io_traits;
mod sink_trait;
mod task_support;
mod traits;
#[cfg(not(feature = "utilities"))]
/// Future traits.
pub mod future {
    #[cfg(feature = "alloc")]
    pub use crate::traits::future::{BoxFuture, LocalBoxFuture};
    pub use crate::traits::future::{FusedFuture, TryFuture};
    pub use core::future::Future;
}
#[cfg(not(feature = "utilities"))]
/// Asynchronous stream traits.
pub mod stream {
    pub use crate::traits::stream::*;
}
#[cfg(not(feature = "utilities"))]
pub use future::{Future, TryFuture};
#[cfg(not(feature = "utilities"))]
pub use stream::{Stream, TryStream};
#[cfg(not(feature = "utilities"))]
/// Task spawning and notification primitives.
pub mod task {
    pub use crate::task_support::*;
    pub use crate::traits::task::__internal::AtomicWaker;
    #[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
    pub use alloc::task::Wake;
    pub use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
}
#[cfg(not(all(feature = "utilities", feature = "sink")))]
/// Asynchronous sink traits.
pub mod sink {
    pub use crate::sink_trait::Sink;
}
#[cfg(not(all(feature = "utilities", feature = "sink")))]
pub use sink::Sink;
pub use traits::future::FusedFuture;
pub use traits::stream::FusedStream;
#[cfg(all(feature = "std", not(feature = "io")))]
/// Asynchronous I/O traits.
pub mod io {
    pub use crate::io_traits::*;
}
#[cfg(all(feature = "std", not(feature = "io")))]
pub use io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite};
/// Common asynchronous traits and extension methods.
pub mod prelude {
    #[cfg(feature = "std")]
    pub use crate::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite};
    #[cfg(feature = "io")]
    pub use crate::io::{
        AsyncBufReadExt as _, AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _,
    };
    #[cfg(all(feature = "utilities", feature = "sink"))]
    pub use crate::SinkExt as _;
    pub use crate::{Future, Sink, Stream, TryFuture, TryStream};
    #[cfg(feature = "utilities")]
    pub use crate::{FutureExt as _, StreamExt as _, TryFutureExt as _, TryStreamExt as _};
}
