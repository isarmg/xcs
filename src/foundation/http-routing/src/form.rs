use crate::extract::Request;
use crate::extract::{rejection::*, FromRequest, RawForm};
use crate::kernel::response::{IntoResponse, Response};
use crate::kernel::RequestExt;
use http::header::CONTENT_TYPE;
use http::StatusCode;
use serialization_core::{de::DeserializeOwned, Serialize};

/// URL encoded extractor and response.
///
/// # As extractor
///
/// If used as an extractor, `Form` will deserialize form data from the request,
/// specifically:
///
/// - If the request has a method of `GET` or `HEAD`, the form data will be read
///   from the query string (same as with [`Query`])
/// - If the request has a different method, the form will be read from the body
///   of the request. It must have a `content-type` of
///   `application/x-www-form-urlencoded` for this to work. This component does
///   not provide a `multipart/form-data` parser.
///
/// This matches how HTML forms are sent by browsers by default.
/// In both cases, the inner type `T` must implement [`data_serialization::Deserialize`].
///
/// ⚠️ Since parsing form data might require consuming the request body, the `Form` extractor must be
/// *last* if there are multiple extractors in a handler. See ["the order of
/// extractors"][order-of-extractors]
///
/// [order-of-extractors]: crate::extract#the-order-of-extractors
///
/// ```rust
/// use http_routing::Form;
/// use data_serialization::Deserialize;
///
/// #[derive(Deserialize)]
/// struct SignUp {
///     username: String,
///     password: String,
/// }
///
/// async fn accept_form(Form(sign_up): Form<SignUp>) {
///     // ...
/// }
/// ```
///
/// # As response
///
/// `Form` can also be used to encode any type that implements
/// [`data_serialization::Serialize`] as `application/x-www-form-urlencoded`
///
/// ```rust
/// use http_routing::Form;
/// use data_serialization::Serialize;
///
/// #[derive(Serialize)]
/// struct Payload {
///     value: String,
/// }
///
/// async fn handler() -> Form<Payload> {
///     Form(Payload { value: "foo".to_owned() })
/// }
/// ```
///
/// [`Query`]: crate::extract::Query
#[cfg_attr(docsrs, doc(cfg(feature = "form")))]
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Form<T>(pub T);

impl<T, S> FromRequest<S> for Form<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = FormRejection;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let is_get_or_head =
            req.method() == http::Method::GET || req.method() == http::Method::HEAD;

        match req.extract().await {
            Ok(RawForm(bytes)) => {
                let deserializer = form_codec::Deserializer::new(form_urlencoded::parse(&bytes));
                let value = serialization_error_path::deserialize(deserializer).map_err(
                    |err| -> FormRejection {
                        if is_get_or_head {
                            FailedToDeserializeForm::from_err(err).into()
                        } else {
                            FailedToDeserializeFormBody::from_err(err).into()
                        }
                    },
                )?;
                Ok(Form(value))
            }
            Err(RawFormRejection::BytesRejection(r)) => Err(FormRejection::BytesRejection(r)),
            Err(RawFormRejection::InvalidFormContentType(r)) => {
                Err(FormRejection::InvalidFormContentType(r))
            }
        }
    }
}

impl<T> IntoResponse for Form<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // Extracted into separate fn so it's only compiled once for all T.
        fn make_response(ser_result: Result<String, form_codec::ser::Error>) -> Response {
            match ser_result {
                Ok(body) => (
                    [(CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())],
                    body,
                )
                    .into_response(),
                Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
            }
        }

        make_response(form_codec::to_string(&self.0))
    }
}
crate::__impl_deref!(Form);

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/http-routing/unit/src/form/tests_129.rs"
));
