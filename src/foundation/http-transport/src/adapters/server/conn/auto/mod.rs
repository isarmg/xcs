//! HTTP/1 connections with explicit rejection of the HTTP/2 preface.

pub mod upgrade;

use crate::service::HttpService;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::{error::Error as StdError, io, time::Duration};

use crate::{
    body::Incoming,
    rt::{Read, ReadBuf, Sleep, Timer, Write},
    service::Service,
};
use bytes::Bytes;
use http::{Request, Response};
use http_body::Body;

#[cfg(feature = "http1")]
use crate::server::conn::http1;

use pin_project_lite::pin_project;

use crate::adapters::common::rewind::Rewind;

type Error = Box<dyn std::error::Error + Send + Sync>;

type Result<T> = std::result::Result<T, Error>;

const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// HTTP/1 connection builder.
#[derive(Clone, Debug)]
pub struct Builder {
    #[cfg(feature = "http1")]
    http1: http1::Builder,

    #[cfg(feature = "http1")]
    version: Option<Version>,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    /// Create a new auto connection builder.
    ///
    /// The connection is driven by polling its future; no executor is needed.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "tokio")]
    /// # {
    /// use http_transport::adapters::{
    ///     server::conn::auto,
    /// };
    ///
    /// auto::Builder::new();
    /// # }
    /// ```
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "http1")]
            http1: http1::Builder::new(),

            #[cfg(feature = "http1")]
            version: None,
        }
    }

    /// Http1 configuration.
    #[cfg(feature = "http1")]
    pub fn http1(&mut self) -> Http1Builder<'_> {
        Http1Builder { inner: self }
    }

    /// Only accepts HTTP/1
    ///
    /// Does not do anything if used with [`serve_connection_with_upgrades`]
    ///
    /// [`serve_connection_with_upgrades`]: Builder::serve_connection_with_upgrades
    #[cfg(feature = "http1")]
    pub fn http1_only(mut self) -> Self {
        assert!(self.version.is_none());
        self.version = Some(Version::H1);
        self
    }

    /// Returns `true` if this builder can serve an HTTP/1.1-based connection.
    pub fn is_http1_available(&self) -> bool {
        true
    }

    /// Returns `false`: HTTP/2 transport is not supported.
    pub fn is_http2_available(&self) -> bool {
        false
    }

    /// Set whether HTTP/1 connections will write header names as title case at
    /// the socket level.
    ///
    /// This setting only affects HTTP/1 connections. HTTP/2 connections are
    /// not affected by this setting.
    ///
    /// Default is false.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "tokio")]
    /// # {
    /// use http_transport::adapters::{
    ///     server::conn::auto,
    /// };
    ///
    /// auto::Builder::new()
    ///     .title_case_headers(true);
    /// # }
    /// ```
    #[cfg(feature = "http1")]
    pub fn title_case_headers(mut self, enabled: bool) -> Self {
        self.http1.title_case_headers(enabled);
        self
    }

    /// Set whether HTTP/1 connections will preserve the original case of header names.
    ///
    /// This setting only affects HTTP/1 connections. HTTP/2 connections are
    /// not affected by this setting.
    ///
    /// Default is false.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "tokio")]
    /// # {
    /// use http_transport::adapters::{
    ///     server::conn::auto,
    /// };
    ///
    /// auto::Builder::new()
    ///     .preserve_header_case(true);
    /// # }
    /// ```
    #[cfg(feature = "http1")]
    pub fn preserve_header_case(mut self, enabled: bool) -> Self {
        self.http1.preserve_header_case(enabled);
        self
    }

    /// Bind a connection together with a [`Service`].
    pub fn serve_connection<I, S, B>(&self, io: I, service: S) -> Connection<'_, I, S>
    where
        S: Service<Request<Incoming>, Response = Response<B>>,
        S::Future: 'static,
        S::Error: Into<Box<dyn StdError + Send + Sync>>,
        B: Body + 'static,
        B::Error: Into<Box<dyn StdError + Send + Sync>>,
        I: Read + Write + Unpin + 'static,
    {
        let state = match self.version {
            #[cfg(feature = "http1")]
            Some(Version::H1) => {
                let io = Rewind::new_buffered(io, Bytes::new());
                let conn = self.http1.serve_connection(io, service);
                ConnState::H1 { conn }
            }

            #[cfg(feature = "http1")]
            _ => ConnState::ReadVersion {
                read_version: read_version(io, &self.http1),
                builder: Cow::Borrowed(self),
                service: Some(service),
            },
        };

        Connection { state }
    }

    /// Bind a connection together with a [`Service`], with the ability to
    /// handle HTTP upgrades. This requires that the IO object implements
    /// `Send`.
    ///
    /// Note that if you ever want to use [`crate::upgrade::Upgraded::downcast`]
    /// with this crate, you'll need to use [`http_transport::adapters::server::conn::auto::upgrade::downcast`]
    /// instead. See the documentation of the latter to understand why.
    ///
    /// [`http_transport::adapters::server::conn::auto::upgrade::downcast`]: crate::adapters::server::conn::auto::upgrade::downcast
    pub fn serve_connection_with_upgrades<I, S, B>(
        &self,
        io: I,
        service: S,
    ) -> UpgradeableConnection<'_, I, S>
    where
        S: Service<Request<Incoming>, Response = Response<B>>,
        S::Future: 'static,
        S::Error: Into<Box<dyn StdError + Send + Sync>>,
        B: Body + 'static,
        B::Error: Into<Box<dyn StdError + Send + Sync>>,
        I: Read + Write + Unpin + Send + 'static,
    {
        UpgradeableConnection {
            state: UpgradeableConnState::ReadVersion {
                read_version: read_version(io, &self.http1),
                builder: Cow::Borrowed(self),
                service: Some(service),
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Version {
    H1,
    H2,
}

impl Version {
    #[must_use]

    pub fn unsupported(self) -> Error {
        match self {
            Version::H1 => Error::from("HTTP/1 is not supported"),
            Version::H2 => Error::from("HTTP/2 is not supported"),
        }
    }
}

fn read_version<I>(io: I, builder: &http1::Builder) -> ReadVersion<I>
where
    I: Read + Unpin,
{
    let (timer, header_timeout) = builder.header_timeout_config();
    ReadVersion {
        timer,
        header_timeout,
        timeout: None,
        io: Some(io),
        buf: [MaybeUninit::uninit(); 24],
        filled: 0,
        version: Version::H2,
        cancelled: false,
        _pin: PhantomPinned,
    }
}

pin_project! {
    struct ReadVersion<I> {
        timer: crate::common::time::Time,
        header_timeout: Option<Duration>,
        timeout: Option<Pin<Box<dyn Sleep>>>,
        io: Option<I>,
        buf: [MaybeUninit<u8>; 24],
        // the amount of `buf` thats been filled
        filled: usize,
        version: Version,
        cancelled: bool,
        // Make this future `!Unpin` for compatibility with async trait methods.
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<I> ReadVersion<I> {
    pub fn cancel(self: Pin<&mut Self>) {
        *self.project().cancelled = true;
    }
}

impl<I> Future for ReadVersion<I>
where
    I: Read + Unpin,
{
    type Output = Result<(Version, Rewind<I>, Option<Pin<Box<dyn Sleep>>>)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.cancelled {
            return Poll::Ready(Err(
                io::Error::new(io::ErrorKind::Interrupted, "Cancelled").into()
            ));
        }

        // Match HTTP/1's first-poll start and data-before-timeout precedence.
        if let Some(duration) = this.header_timeout.take() {
            *this.timeout = Some(this.timer.sleep_until(this.timer.now() + duration));
        }
        let mut buf = ReadBuf::uninit(&mut *this.buf);
        // SAFETY: `this.filled` tracks how many bytes have been read (and thus initialized) and
        // we're only advancing by that many.
        unsafe {
            buf.unfilled().advance(*this.filled);
        };

        // We start as H2 and switch to H1 as soon as we don't have the preface.
        while buf.filled().len() < H2_PREFACE.len() {
            let len = buf.filled().len();
            match Pin::new(this.io.as_mut().unwrap()).poll_read(cx, buf.unfilled()) {
                Poll::Ready(result) => result?,
                Poll::Pending => {
                    if let Some(timeout) = this.timeout.as_mut() {
                        if timeout.as_mut().poll(cx).is_ready() {
                            return Poll::Ready(Err(crate::Error::new_header_timeout().into()));
                        }
                    }
                    return Poll::Pending;
                }
            }
            *this.filled = buf.filled().len();

            // We starts as H2 and switch to H1 when we don't get the preface.
            if buf.filled().len() == len
                || buf.filled()[len..] != H2_PREFACE[len..buf.filled().len()]
            {
                *this.version = Version::H1;
                break;
            }
        }

        let io = this.io.take().unwrap();
        let buf = buf.filled().to_vec();
        Poll::Ready(Ok((
            *this.version,
            Rewind::new_buffered(io, Bytes::from(buf)),
            this.timeout.take(),
        )))
    }
}

pin_project! {
    /// A [`Future`](core::future::Future) representing an HTTP/1 connection, returned from
    /// [`Builder::serve_connection`](struct.Builder.html#method.serve_connection).
    ///
    /// To drive HTTP on this connection this future **must be polled**, typically with
    /// `.await`. If it isn't polled, no progress will be made on this connection.
    #[must_use = "futures do nothing unless polled"]
    pub struct Connection<'a, I, S>
    where
        S: HttpService<Incoming>,
    {
        #[pin]
        state: ConnState<'a, I, S>,
    }
}

// A custom COW, since the libstd is has ToOwned bounds that are too eager.
enum Cow<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<T> std::ops::Deref for Cow<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            Cow::Borrowed(t) => &*t,
            Cow::Owned(t) => t,
        }
    }
}

#[cfg(feature = "http1")]
type Http1Connection<I, S> = crate::server::conn::http1::Connection<Rewind<I>, S>;

pin_project! {
    #[project = ConnStateProj]
    enum ConnState<'a, I, S>
    where
        S: HttpService<Incoming>,
    {
        ReadVersion {
            #[pin]
            read_version: ReadVersion<I>,
            builder: Cow<'a, Builder>,
            service: Option<S>,
        },
        H1 {
            #[pin]
            conn: Http1Connection<I, S>,
        },
    }
}

impl<I, S, B> Connection<'_, I, S>
where
    S: HttpService<Incoming, ResBody = B>,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    I: Read + Write + Unpin,
    B: Body + 'static,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    /// Start a graceful shutdown process for this connection.
    ///
    /// This `Connection` should continue to be polled until shutdown can finish.
    ///
    /// # Note
    ///
    /// This should only be called while the `Connection` future is still pending. If called after
    /// `Connection::poll` has resolved, this does nothing.
    pub fn graceful_shutdown(self: Pin<&mut Self>) {
        match self.project().state.project() {
            ConnStateProj::ReadVersion { read_version, .. } => read_version.cancel(),
            #[cfg(feature = "http1")]
            ConnStateProj::H1 { conn } => conn.graceful_shutdown(),
        }
    }

    /// Make this Connection static, instead of borrowing from Builder.
    pub fn into_owned(self) -> Connection<'static, I, S> {
        Connection {
            state: match self.state {
                ConnState::ReadVersion {
                    read_version,
                    builder,
                    service,
                } => ConnState::ReadVersion {
                    read_version,
                    service,
                    builder: Cow::Owned(builder.clone()),
                },
                #[cfg(feature = "http1")]
                ConnState::H1 { conn } => ConnState::H1 { conn },
            },
        }
    }
}

