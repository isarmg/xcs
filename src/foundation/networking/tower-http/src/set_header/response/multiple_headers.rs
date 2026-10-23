//! Set multiple headers on the response.
//!
//! See the root [`crate::set_header::response`] module for full documentation and usage examples.
//!
use http::{Request, Response};
use pin_project_lite::pin_project;
use std::{
    fmt,
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};
use service_stack::Layer;
use service_stack::Service;

use crate::set_header::{HeaderInsertionConfig, HeaderMetadata, InsertHeaderMode};

/// Layer that applies [`SetMultipleResponseHeader`] which adds multiple response headers.
///
/// See [`SetMultipleResponseHeader`] for more details.
#[derive(Clone)]
pub struct SetMultipleResponseHeadersLayer<M> {
    headers: Vec<HeaderInsertionConfig<M>>,
}

impl<M> fmt::Debug for SetMultipleResponseHeadersLayer<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetMultipleResponseHeadersLayer")
            .field("headers", &self.headers)
            .finish()
    }
}

impl<M> SetMultipleResponseHeadersLayer<M> {
    /// Create a new [`SetMultipleResponseHeadersLayer`] that overrides any existing values for the same header.
    ///
    /// If any previous value exists for the same header, it is removed and replaced with the new matching header value.
    pub fn overriding(metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Override))
            .collect();

        Self::new(headers)
    }

    /// Create a new [`SetMultipleResponseHeadersLayer`] that appends header values.
    ///
    /// The new header is always added, preserving any existing values. If previous values exist, the header will have multiple values.
    pub fn appending(metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Append))
            .collect();

        Self::new(headers)
    }

    /// Create a new [`SetMultipleResponseHeadersLayer`] that only inserts if the header is not already present.
    ///
    /// If a previous value exists for the header, the new value is not inserted.
    pub fn if_not_present(metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::IfNotPresent))
            .collect();

        Self::new(headers)
    }

    /// Internal constructor for a new [`SetMultipleResponseHeadersLayer`] from a list of headers.
    fn new(headers: Vec<HeaderInsertionConfig<M>>) -> Self {
        Self { headers }
    }
}

impl<S, M> Layer<S> for SetMultipleResponseHeadersLayer<M> {
    type Service = SetMultipleResponseHeader<S, M>;

    fn layer(&self, inner: S) -> Self::Service {
        SetMultipleResponseHeader {
            inner,
            headers: self.headers.clone(),
        }
    }
}

/// Middleware that sets multiple headers on the response.

#[derive(Clone)]
pub struct SetMultipleResponseHeader<S, M> {
    inner: S,
    headers: Vec<HeaderInsertionConfig<M>>,
}

impl<S, M> SetMultipleResponseHeader<S, M> {
    /// Create a new [`SetMultipleResponseHeader`] that overrides any existing values for the same header.
    ///
    /// If a previous value exists for the same header, it is removed and replaced with the new header value.
    pub fn overriding(inner: S, metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Override))
            .collect();

        Self::new(inner, headers)
    }

    /// Create a new [`SetMultipleResponseHeader`] that appends header values.
    ///
    /// The new header is always added, preserving any existing values. If previous values exist, the header will have multiple values.
    pub fn appending(inner: S, metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Append))
            .collect();

        Self::new(inner, headers)
    }

    /// Create a new [`SetMultipleResponseHeader`] that only inserts if the header is not already present.
    ///
    /// If a previous value exists for the header, the new value is not inserted.
    pub fn if_not_present(inner: S, metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::IfNotPresent))
            .collect();

        Self::new(inner, headers)
    }

    /// Internal constructor for a new [`SetMultipleResponseHeader`] from an inner service and a list of headers.
    fn new(inner: S, headers: Vec<HeaderInsertionConfig<M>>) -> Self {
        Self { inner, headers }
    }

    define_inner_service_accessors!();
}

impl<S, M> fmt::Debug for SetMultipleResponseHeader<S, M>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetMultipleResponseHeader")
            .field("inner", &self.inner)
            .field("headers", &self.headers)
            .finish()
    }
}

impl<ReqBody, ResBody, S> Service<Request<ReqBody>>
    for SetMultipleResponseHeader<S, Response<ResBody>>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, Response<ResBody>>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    /// Call the inner service and apply all configured headers to the response.
    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        ResponseFuture {
            future: self.inner.call(req),
            headers: self.headers.clone(),
        }
    }
}

pin_project! {
    /// Response future for [`SetMultipleResponseHeader`].
    #[derive(Debug)]
    pub struct ResponseFuture<F, M> {
        #[pin]
        future: F,
        headers: Vec<HeaderInsertionConfig<M>>,
    }
}

impl<F, ResBody, E> Future for ResponseFuture<F, Response<ResBody>>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
{
    type Output = F::Output;

    /// Polls the inner future and applies all configured headers to the response before returning it.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let mut res = ready!(this.future.poll(cx)?);

        for header in this.headers {
            header
                .mode
                .apply(&header.header_name, &mut res, &mut header.make);
        }

        Poll::Ready(Ok(res))
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tower-http/unit/src/set_header/response/multiple_headers/tests_207.rs"));
