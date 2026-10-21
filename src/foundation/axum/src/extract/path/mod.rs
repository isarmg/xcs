//! Extractor that will get captures from the URL and parse them using
//! [`serde`].

mod de;

use crate::{
    extract::{rejection::*, FromRequestParts},
    routing::url_params::UrlParams,
    util::PercentDecodedStr,
};
use axum_core::{
    extract::OptionalFromRequestParts,
    response::{IntoResponse, Response},
    RequestPartsExt as _,
};
use http::{request::Parts, StatusCode};
use serialization_core::de::DeserializeOwned;
use std::{fmt, sync::Arc};

/// Extractor that will get captures from the URL and parse them using
/// [`serde`].
///
/// Any percent encoded parameters will be automatically decoded. The decoded
/// parameters must be valid UTF-8, otherwise `Path` will fail and return a `400
/// Bad Request` response.
///
/// # `Option<Path<T>>` behavior
///
/// You can use `Option<Path<T>>` as an extractor to allow the same handler to
/// be used in a route with parameters that deserialize to `T`, and another
/// route with no parameters at all.
///
/// # Example
///
/// These examples assume the `serde` feature of the [`uuid`] crate is enabled.
///
/// One `Path` can extract multiple captures. It is not necessary (and does
/// not work) to give a handler more than one `Path` argument.
///
/// [`uuid`]: https://crates.io/crates/uuid
///
/// ```rust,no_run
/// use axum::{
///     extract::Path,
///     routing::get,
///     Router,
/// };
/// use uuid::Uuid;
///
/// async fn users_teams_show(
///     Path((user_id, team_id)): Path<(Uuid, Uuid)>,
/// ) {
///     // ...
/// }
///
/// let app = Router::new().route("/users/{user_id}/team/{team_id}", get(users_teams_show));
/// # let _: Router = app;
/// ```
///
/// If the path contains only one parameter, then you can omit the tuple.
///
/// ```rust,no_run
/// use axum::{
///     extract::Path,
///     routing::get,
///     Router,
/// };
/// use uuid::Uuid;
///
/// async fn user_info(Path(user_id): Path<Uuid>) {
///     // ...
/// }
///
/// let app = Router::new().route("/users/{user_id}", get(user_info));
/// # let _: Router = app;
/// ```
///
/// Path segments also can be deserialized into any type that implements
/// [`data_serialization::Deserialize`]. This includes tuples and structs:
///
/// ```rust,no_run
/// use axum::{
///     extract::Path,
///     routing::get,
///     Router,
/// };
/// use data_serialization::Deserialize;
/// use uuid::Uuid;
///
/// // Path segment labels will be matched with struct field names
/// #[derive(Deserialize)]
/// struct Params {
///     user_id: Uuid,
///     team_id: Uuid,
/// }
///
/// async fn users_teams_show(
///     Path(Params { user_id, team_id }): Path<Params>,
/// ) {
///     // ...
/// }
///
/// // When using tuples the path segments will be matched by their position in the route
/// async fn users_teams_create(
///     Path((user_id, team_id)): Path<(String, String)>,
/// ) {
///     // ...
/// }
///
/// let app = Router::new().route(
///     "/users/{user_id}/team/{team_id}",
///     get(users_teams_show).post(users_teams_create),
/// );
/// # let _: Router = app;
/// ```
///
/// If you wish to capture all path parameters you can use `HashMap` or `Vec`:
///
/// ```rust,no_run
/// use axum::{
///     extract::Path,
///     routing::get,
///     Router,
/// };
/// use std::collections::HashMap;
///
/// async fn params_map(
///     Path(params): Path<HashMap<String, String>>,
/// ) {
///     // ...
/// }
///
/// async fn params_vec(
///     Path(params): Path<Vec<(String, String)>>,
/// ) {
///     // ...
/// }
///
/// let app = Router::new()
///     .route("/users/{user_id}/team/{team_id}", get(params_map).post(params_vec));
/// # let _: Router = app;
/// ```
///
/// # Providing detailed rejection output
///
/// If the URI cannot be deserialized into the target type the request will be rejected and an
/// error response will be returned. See [`customize-path-rejection`] for an example of how to customize that error.
///
/// [`serde`]: https://crates.io/crates/serde
/// [`data_serialization::Deserialize`]: https://docs.rs/serde/1.0.127/serde/trait.Deserialize.html
/// [`customize-path-rejection`]: https://github.com/tokio-rs/axum/blob/main/examples/customize-path-rejection/src/main.rs
#[derive(Debug)]
pub struct Path<T>(pub T);