impl<I, S, B> Future for Connection<'_, I, S>
where
    S: Service<Request<Incoming>, Response = Response<B>>,
    S::Future: 'static,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B: Body + 'static,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
    I: Read + Write + Unpin + 'static,
{
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut this = self.as_mut().project();

            match this.state.as_mut().project() {
                ConnStateProj::ReadVersion {
                    read_version,
                    builder,
                    service,
                } => {
                    let (version, io, timeout) = ready!(read_version.poll(cx))?;
                    let service = service.take().unwrap();
                    match version {
                        #[cfg(feature = "http1")]
                        Version::H1 => {
                            let conn = builder
                                .http1
                                .serve_connection_with_header_timeout(io, service, timeout);
                            this.state.set(ConnState::H1 { conn });
                        }

                        _ => return Poll::Ready(Err(version.unsupported())),
                    }
                }
                #[cfg(feature = "http1")]
                ConnStateProj::H1 { conn } => {
                    return conn.poll(cx).map_err(Into::into);
                }
            }
        }
    }
}

pin_project! {
    /// An upgradable [`Connection`], returned by
    /// [`Builder::serve_upgradable_connection`](struct.Builder.html#method.serve_connection_with_upgrades).
    ///
    /// To drive HTTP on this connection this future **must be polled**, typically with
    /// `.await`. If it isn't polled, no progress will be made on this connection.
    #[must_use = "futures do nothing unless polled"]
    pub struct UpgradeableConnection<'a, I, S>
    where
        S: HttpService<Incoming>,
    {
        #[pin]
        state: UpgradeableConnState<'a, I, S>,
    }
}

