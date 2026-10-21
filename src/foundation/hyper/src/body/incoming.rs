use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
#[cfg(all(
    any(feature = "http1", feature = "http2"),
    any(feature = "client", feature = "server")
))]
use futures_core::ready;
use http_body::{Body, Frame, SizeHint};

#[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
use super::chan;
#[cfg(all(
    any(feature = "http1", feature = "http2"),
    any(feature = "client", feature = "server")
))]
use super::DecodedLength;
#[cfg(all(feature = "http2", any(feature = "client", feature = "server")))]
use crate::proto::h2::ping;

/// A stream of `Bytes`, used when receiving bodies from the network.
///
/// Note that Users should not instantiate this struct directly. When working with the hyper client,
/// `Incoming` is returned to you in responses. Similarly, when operating with the hyper server,
/// it is provided within requests.
///
/// # Examples
///
/// ```rust,ignore
/// async fn echo(
///    req: Request<hyper::body::Incoming>,
/// ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
///    //Here, you can process `Incoming`
/// }
/// ```
#[must_use = "streams do nothing unless polled"]
pub struct Incoming {
    kind: Kind,
}

enum Kind {
    Empty,
    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    Chan {
        content_length: DecodedLength,
        rx: chan::Receiver,
    },
    #[cfg(all(feature = "http2", any(feature = "client", feature = "server")))]
    H2 {
        content_length: DecodedLength,
        data_done: bool,
        ping: ping::Recorder,
        recv: h2::RecvStream,
    },
    #[cfg(feature = "ffi")]
    Ffi(crate::ffi::UserBody),
}

/// A sender half created through [`Body::channel()`].
///
/// Useful when wanting to stream chunks from another thread.
///
/// ## Body Closing
///
/// Note that the request body will always be closed normally when the sender is dropped (meaning
/// that the empty terminating chunk will be sent to the remote). If you desire to close the
/// connection with an incomplete response (e.g. in the case of an error during asynchronous
/// processing), call the [`Sender::abort()`] method to abort the body in an abnormal fashion.
///
/// [`Body::channel()`]: struct.Body.html#method.channel
/// [`Sender::abort()`]: struct.Sender.html#method.abort
#[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
pub(crate) use super::chan::Sender;

impl Incoming {
    /// Create a `Body` stream with an associated sender half.
    ///
    /// Useful when wanting to stream chunks from another thread.
    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    #[inline]
    #[cfg(test)]
    pub(crate) fn channel() -> (Sender, Incoming) {
        Self::new_channel(DecodedLength::CHUNKED, /*wanter =*/ false)
    }

    #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
    pub(crate) fn new_channel(content_length: DecodedLength, wanter: bool) -> (Sender, Incoming) {
        let (tx, rx) = chan::channel(wanter);
        let rx = Incoming::new(Kind::Chan { content_length, rx });

        (tx, rx)
    }

    fn new(kind: Kind) -> Incoming {
        Incoming { kind }
    }

    #[allow(dead_code)]
    pub(crate) fn empty() -> Incoming {
        Incoming::new(Kind::Empty)
    }

    #[cfg(feature = "ffi")]
    pub(crate) fn ffi() -> Incoming {
        Incoming::new(Kind::Ffi(crate::ffi::UserBody::new()))
    }

    #[cfg(all(feature = "http2", any(feature = "client", feature = "server")))]
    pub(crate) fn h2(
        recv: h2::RecvStream,
        mut content_length: DecodedLength,
        ping: ping::Recorder,
    ) -> Self {
        // If the stream is already EOS, then the "unknown length" is clearly
        // actually ZERO.
        if !content_length.is_exact() && recv.is_end_stream() {
            content_length = DecodedLength::ZERO;
        }

        Incoming::new(Kind::H2 {
            data_done: false,
            ping,
            content_length,
            recv,
        })
    }

    #[cfg(feature = "ffi")]
    pub(crate) fn as_ffi_mut(&mut self) -> &mut crate::ffi::UserBody {
        if !matches!(self.kind, Kind::Ffi(_)) {
            self.kind = Kind::Ffi(crate::ffi::UserBody::new());
        }

        match &mut self.kind {
            Kind::Ffi(body) => body,
            _ => unreachable!(),
        }
    }
}

impl Body for Incoming {
    type Data = Bytes;
    type Error = crate::Error;

