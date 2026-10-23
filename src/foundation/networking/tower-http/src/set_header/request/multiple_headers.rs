//! Set multiple headers on the request.
//!
//! See the root [`crate::set_header::request`] module for full documentation and usage examples.
//!
use http::{Request, Response};
use std::{
    fmt,
    task::{Context, Poll},
};
use service_stack::Layer;
use service_stack::Service;

use crate::set_header::{HeaderInsertionConfig, HeaderMetadata, InsertHeaderMode};

/// Layer that applies [`SetMultipleRequestHeader`] which adds multiple request headers.
///
/// See [`SetMultipleRequestHeader`] for more details.
#[derive(Clone)]
pub struct SetMultipleRequestHeadersLayer<M> {
    headers: Vec<HeaderInsertionConfig<M>>,
}

impl<M> fmt::Debug for SetMultipleRequestHeadersLayer<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetMultipleRequestHeadersLayer")
            .field("headers", &self.headers)
            .finish()
    }
}

impl<M> SetMultipleRequestHeadersLayer<M> {
    /// Create a new [`SetMultipleRequestHeadersLayer`].
    ///
    /// If any previous value exists for the same header, it is removed and replaced with the new matching header value.
    pub fn overriding(metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Override))
            .collect();

        Self::new(headers)
    }

    /// Create a new [`SetMultipleRequestHeadersLayer`].
    ///
    /// The new header is always added, preserving any existing values. If previous values exist,
    /// the header will have multiple values.
    pub fn appending(metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Append))
            .collect();

        Self::new(headers)
    }

    /// Create a new [`SetMultipleRequestHeadersLayer`].
    ///
    /// If a previous value exists for the header, the new value is not inserted.
    pub fn if_not_present(metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::IfNotPresent))
            .collect();

        Self::new(headers)
    }

    /// Internal constructor for a new [`SetMultipleRequestHeadersLayer`] from a list of headers.
    fn new(headers: Vec<HeaderInsertionConfig<M>>) -> Self {
        Self { headers }
    }
}

impl<S, M> Layer<S> for SetMultipleRequestHeadersLayer<M> {
    type Service = SetMultipleRequestHeader<S, M>;

    fn layer(&self, inner: S) -> Self::Service {
        SetMultipleRequestHeader {
            inner,
            headers: self.headers.clone(),
        }
    }
}

/// Middleware that sets multiple headers on the request.
#[derive(Clone)]
pub struct SetMultipleRequestHeader<S, M> {
    inner: S,
    headers: Vec<HeaderInsertionConfig<M>>,
}

impl<S, M> SetMultipleRequestHeader<S, M> {
    /// Create a new [`SetMultipleRequestHeader`].
    ///
    /// If a previous value exists for the same header, it is removed and replaced with the new
    /// header value.
    pub fn overriding(inner: S, metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Override))
            .collect();

        Self::new(inner, headers)
    }

    /// Create a new [`SetMultipleRequestHeader`].
    ///
    /// The new header is always added, preserving any existing values. If previous values exist,
    /// the header will have multiple values.
    pub fn appending(inner: S, metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::Append))
            .collect();

        Self::new(inner, headers)
    }

    /// Create a new [`SetMultipleRequestHeader`].
    ///
    /// If a previous value exists for the header, the new value is not inserted.
    pub fn if_not_present(inner: S, metadata: Vec<HeaderMetadata<M>>) -> Self {
        let headers: Vec<HeaderInsertionConfig<M>> = metadata
            .into_iter()
            .map(|m| m.build_config(InsertHeaderMode::IfNotPresent))
            .collect();

        Self::new(inner, headers)
    }

    /// Internal constructor for a new [`SetMultipleRequestHeader`] from an inner service and a list of headers.
    fn new(inner: S, headers: Vec<HeaderInsertionConfig<M>>) -> Self {
        Self { inner, headers }
    }

    define_inner_service_accessors!();
}

impl<S, M> fmt::Debug for SetMultipleRequestHeader<S, M>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SetMultipleRequestHeader")
            .field("inner", &self.inner)
            .field("headers", &self.headers)
            .finish()
    }
}

impl<ReqBody, ResBody, S> Service<Request<ReqBody>>
    for SetMultipleRequestHeader<S, Request<ReqBody>>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        for header in &mut self.headers {
            header
                .mode
                .apply(&header.header_name, &mut req, &mut header.make);
        }

        self.inner.call(req)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/tower-http/unit/src/set_header/request/multiple_headers/tests_177.rs"));