#[cfg(feature = "http1")]
type Http1UpgradeableConnection<I, S> = crate::server::conn::http1::UpgradeableConnection<I, S>;

#[cfg(not(feature = "http1"))]
type Http1UpgradeableConnection<I, S> = (PhantomData<I>, PhantomData<S>);

pin_project! {
    #[project = UpgradeableConnStateProj]
    enum UpgradeableConnState<'a, I, S>
    where
        S: HttpService<Incoming>,
    {
        ReadVersion {
            #[pin]
            read_version: ReadVersion<I>,
            builder: Cow<'a, Builder>,
            service: Option<S>,
        },
        H1 {
            #[pin]
            conn: Http1UpgradeableConnection<Rewind<I>, S>,
        },
    }
}

impl<I, S, B> UpgradeableConnection<'_, I, S>
where
    S: HttpService<Incoming, ResBody = B>,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    I: Read + Write + Unpin,
    B: Body + 'static,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
{
    /// Start a graceful shutdown process for this connection.
    ///
    /// This `UpgradeableConnection` should continue to be polled until shutdown can finish.
    ///
    /// # Note
    ///
    /// This should only be called while the `Connection` future is still nothing. pending. If
    /// called after `UpgradeableConnection::poll` has resolved, this does nothing.
    pub fn graceful_shutdown(self: Pin<&mut Self>) {
        match self.project().state.project() {
            UpgradeableConnStateProj::ReadVersion { read_version, .. } => read_version.cancel(),
            #[cfg(feature = "http1")]
            UpgradeableConnStateProj::H1 { conn } => conn.graceful_shutdown(),
        }
    }

    /// Make this Connection static, instead of borrowing from Builder.
    pub fn into_owned(self) -> UpgradeableConnection<'static, I, S> {
        UpgradeableConnection {
            state: match self.state {
                UpgradeableConnState::ReadVersion {
                    read_version,
                    builder,
                    service,
                } => UpgradeableConnState::ReadVersion {
                    read_version,
                    service,
                    builder: Cow::Owned(builder.clone()),
                },
                #[cfg(feature = "http1")]
                UpgradeableConnState::H1 { conn } => UpgradeableConnState::H1 { conn },
            },
        }
    }
}