    fn poll_frame(
        #[cfg_attr(
            not(all(
                any(feature = "http1", feature = "http2"),
                any(feature = "client", feature = "server")
            )),
            allow(unused_mut)
        )]
        mut self: Pin<&mut Self>,
        #[cfg_attr(
            not(all(
                any(feature = "http1", feature = "http2"),
                any(feature = "client", feature = "server")
            )),
            allow(unused_variables)
        )]
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut self.kind {
            Kind::Empty => Poll::Ready(None),
            #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
            Kind::Chan {
                content_length: len,
                rx,
            } => {
                if let Some(chunk) = ready!(rx.poll_next(cx)?) {
                    len.sub_if(chunk.len() as u64);
                    return Poll::Ready(Some(Ok(Frame::data(chunk))));
                }
                Poll::Ready(rx.take_trailers().map(Frame::trailers).map(Ok))
            }
            #[cfg(all(feature = "http2", any(feature = "client", feature = "server")))]
            Kind::H2 {
                data_done,
                ping,
                recv: h2,
                content_length: len,
            } => {
                if !*data_done {
                    match ready!(h2.poll_data(cx)) {
                        Some(Ok(bytes)) => {
                            let _ = h2.flow_control().release_capacity(bytes.len());
                            len.sub_if(bytes.len() as u64);
                            ping.record_data(bytes.len());
                            return Poll::Ready(Some(Ok(Frame::data(bytes))));
                        }
                        Some(Err(e)) => {
                            if let Some(h2::Reason::NO_ERROR) = e.reason() {
                                // As mentioned in RFC 7540 Section 8.1, a RST_STREAM with NO_ERROR
                                // indicates an early response, and should cause the body reading
                                // to stop, but not fail it:
                                return Poll::Ready(None);
                            } else {
                                return Poll::Ready(Some(Err(crate::Error::new_body(e))));
                            }
                        }
                        None => {
                            *data_done = true;
                            // fall through to trailers
                        }
                    }
                }

                // after data, check trailers
                match ready!(h2.poll_trailers(cx)) {
                    Ok(t) => {
                        ping.record_non_data();
                        Poll::Ready(Ok(t.map(Frame::trailers)).transpose())
                    }
                    Err(e) => {
                        if let Some(h2::Reason::NO_ERROR) = e.reason() {
                            // Same as above, a RST_STREAM with NO_ERROR indicates an early
                            // response, and should cause reading the trailers to stop, but
                            // not fail it:
                            Poll::Ready(None)
                        } else {
                            Poll::Ready(Some(Err(crate::Error::new_h2(e))))
                        }
                    }
                }
            }

            #[cfg(feature = "ffi")]
            Kind::Ffi(body) => body.poll_data(cx),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            Kind::Empty => true,
            #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
            Kind::Chan { content_length, .. } => *content_length == DecodedLength::ZERO,
            #[cfg(all(feature = "http2", any(feature = "client", feature = "server")))]
            Kind::H2 { recv: h2, .. } => h2.is_end_stream(),
            #[cfg(feature = "ffi")]
            Kind::Ffi(..) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        #[cfg(all(
            any(feature = "http1", feature = "http2"),
            any(feature = "client", feature = "server")
        ))]
        fn opt_len(decoded_length: DecodedLength) -> SizeHint {
            if let Some(content_length) = decoded_length.into_opt() {
                SizeHint::with_exact(content_length)
            } else {
                SizeHint::default()
            }
        }

        match self.kind {
            Kind::Empty => SizeHint::with_exact(0),
            #[cfg(all(feature = "http1", any(feature = "client", feature = "server")))]
            Kind::Chan { content_length, .. } => opt_len(content_length),
            #[cfg(all(feature = "http2", any(feature = "client", feature = "server")))]
            Kind::H2 { content_length, .. } => opt_len(content_length),
            #[cfg(feature = "ffi")]
            Kind::Ffi(..) => SizeHint::default(),
        }
    }
}

impl fmt::Debug for Incoming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(any(
            all(
                any(feature = "http1", feature = "http2"),
                any(feature = "client", feature = "server")
            ),
            feature = "ffi"
        ))]
        #[derive(Debug)]
        struct Streaming;
        #[derive(Debug)]
        struct Empty;

        let mut builder = f.debug_tuple("Body");
        match self.kind {
            Kind::Empty => builder.field(&Empty),
            #[cfg(any(
                all(
                    any(feature = "http1", feature = "http2"),
                    any(feature = "client", feature = "server")
                ),
                feature = "ffi"
            ))]
            _ => builder.field(&Streaming),
        };

        builder.finish()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hyper/unit/src/body/incoming/tests_302.rs"));
