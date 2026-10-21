//! Utility to gracefully shutdown a server.
//!
//! This module provides a [`GracefulShutdown`] type,
//! which can be used to gracefully shutdown a server.
//!
//! See <https://github.com/hyperium/hyper-util/blob/master/examples/server_graceful.rs>
//! for an example of how to use this.

use std::{
    fmt::{self, Debug},
    pin::Pin,
    task::{self, Poll},
};

use pin_project_lite::pin_project;
use tokio::sync::watch;

/// A graceful shutdown utility
// Purposefully not `Clone`, see `watcher()` method for why.
pub struct GracefulShutdown {
    tx: watch::Sender<()>,
}

/// A watcher side of the graceful shutdown.
///
/// This type can only watch a connection, it cannot trigger a shutdown.
///
/// Call [`GracefulShutdown::watcher()`] to construct one of these.
pub struct Watcher {
    rx: watch::Receiver<()>,
}

impl GracefulShutdown {
    /// Create a new graceful shutdown helper.
    pub fn new() -> Self {
        let (tx, _) = watch::channel(());
        Self { tx }
    }

    /// Wrap a future for graceful shutdown watching.
    pub fn watch<C: GracefulConnection>(
        &self,
        conn: C,
    ) -> impl Future<Output = C::Output> + use<C> {
        self.watcher().watch(conn)
    }

    /// Create an owned type that can watch a connection.
    ///
    /// This method allows created an owned type that can be sent onto another
    /// task before calling [`Watcher::watch()`].
    // Internal: this function exists because `Clone` allows footguns.
    // If the `tx` were cloned (or the `rx`), race conditions can happens where
    // one task starting a shutdown is scheduled and interwined with a task
    // starting to watch a connection, and the "watch version" is one behind.
    pub fn watcher(&self) -> Watcher {
        let rx = self.tx.subscribe();
        Watcher { rx }
    }

    /// Signal shutdown for all watched connections.
    ///
    /// This returns a `Future` which will complete once all watched
    /// connections have shutdown.
    pub async fn shutdown(self) {
        let Self { tx } = self;

        // signal all the watched futures about the change
        let _ = tx.send(());
        // and then wait for all of them to complete
        tx.closed().await;
    }

    /// Returns the number of the watching connections.
    pub fn count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Debug for GracefulShutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GracefulShutdown").finish()
    }
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Watcher {
    /// Wrap a future for graceful shutdown watching.
    pub fn watch<C: GracefulConnection>(self, conn: C) -> impl Future<Output = C::Output> {
        let Watcher { mut rx } = self;
        GracefulConnectionFuture::new(conn, async move {
            let _ = rx.changed().await;
            // hold onto the rx until the watched future is completed
            rx
        })
    }
}

impl Debug for Watcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GracefulWatcher").finish()
    }
}

pin_project! {
    struct GracefulConnectionFuture<C, F: Future> {
        #[pin]
        conn: C,
        #[pin]
        cancel: F,
        #[pin]
        // If cancelled, this is held until the inner conn is done.
        cancelled_guard: Option<F::Output>,
    }
}

impl<C, F: Future> GracefulConnectionFuture<C, F> {
    fn new(conn: C, cancel: F) -> Self {
        Self {
            conn,
            cancel,
            cancelled_guard: None,
        }
    }
}

impl<C, F: Future> Debug for GracefulConnectionFuture<C, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GracefulConnectionFuture").finish()
    }
}

impl<C, F> Future for GracefulConnectionFuture<C, F>
where
    C: GracefulConnection,
    F: Future,
{
    type Output = C::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        if this.cancelled_guard.is_none() {
            if let Poll::Ready(guard) = this.cancel.poll(cx) {
                this.cancelled_guard.set(Some(guard));
                this.conn.as_mut().graceful_shutdown();
            }
        }
        this.conn.poll(cx)
    }
}

/// An internal utility trait as an umbrella target for all (hyper) connection
/// types that the [`GracefulShutdown`] can watch.
pub trait GracefulConnection: Future<Output = Result<(), Self::Error>> + private::Sealed {
    /// The error type returned by the connection when used as a future.
    type Error;