impl<I, S, B> Future for UpgradeableConnection<'_, I, S>
where
    S: Service<Request<Incoming>, Response = Response<B>>,
    S::Future: 'static,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    B: Body + 'static,
    B::Error: Into<Box<dyn StdError + Send + Sync>>,
    I: Read + Write + Unpin + Send + 'static,
{
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut this = self.as_mut().project();

            match this.state.as_mut().project() {
                UpgradeableConnStateProj::ReadVersion {
                    read_version,
                    builder,
                    service,
                } => {
                    let (version, io, timeout) = ready!(read_version.poll(cx))?;
                    let service = service.take().unwrap();
                    match version {
                        #[cfg(feature = "http1")]
                        Version::H1 => {
                            let conn = builder
                                .http1
                                .serve_connection_with_header_timeout(io, service, timeout)
                                .with_upgrades();
                            this.state.set(UpgradeableConnState::H1 { conn });
                        }

                        _ => return Poll::Ready(Err(version.unsupported())),
                    }
                }
                #[cfg(feature = "http1")]
                UpgradeableConnStateProj::H1 { conn } => {
                    return conn.poll(cx).map_err(Into::into);
                }
            }
        }
    }
}

/// Http1 part of builder.
#[cfg(feature = "http1")]
pub struct Http1Builder<'a> {
    inner: &'a mut Builder,
}

