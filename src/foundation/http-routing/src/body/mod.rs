//! HTTP body utilities.

#[doc(no_inline)]
pub use http_body::Body as HttpBody;

#[doc(no_inline)]
pub use bytes::Bytes;

#[doc(inline)]
pub use crate::kernel::body::{Body, BodyDataStream};

use http_body::{BodyExt, Limited};

/// Converts [`Body`] into [`Bytes`] and limits the maximum size of the body.
///
/// # Example
///
/// ```rust
/// use http_routing::body::{to_bytes, Body};
///
/// # async fn foo() -> Result<(), crate::kernel::Error> {
/// let body = Body::from(vec![1, 2, 3]);
/// // Use `usize::MAX` if you don't care about the maximum size.
/// let bytes = to_bytes(body, usize::MAX).await?;
/// assert_eq!(&bytes[..], &[1, 2, 3]);
/// # Ok(())
/// # }
/// ```
///
/// You can detect if the limit was hit by checking the source of the error:
///
/// ```rust
/// use http_routing::body::{to_bytes, Body};
/// use http_body::LengthLimitError;
///
/// # #[tokio::main]
/// # async fn main() {
/// let body = Body::from(vec![1, 2, 3]);
/// match to_bytes(body, 1).await {
///     Ok(_bytes) => panic!("should have hit the limit"),
///     Err(err) => {
///         let source = std::error::Error::source(&err).unwrap();
///         assert!(source.is::<LengthLimitError>());
///     }
/// }
/// # }
/// ```
pub async fn to_bytes(body: Body, limit: usize) -> Result<Bytes, crate::kernel::Error> {
    Limited::new(body, limit)
        .collect()
        .await
        .map(|col| col.to_bytes())
        .map_err(crate::kernel::Error::new)
}