    /// Start a graceful shutdown process for this connection.
    fn graceful_shutdown(self: Pin<&mut Self>);
}

#[cfg(feature = "http1")]
impl<I, B, S> GracefulConnection for crate::server::conn::http1::Connection<I, S>
where
    S: crate::service::HttpService<crate::body::Incoming, ResBody = B>,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    I: crate::rt::Read + crate::rt::Write + Unpin + 'static,
    B: crate::body::Body + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Error = crate::Error;

    fn graceful_shutdown(self: Pin<&mut Self>) {
        crate::server::conn::http1::Connection::graceful_shutdown(self);
    }
}

#[cfg(feature = "server-auto")]
impl<I, B, S, E> GracefulConnection for crate::adapters::server::conn::auto::Connection<'_, I, S, E>
where
    S: crate::service::Service<http::Request<crate::body::Incoming>, Response = http::Response<B>>,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: 'static,
    I: crate::rt::Read + crate::rt::Write + Unpin + 'static,
    B: crate::body::Body + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    E: crate::adapters::server::conn::auto::HttpServerConnExec<S::Future, B>,
{
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn graceful_shutdown(self: Pin<&mut Self>) {
        crate::adapters::server::conn::auto::Connection::graceful_shutdown(self);
    }
}

#[cfg(feature = "server-auto")]
impl<I, B, S, E> GracefulConnection
    for crate::adapters::server::conn::auto::UpgradeableConnection<'_, I, S, E>
where
    S: crate::service::Service<http::Request<crate::body::Incoming>, Response = http::Response<B>>,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: 'static,
    I: crate::rt::Read + crate::rt::Write + Unpin + Send + 'static,
    B: crate::body::Body + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    E: crate::adapters::server::conn::auto::HttpServerConnExec<S::Future, B>,
{
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn graceful_shutdown(self: Pin<&mut Self>) {
        crate::adapters::server::conn::auto::UpgradeableConnection::graceful_shutdown(self);
    }
}

mod private {
    pub trait Sealed {}

    #[cfg(feature = "http1")]
    impl<I, B, S> Sealed for crate::server::conn::http1::Connection<I, S>
    where
        S: crate::service::HttpService<crate::body::Incoming, ResBody = B>,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        I: crate::rt::Read + crate::rt::Write + Unpin + 'static,
        B: crate::body::Body + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
    }

    #[cfg(feature = "http1")]
    impl<I, B, S> Sealed for crate::server::conn::http1::UpgradeableConnection<I, S>
    where
        S: crate::service::HttpService<crate::body::Incoming, ResBody = B>,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        I: crate::rt::Read + crate::rt::Write + Unpin + 'static,
        B: crate::body::Body + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
    }

    #[cfg(feature = "server-auto")]
    impl<I, B, S, E> Sealed for crate::adapters::server::conn::auto::Connection<'_, I, S, E>
    where
        S: crate::service::Service<
            http::Request<crate::body::Incoming>,
            Response = http::Response<B>,
        >,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        S::Future: 'static,
        I: crate::rt::Read + crate::rt::Write + Unpin + 'static,
        B: crate::body::Body + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        E: crate::adapters::server::conn::auto::HttpServerConnExec<S::Future, B>,
    {
    }

    #[cfg(feature = "server-auto")]
    impl<I, B, S, E> Sealed for crate::adapters::server::conn::auto::UpgradeableConnection<'_, I, S, E>
    where
        S: crate::service::Service<
            http::Request<crate::body::Incoming>,
            Response = http::Response<B>,
        >,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        S::Future: 'static,
        I: crate::rt::Read + crate::rt::Write + Unpin + Send + 'static,
        B: crate::body::Body + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        E: crate::adapters::server::conn::auto::HttpServerConnExec<S::Future, B>,
    {
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/http-transport/adapters/unit/src/server/graceful/test_307.rs"
));

use std::future::Future;
