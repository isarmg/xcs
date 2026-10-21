#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::Body;
    use http::{header::ACCEPT_ENCODING, Request, Response};
    use http_body::BodyExt;
    use std::convert::Infallible;
    use tokio::fs::File;
    use tokio_util::io::ReaderStream;
    use tower::{Service, ServiceBuilder, ServiceExt};

    async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        // Open the file.
        let file = File::open("Cargo.toml").await.expect("file missing");
        // Convert the file into a `Stream`.
        let stream = ReaderStream::new(file);
        // Convert the `Stream` into a `Body`.
        let body = Body::from_stream(stream);
        // Create response.
        Ok(Response::new(body))
    }

    #[tokio::test]
    async fn accept_encoding_configuration_works() -> Result<(), crate::BoxError> {
        use std::io::Read;

        fn decode<R: Read>(mut r: R) -> std::io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            r.read_to_end(&mut buf)?;
            Ok(buf)
        }

        // Read the source file once so we can verify each response round-trips to the same bytes.
        let expected = tokio::fs::read("Cargo.toml").await?;

        // Configure a layer that only offers deflate, then confirm the response is actually
        // deflate-encoded by decoding it and comparing to the original content.
        let deflate_only_layer = CompressionLayer::new()
            .quality(CompressionLevel::Best)
            .no_br()
            .no_gzip();

        let mut service = ServiceBuilder::new()
            .layer(deflate_only_layer)
            .service_fn(handle);

        let request = Request::builder()
            .header(ACCEPT_ENCODING, "gzip, deflate, br")
            .body(Body::empty())?;

        let response = service.ready().await?.call(request).await?;

        assert_eq!(response.headers()["content-encoding"], "deflate");

        let deflate_body = response.into_body().collect().await?.to_bytes();

        // The "deflate" Content-Encoding is RFC 1950 zlib framing (2-byte header + Adler-32),
        // not raw RFC 1951 deflate, so use ZlibDecoder rather than DeflateDecoder.
        let decoded = decode(flate2::bufread::ZlibDecoder::new(&deflate_body[..]))?;
        assert_eq!(decoded, expected);

        // Same check for brotli.
        let br_only_layer = CompressionLayer::new()
            .quality(CompressionLevel::Best)
            .no_gzip()
            .no_deflate();

        let mut service = ServiceBuilder::new()
            .layer(br_only_layer)
            .service_fn(handle);

        let request = Request::builder()
            .header(ACCEPT_ENCODING, "gzip, deflate, br")
            .body(Body::empty())?;

        let response = service.ready().await?.call(request).await?;

        assert_eq!(response.headers()["content-encoding"], "br");

        let br_body = response.into_body().collect().await?.to_bytes();

        // 4096 is the decoder's internal read-buffer size, not a content-length bound.
        let decoded = decode(brotli::Decompressor::new(&br_body[..], 4096))?;
        assert_eq!(decoded, expected);

        Ok(())
    }

    /// Test ensuring that zstd compression will not exceed an 8MiB window size; browsers do not
    /// accept responses using 16MiB+ window sizes.
    #[tokio::test]
    async fn zstd_is_web_safe() -> Result<(), crate::BoxError> {
        async fn zeroes(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
            Ok(Response::new(Body::from(vec![0u8; 18_874_368])))
        }
        // zstd will (I believe) lower its window size if a larger one isn't beneficial and
        // it knows the size of the input; use an 18MiB body to ensure it would want a
        // >=16MiB window (though it might not be able to see the input size here).

        let zstd_layer = CompressionLayer::new()
            .quality(CompressionLevel::Best)
            .no_br()
            .no_deflate()
            .no_gzip();

        let mut service = ServiceBuilder::new().layer(zstd_layer).service_fn(zeroes);

        let request = Request::builder()
            .header(ACCEPT_ENCODING, "zstd")
            .body(Body::empty())?;

        let response = service.ready().await?.call(request).await?;

        assert_eq!(response.headers()["content-encoding"], "zstd");

        let body = response.into_body();
        let bytes = body.collect().await?.to_bytes();
        let mut dec = zstd::Decoder::new(&*bytes)?;
        dec.window_log_max(23)?; // Limit window size accepted by decoder to 2 ^ 23 bytes (8MiB)

        std::io::copy(&mut dec, &mut std::io::sink())?;

        Ok(())
    }
}
