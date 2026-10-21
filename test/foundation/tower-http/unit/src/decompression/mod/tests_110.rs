#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::io::Write;

    use super::*;
    use crate::test_helpers::Body;
    use crate::{compression::Compression, test_helpers::WithTrailers};
    use http::Response;
    use http::{HeaderMap, HeaderName, Request};
    use http_body_util::BodyExt;
    use tower::{service_fn, Service, ServiceExt};

    #[tokio::test]
    async fn works() {
        let mut client = Decompression::new(Compression::new(service_fn(handle)));

        let req = Request::builder()
            .header("accept-encoding", "gzip")
            .body(Body::empty())
            .unwrap();
        let res = client.ready().await.unwrap().call(req).await.unwrap();

        // read the body, it will be decompressed automatically
        let body = res.into_body();
        let collected = body.collect().await.unwrap();
        let trailers = collected.trailers().cloned().unwrap();
        let decompressed_data = String::from_utf8(collected.to_bytes().to_vec()).unwrap();

        assert_eq!(decompressed_data, "Hello, World!");

        // maintains trailers
        assert_eq!(trailers["foo"], "bar");
    }

    async fn handle(_req: Request<Body>) -> Result<Response<WithTrailers<Body>>, Infallible> {
        let mut trailers = HeaderMap::new();
        trailers.insert(HeaderName::from_static("foo"), "bar".parse().unwrap());
        let body = Body::from("Hello, World!").with_trailers(trailers);
        Ok(Response::builder().body(body).unwrap())
    }

    #[tokio::test]
    async fn decompress_multi_zstd() {
        let mut client = Decompression::new(service_fn(handle_multi_zstd));

        let req = Request::builder()
            .header("accept-encoding", "zstd")
            .body(Body::empty())
            .unwrap();
        let res = client.ready().await.unwrap().call(req).await.unwrap();

        // read the body, it will be decompressed automatically
        let body = res.into_body();
        let decompressed_data =
            String::from_utf8(body.collect().await.unwrap().to_bytes().to_vec()).unwrap();

        assert_eq!(decompressed_data, "Hello, World!");
    }

    async fn handle_multi_zstd(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let mut buf = Vec::new();
        let mut enc1 = zstd::Encoder::new(&mut buf, Default::default()).unwrap();
        enc1.write_all(b"Hello, ").unwrap();
        enc1.finish().unwrap();

        let mut enc2 = zstd::Encoder::new(&mut buf, Default::default()).unwrap();
        enc2.write_all(b"World!").unwrap();
        enc2.finish().unwrap();

        let mut res = Response::new(Body::from(buf));
        res.headers_mut()
            .insert("content-encoding", "zstd".parse().unwrap());
        Ok(res)
    }

    #[allow(dead_code)]
    async fn is_compatible_with_hyper() {
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();
        let mut client = Decompression::new(client);

        let req = Request::new(Body::empty());

        let _: Response<DecompressionBody<_>> =
            client.ready().await.unwrap().call(req).await.unwrap();
    }

    #[tokio::test]
    async fn decompress_empty() {
        let mut client = Decompression::new(Compression::new(service_fn(handle_empty)));

        let req = Request::builder()
            .header("accept-encoding", "gzip")
            .body(Body::empty())
            .unwrap();
        let res = client.ready().await.unwrap().call(req).await.unwrap();

        let body = res.into_body();
        let decompressed_data =
            String::from_utf8(body.collect().await.unwrap().to_bytes().to_vec()).unwrap();

        assert_eq!(decompressed_data, "");
    }

    async fn handle_empty(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        let mut res = Response::new(Body::empty());
        res.headers_mut()
            .insert("content-encoding", "gzip".parse().unwrap());
        Ok(res)
    }

    #[tokio::test]
    async fn decompress_empty_with_trailers() {
        let mut client =
            Decompression::new(Compression::new(service_fn(handle_empty_with_trailers)));

        let req = Request::builder()
            .header("accept-encoding", "gzip")
            .body(Body::empty())
            .unwrap();
        let res = client.ready().await.unwrap().call(req).await.unwrap();

        let body = res.into_body();
        let collected = body.collect().await.unwrap();
        let trailers = collected.trailers().cloned().unwrap();
        let decompressed_data = String::from_utf8(collected.to_bytes().to_vec()).unwrap();

        assert_eq!(decompressed_data, "");
        assert_eq!(trailers["foo"], "bar");
    }

    async fn handle_empty_with_trailers(
        _req: Request<Body>,
    ) -> Result<Response<WithTrailers<Body>>, Infallible> {
        let mut trailers = HeaderMap::new();
        trailers.insert(HeaderName::from_static("foo"), "bar".parse().unwrap());
        let body = Body::empty().with_trailers(trailers);
        Ok(Response::builder()
            .header("content-encoding", "gzip")
            .body(body)
            .unwrap())
    }
}