#[cfg(feature = "http1")]
impl Http1Builder<'_> {
    /// Set whether the `date` header should be included in HTTP responses.
    ///
    /// Note that including the `date` header is recommended by RFC 7231.
    ///
    /// Default is true.
    pub fn auto_date_header(&mut self, enabled: bool) -> &mut Self {
        self.inner.http1.auto_date_header(enabled);
        self
    }

    /// Set whether HTTP/1 connections should support half-closures.
    ///
    /// Clients can chose to shutdown their write-side while waiting
    /// for the server to respond. Setting this to `true` will
    /// prevent closing the connection immediately if `read`
    /// detects an EOF in the middle of a request.
    ///
    /// Default is `false`.
    pub fn half_close(&mut self, val: bool) -> &mut Self {
        self.inner.http1.half_close(val);
        self
    }

    /// Enables or disables HTTP/1 keep-alive.
    ///
    /// Default is true.
    pub fn keep_alive(&mut self, val: bool) -> &mut Self {
        self.inner.http1.keep_alive(val);
        self
    }

    /// Set whether HTTP/1 connections will write header names as title case at
    /// the socket level.
    ///
    /// Note that this setting does not affect HTTP/2.
    ///
    /// Default is false.
    pub fn title_case_headers(&mut self, enabled: bool) -> &mut Self {
        self.inner.http1.title_case_headers(enabled);
        self
    }

    /// Set whether HTTP/1 connections will silently ignored malformed header lines.
    ///
    /// If this is enabled and a header line does not start with a valid header
    /// name, or does not include a colon at all, the line will be silently ignored
    /// and no error will be reported.
    ///
    /// Default is false.
    pub fn ignore_invalid_headers(&mut self, enabled: bool) -> &mut Self {
        self.inner.http1.ignore_invalid_headers(enabled);
        self
    }

    /// Set whether to support preserving original header cases.
    ///
    /// Currently, this will record the original cases received, and store them
    /// in a private extension on the `Request`. It will also look for and use
    /// such an extension in any provided `Response`.
    ///
    /// Since the relevant extension is still private, there is no way to
    /// interact with the original cases. The only effect this can have now is
    /// to forward the cases in a proxy-like fashion.
    ///
    /// Note that this setting does not affect HTTP/2.
    ///
    /// Default is false.
    pub fn preserve_header_case(&mut self, enabled: bool) -> &mut Self {
        self.inner.http1.preserve_header_case(enabled);
        self
    }

    /// Set the maximum number of headers.
    ///
    /// When a request is received, the parser will reserve a buffer to store headers for optimal
    /// performance.
    ///
    /// If server receives more headers than the buffer size, it responds to the client with
    /// "431 Request Header Fields Too Large".
    ///
    /// The headers is allocated on the stack by default, which has higher performance. After
    /// setting this value, headers will be allocated in heap memory, that is, heap memory
    /// allocation will occur for each request, and there will be a performance drop of about 5%.
    ///
    /// Note that this setting does not affect HTTP/2.
    ///
    /// Default is 100.
    pub fn max_headers(&mut self, val: usize) -> &mut Self {
        self.inner.http1.max_headers(val);
        self
    }

    /// Set a timeout for reading client request headers. If a client does not
    /// transmit the entire header within this time, the connection is closed.
    ///
    /// Requires a [`Timer`] set by [`Http1Builder::timer`] to take effect. Panics if `header_read_timeout` is configured
    /// without a [`Timer`].
    ///
    /// Pass `None` to disable.
    ///
    /// Default is currently 30 seconds, but do not depend on that.
    pub fn header_read_timeout(&mut self, read_timeout: impl Into<Option<Duration>>) -> &mut Self {
        self.inner.http1.header_read_timeout(read_timeout);
        self
    }

    /// Set whether HTTP/1 connections should try to use vectored writes,
    /// or always flatten into a single buffer.
    ///
    /// Note that setting this to false may mean more copies of body data,
    /// but may also improve performance when an IO transport doesn't
    /// support vectored writes well, such as most TLS implementations.
    ///
    /// Setting this to true will force hyper to use queued strategy
    /// which may eliminate unnecessary cloning on some TLS backends
    ///
    /// Default is `auto`. In this mode hyper will try to guess which
    /// mode to use
    pub fn writev(&mut self, val: bool) -> &mut Self {
        self.inner.http1.writev(val);
        self
    }

    /// Set the maximum buffer size for the connection.
    ///
    /// Default is ~400kb.
    ///
    /// # Panics
    ///
    /// The minimum value allowed is 8192. This method panics if the passed `max` is less than the minimum.
    pub fn max_buf_size(&mut self, max: usize) -> &mut Self {
        self.inner.http1.max_buf_size(max);
        self
    }

    /// Aggregates flushes to better support pipelined responses.
    ///
    /// Experimental, may have bugs.
    ///
    /// Default is false.
    pub fn pipeline_flush(&mut self, enabled: bool) -> &mut Self {
        self.inner.http1.pipeline_flush(enabled);
        self
    }

    /// Set the timer used in background tasks.
    pub fn timer<M>(&mut self, timer: M) -> &mut Self
    where
        M: Timer + Send + Sync + 'static,
    {
        self.inner.http1.timer(timer);
        self
    }

    /// Bind a connection together with a [`Service`].

    pub async fn serve_connection<I, S, B>(&self, io: I, service: S) -> Result<()>
    where
        S: Service<Request<Incoming>, Response = Response<B>>,
        S::Future: 'static,
        S::Error: Into<Box<dyn StdError + Send + Sync>>,
        B: Body + 'static,
        B::Error: Into<Box<dyn StdError + Send + Sync>>,
        I: Read + Write + Unpin + 'static,
    {
        self.inner.serve_connection(io, service).await
    }
}

#[cfg(all(feature = "tokio", test))]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/http-transport/adapters/unit/src/server/conn/auto/mod/guarded_tests_1135.rs"));

use std::future::Future;