axum_core::__impl_deref!(Path);

impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = PathRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extracted into separate fn so it's only compiled once for all T.
        fn get_params(parts: &Parts) -> Result<&[(Arc<str>, PercentDecodedStr)], PathRejection> {
            match parts.extensions.get::<UrlParams>() {
                Some(UrlParams::Params(params)) => Ok(params),
                Some(UrlParams::InvalidUtf8InPathParam { key }) => {
                    let err = PathDeserializationError {
                        kind: ErrorKind::InvalidUtf8InPathParam {
                            key: key.to_string(),
                        },
                    };
                    Err(FailedToDeserializePathParams(err).into())
                }
                None => Err(MissingPathParams.into()),
            }
        }

        fn failed_to_deserialize_path_params(err: PathDeserializationError) -> PathRejection {
            PathRejection::FailedToDeserializePathParams(FailedToDeserializePathParams(err))
        }

        match T::deserialize(de::PathDeserializer::new(get_params(parts)?)) {
            Ok(val) => Ok(Path(val)),
            Err(e) => Err(failed_to_deserialize_path_params(e)),
        }
    }
}

impl<T, S> OptionalFromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send + 'static,
    S: Send + Sync,
{
    type Rejection = PathRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        match parts.extract::<Self>().await {
            Ok(Self(params)) => Ok(Some(Self(params))),
            Err(PathRejection::FailedToDeserializePathParams(e))
                if matches!(e.kind(), ErrorKind::WrongNumberOfParameters { got: 0, .. }) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

// this wrapper type is used as the deserializer error to hide the `data_serialization::de::Error` impl which
// would otherwise be public if we used `ErrorKind` as the error directly
#[derive(Debug)]
pub(crate) struct PathDeserializationError {
    pub(super) kind: ErrorKind,
}

impl PathDeserializationError {
    pub(super) fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    pub(super) fn wrong_number_of_parameters() -> WrongNumberOfParameters<()> {
        WrongNumberOfParameters { got: () }
    }

    #[track_caller]
    pub(super) fn unsupported_type(name: &'static str) -> Self {
        Self::new(ErrorKind::UnsupportedType { name })
    }
}

pub(super) struct WrongNumberOfParameters<G> {
    got: G,
}

impl<G> WrongNumberOfParameters<G> {
    #[allow(clippy::unused_self)]
    pub(super) fn got<G2>(self, got: G2) -> WrongNumberOfParameters<G2> {
        WrongNumberOfParameters { got }
    }
}

impl WrongNumberOfParameters<usize> {
    pub(super) fn expected(self, expected: usize) -> PathDeserializationError {
        PathDeserializationError::new(ErrorKind::WrongNumberOfParameters {
            got: self.got,
            expected,
        })
    }
}

impl serialization_core::de::Error for PathDeserializationError {
    #[inline]
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self {
            kind: ErrorKind::Message(msg.to_string()),
        }
    }
}

impl fmt::Display for PathDeserializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for PathDeserializationError {}

/// The kinds of errors that can happen we deserializing into a [`Path`].
///
/// This type is obtained through [`FailedToDeserializePathParams::kind`] or
/// [`FailedToDeserializePathParams::into_kind`] and is useful for building
/// more precise error messages.
#[must_use]
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The URI contained the wrong number of parameters.
    WrongNumberOfParameters {
        /// The number of actual parameters in the URI.
        got: usize,
        /// The number of expected parameters.
        expected: usize,
    },

    /// Failed to parse the value at a specific key into the expected type.
    ///
    /// This variant is used when deserializing into types that have named fields, such as structs.
    ParseErrorAtKey {
        /// The key at which the value was located.
        key: String,
        /// The value from the URI.
        value: String,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// Failed to parse the value at a specific index into the expected type.
    ///
    /// This variant is used when deserializing into sequence types, such as tuples.
    ParseErrorAtIndex {
        /// The index at which the value was located.
        index: usize,
        /// The value from the URI.
        value: String,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// Failed to parse a value into the expected type.
    ///
    /// This variant is used when deserializing into a primitive type (such as `String` and `u32`).
    ParseError {
        /// The value from the URI.
        value: String,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// A parameter contained text that, once percent decoded, wasn't valid UTF-8.
    InvalidUtf8InPathParam {
        /// The key at which the invalid value was located.
        key: String,
    },

    /// Tried to serialize into an unsupported type such as nested maps.
    ///
    /// This error kind is caused by programmer errors and thus gets converted into a `500 Internal
    /// Server Error` response.
    UnsupportedType {
        /// The name of the unsupported type.
        name: &'static str,
    },

    /// Failed to deserialize the value with a custom deserialization error.
    DeserializeError {
        /// The key at which the invalid value was located.
        key: String,
        /// The value that failed to deserialize.
        value: String,
        /// The deserializaation failure message.
        message: String,
    },

    /// Catch-all variant for errors that don't fit any other variant.
    Message(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Message(error) => error.fmt(f),
            ErrorKind::InvalidUtf8InPathParam { key } => write!(f, "Invalid UTF-8 in `{key}`"),
            ErrorKind::WrongNumberOfParameters { got, expected } => {
                write!(
                    f,
                    "Wrong number of path arguments for `Path`. Expected {expected} but got {got}"
                )?;

                if *expected == 1 {
                    write!(f, ". Note that multiple parameters must be extracted with a tuple `Path<(_, _)>` or a struct `Path<YourParams>`")?;
                }

                Ok(())
            }
            ErrorKind::UnsupportedType { name } => write!(f, "Unsupported type `{name}`"),
            ErrorKind::ParseErrorAtKey {
                key,
                value,
                expected_type,
            } => write!(
                f,
                "Cannot parse `{key}` with value `{value}` to a `{expected_type}`"
            ),
            ErrorKind::ParseError {
                value,
                expected_type,
            } => write!(f, "Cannot parse `{value}` to a `{expected_type}`"),
            ErrorKind::ParseErrorAtIndex {
                index,
                value,
                expected_type,
            } => write!(
                f,
                "Cannot parse value at index {index} with value `{value}` to a `{expected_type}`"
            ),
            ErrorKind::DeserializeError {
                key,
                value,
                message,
            } => write!(f, "Cannot parse `{key}` with value `{value}`: {message}"),
        }
    }
}

/// Rejection type for [`Path`] if the captured routes params couldn't be deserialized
/// into the expected type.
#[derive(Debug)]
pub struct FailedToDeserializePathParams(PathDeserializationError);

impl FailedToDeserializePathParams {
    /// Get a reference to the underlying error kind.
    pub fn kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    /// Convert this error into the underlying error kind.
    pub fn into_kind(self) -> ErrorKind {
        self.0.kind
    }

    /// Get the response body text used for this rejection.
    #[must_use]
    pub fn body_text(&self) -> String {
        match self.0.kind {
            ErrorKind::Message(_)
            | ErrorKind::DeserializeError { .. }
            | ErrorKind::InvalidUtf8InPathParam { .. }
            | ErrorKind::ParseError { .. }
            | ErrorKind::ParseErrorAtIndex { .. }
            | ErrorKind::ParseErrorAtKey { .. } => format!("Invalid URL: {}", self.0.kind),
            ErrorKind::WrongNumberOfParameters { .. } | ErrorKind::UnsupportedType { .. } => {
                self.0.kind.to_string()
            }
        }
    }

    /// Get the status code used for this rejection.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self.0.kind {
            ErrorKind::Message(_)
            | ErrorKind::DeserializeError { .. }
            | ErrorKind::InvalidUtf8InPathParam { .. }
            | ErrorKind::ParseError { .. }
            | ErrorKind::ParseErrorAtIndex { .. }
            | ErrorKind::ParseErrorAtKey { .. } => StatusCode::BAD_REQUEST,
            ErrorKind::WrongNumberOfParameters { .. } | ErrorKind::UnsupportedType { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for FailedToDeserializePathParams {
    fn into_response(self) -> Response {
        let body = self.body_text();
        axum_core::__log_rejection!(
            rejection_type = Self,
            body_text = body,
            status = self.status(),
        );
        (self.status(), body).into_response()
    }
}

impl fmt::Display for FailedToDeserializePathParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for FailedToDeserializePathParams {}

/// Extractor that will get captures from the URL without deserializing them.
///
/// In general you should prefer to use [`Path`] as it is higher level, however `RawPathParams` is
/// suitable if just want the raw params without deserializing them and thus saving some
/// allocations.
///
/// Any percent encoded parameters will be automatically decoded. The decoded parameters must be
/// valid UTF-8, otherwise `RawPathParams` will fail and return a `400 Bad Request` response.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{
///     extract::RawPathParams,
///     routing::get,
///     Router,
/// };
///
/// async fn users_teams_show(params: RawPathParams) {
///     for (key, value) in &params {
///         println!("{key:?} = {value:?}");
///     }
/// }
///
/// let app = Router::new().route("/users/{user_id}/team/{team_id}", get(users_teams_show));
/// # let _: Router = app;
/// ```
#[derive(Debug)]
pub struct RawPathParams(Vec<(Arc<str>, PercentDecodedStr)>);

impl<S> FromRequestParts<S> for RawPathParams
where
    S: Send + Sync,
{
    type Rejection = RawPathParamsRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let params = match parts.extensions.get::<UrlParams>() {
            Some(UrlParams::Params(params)) => params,
            Some(UrlParams::InvalidUtf8InPathParam { key }) => {
                return Err(InvalidUtf8InPathParam {
                    key: Arc::clone(key),
                }
                .into());
            }
            None => {
                return Err(MissingPathParams.into());
            }
        };

        Ok(Self(params.clone()))
    }
}

impl RawPathParams {
    /// Get an iterator over the path parameters.
    #[must_use]
    pub fn iter(&self) -> RawPathParamsIter<'_> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a RawPathParams {
    type Item = (&'a str, &'a str);
    type IntoIter = RawPathParamsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        RawPathParamsIter(self.0.iter())
    }
}

/// An iterator over raw path parameters.
///
/// Created with [`RawPathParams::iter`].
#[derive(Debug)]
pub struct RawPathParamsIter<'a>(std::slice::Iter<'a, (Arc<str>, PercentDecodedStr)>);

impl<'a> Iterator for RawPathParamsIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = self.0.next()?;
        Some((&**key, value.as_str()))
    }
}

/// Rejection used by [`RawPathParams`] if a parameter contained text that, once percent decoded,
/// wasn't valid UTF-8.
#[derive(Debug)]
pub struct InvalidUtf8InPathParam {
    key: Arc<str>,
}

impl InvalidUtf8InPathParam {
    /// Get the response body text used for this rejection.
    #[must_use]
    pub fn body_text(&self) -> String {
        self.to_string()
    }

    /// Get the status code used for this rejection.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

impl fmt::Display for InvalidUtf8InPathParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid UTF-8 in `{}`", self.key)
    }
}

impl std::error::Error for InvalidUtf8InPathParam {}

impl IntoResponse for InvalidUtf8InPathParam {
    fn into_response(self) -> Response {
        let body = self.body_text();
        axum_core::__log_rejection!(
            rejection_type = Self,
            body_text = body,
            status = self.status(),
        );
        (self.status(), body).into_response()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/axum/unit/src/extract/path/mod/tests_600.rs"));
