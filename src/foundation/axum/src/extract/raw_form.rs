use axum_core::extract::{FromRequest, Request};
use bytes::Bytes;
use http::Method;

use super::{
    has_content_type,
    rejection::{InvalidFormContentType, RawFormRejection},
};

/// Extractor that extracts raw form requests.
///
/// For `GET` requests it will extract the raw query. For other methods it extracts the raw
/// `application/x-www-form-urlencoded` encoded request body.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{
///     extract::RawForm,
///     routing::get,
///     Router
/// };
///
/// async fn handler(RawForm(form): RawForm) {}
///
/// let app = Router::new().route("/", get(handler));
/// # let _: Router = app;
/// ```
#[derive(Debug)]
pub struct RawForm(pub Bytes);

impl<S> FromRequest<S> for RawForm
where
    S: Send + Sync,
{
    type Rejection = RawFormRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        if req.method() == Method::GET {
            if let Some(query) = req.uri().query() {
                return Ok(Self(Bytes::copy_from_slice(query.as_bytes())));
            }

            Ok(Self(Bytes::new()))
        } else {
            if !has_content_type(req.headers(), &mime::APPLICATION_WWW_FORM_URLENCODED) {
                return Err(InvalidFormContentType.into());
            }

            Ok(Self(Bytes::from_request(req, state).await?))
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/axum/unit/src/extract/raw_form/tests_55.rs"));
