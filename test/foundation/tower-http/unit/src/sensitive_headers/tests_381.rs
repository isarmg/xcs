#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use http::header;
    use service_stack::{ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn multiple_value_header() {
        async fn response_set_cookie(req: http::Request<()>) -> Result<http::Response<()>, ()> {
            let mut iter = req.headers().get_all(header::COOKIE).iter().peekable();

            assert!(iter.peek().is_some());

            for value in iter {
                assert!(value.is_sensitive())
            }

            let mut resp = http::Response::new(());
            resp.headers_mut().append(
                header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/html"),
            );
            resp.headers_mut().append(
                header::SET_COOKIE,
                http::HeaderValue::from_static("cookie-1"),
            );
            resp.headers_mut().append(
                header::SET_COOKIE,
                http::HeaderValue::from_static("cookie-2"),
            );
            resp.headers_mut().append(
                header::SET_COOKIE,
                http::HeaderValue::from_static("cookie-3"),
            );
            Ok(resp)
        }

        let mut service = ServiceBuilder::new()
            .layer(SetSensitiveRequestHeadersLayer::new(vec![header::COOKIE]))
            .layer(SetSensitiveResponseHeadersLayer::new(vec![
                header::SET_COOKIE,
            ]))
            .service_fn(response_set_cookie);

        let mut req = http::Request::new(());
        req.headers_mut()
            .append(header::COOKIE, http::HeaderValue::from_static("cookie+1"));
        req.headers_mut()
            .append(header::COOKIE, http::HeaderValue::from_static("cookie+2"));

        let resp = service.ready().await.unwrap().call(req).await.unwrap();

        assert!(!resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .is_sensitive());

        let mut iter = resp.headers().get_all(header::SET_COOKIE).iter().peekable();

        assert!(iter.peek().is_some());

        for value in iter {
            assert!(value.is_sensitive())
        }
    }
}
