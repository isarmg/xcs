#[cfg(test)]
mod tests {
    use super::{policy::*, *};
    use crate::test_helpers::Body;
    use http::header::LOCATION;
    use std::convert::Infallible;
    use service_stack::{ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn follows() {
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(Action::Follow))
            .buffer(1)
            .service_fn(handle);
        let req = Request::builder()
            .uri("http://example.com/42")
            .body(Body::empty())
            .unwrap();
        let res = svc.oneshot(req).await.unwrap();
        assert_eq!(*res.body(), 0);
        assert_eq!(
            res.extensions().get::<RequestUri>().unwrap().0,
            "http://example.com/0"
        );
    }

    #[tokio::test]
    async fn stops() {
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(Action::Stop))
            .buffer(1)
            .service_fn(handle);
        let req = Request::builder()
            .uri("http://example.com/42")
            .body(Body::empty())
            .unwrap();
        let res = svc.oneshot(req).await.unwrap();
        assert_eq!(*res.body(), 42);
        assert_eq!(
            res.extensions().get::<RequestUri>().unwrap().0,
            "http://example.com/42"
        );
    }

    #[tokio::test]
    async fn limited() {
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(Limited::new(10)))
            .buffer(1)
            .service_fn(handle);
        let req = Request::builder()
            .uri("http://example.com/42")
            .body(Body::empty())
            .unwrap();
        let res = svc.oneshot(req).await.unwrap();
        assert_eq!(*res.body(), 42 - 10);
        assert_eq!(
            res.extensions().get::<RequestUri>().unwrap().0,
            "http://example.com/32"
        );
    }

    /// A server with an endpoint `/{n}` which redirects to `/{n-1}` unless `n` equals zero,
    /// returning `n` as the response body.
    async fn handle<B>(req: Request<B>) -> Result<Response<u64>, Infallible> {
        let n: u64 = req.uri().path()[1..].parse().unwrap();
        let mut res = Response::builder();
        if n > 0 {
            res = res
                .status(StatusCode::MOVED_PERMANENTLY)
                .header(LOCATION, format!("/{}", n - 1));
        }
        Ok::<_, Infallible>(res.body(n).unwrap())
    }

    #[tokio::test]
    async fn test_301_redirects() {
        let policy = policy::redirect_fn(|attempt| -> Result<_, Infallible> {
            if attempt.previous_method() == Method::POST && attempt.method() == Method::GET {
                Ok(Action::Stop)
            } else {
                Ok(Action::Follow)
            }
        });
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(policy))
            .service_fn(redirections);

        // A POST request with a 301 redirection should turn into a GET
        // request, and the policy should stop the redirection.
        {
            let req = Request::builder()
                .method(Method::POST)
                .uri("http://example.com/301")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/301");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/301"
            );
        }

        // A GET request with a 301 redirection should remain a GET
        // request, and the policy should allow the redirection.
        {
            let req = Request::builder()
                .method(Method::GET)
                .uri("http://example.com/301")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/301/final");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/target/301"
            );
        }
    }

    #[tokio::test]
    async fn test_302_redirects() {
        let policy = policy::redirect_fn(|attempt| -> Result<_, Infallible> {
            if attempt.previous_method() != attempt.method() {
                Ok(Action::Stop)
            } else {
                Ok(Action::Follow)
            }
        });
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(policy))
            .service_fn(redirections);

        // A POST request with a 302 redirection should turn into a GET
        // request, and the policy should stop the redirection.
        {
            let req = Request::builder()
                .method(Method::POST)
                .uri("http://example.com/302")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/302");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/302"
            );
        }

        // A PUT request with a 302 redirection should remain a PUT
        // request, and the policy should allow the redirection.
        {
            let req = Request::builder()
                .method(Method::PUT)
                .uri("http://example.com/302")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/302/final");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/target/302"
            );
        }

        // A HEAD request with a 302 redirection should remain a HEAD
        // request, and the policy should allow the redirection.
        {
            let req = Request::builder()
                .method(Method::HEAD)
                .uri("http://example.com/302")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/302/final");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/target/302"
            );
        }
    }

    #[tokio::test]
    async fn test_303_redirects() {
        let policy = policy::redirect_fn(|attempt| -> Result<_, Infallible> {
            if attempt.previous_method() != attempt.method() {
                Ok(Action::Stop)
            } else {
                Ok(Action::Follow)
            }
        });
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(policy))
            .service_fn(redirections);

        // A POST request with a 303 redirection should turn into a GET
        // request, and the policy should stop the redirection.
        {
            let req = Request::builder()
                .method(Method::POST)
                .uri("http://example.com/303")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/303");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/303"
            );
        }

        // A PUT request with a 303 redirection should turn into a GET
        // request, and the policy should stop the redirection.
        {
            let req = Request::builder()
                .method(Method::PUT)
                .uri("http://example.com/303")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/303");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/303"
            );
        }

        // A HEAD request with a 303 redirection should remain a HEAD
        // request, and the policy should allow the redirection.
        {
            let req = Request::builder()
                .method(Method::HEAD)
                .uri("http://example.com/303")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/303/final");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/target/303"
            );
        }
    }

    #[tokio::test]
    async fn test_307_308_redirects() {
        let policy = policy::redirect_fn(|attempt| -> Result<_, Infallible> {
            if attempt.previous_method() != Method::POST || attempt.method() != Method::POST {
                Ok(Action::Stop)
            } else {
                Ok(Action::Follow)
            }
        });
        let svc = ServiceBuilder::new()
            .layer(FollowRedirectLayer::with_policy(policy))
            .service_fn(redirections);

        // A POST request with a 307 redirection should remain a POST
        // request, and the policy should allow the redirection.
        {
            let req = Request::builder()
                .method(Method::POST)
                .uri("http://example.com/307")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/307/final");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/target/307"
            );
        }

        // A POST request with a 308 redirection should remain a POST
        // request, and the policy should allow the redirection.
        {
            let req = Request::builder()
                .method(Method::POST)
                .uri("http://example.com/308")
                .body(Body::empty())
                .unwrap();
            let res = svc.clone().oneshot(req).await.unwrap();
            assert_eq!(*res.body(), "/target/308/final");
            assert_eq!(
                res.extensions().get::<RequestUri>().unwrap().0,
                "http://example.com/target/308"
            );
        }
    }

    /// Returns different 3xx redirections based on the request's URI.
    async fn redirections<B>(req: Request<B>) -> Result<Response<String>, Infallible> {
        let path = req.uri().path();
        let mut res = Response::builder();
        let body_str;
        res = match path {
            "/301" => {
                let case = "/target/301";
                body_str = case.to_string();
                res.status(StatusCode::MOVED_PERMANENTLY)
                    .header(LOCATION, case)
            }
            "/302" => {
                let case = "/target/302";
                body_str = case.to_string();
                res.status(StatusCode::FOUND).header(LOCATION, case)
            }
            "/303" => {
                let case = "/target/303";
                body_str = case.to_string();
                res.status(StatusCode::SEE_OTHER).header(LOCATION, case)
            }
            "/307" => {
                let case = "/target/307";
                body_str = case.to_string();
                res.status(StatusCode::TEMPORARY_REDIRECT)
                    .header(LOCATION, case)
            }
            "/308" => {
                let case = "/target/308";
                body_str = case.to_string();
                res.status(StatusCode::PERMANENT_REDIRECT)
                    .header(LOCATION, case)
            }
            v => {
                body_str = format!("{v}/final");
                res.status(StatusCode::OK)
            }
        };
        Ok::<_, Infallible>(res.body(body_str).unwrap())
    }

    #[tokio::test]
    async fn test_resolve_uri_unicode() {
        let base = Uri::from_static("https://example.com/api");
        // Case 1: Unicode in path
        let relative = "/café";
        let resolved = resolve_uri(relative, &base);
        assert!(resolved.is_some(), "Should resolve URI with unicode path");
        assert_eq!(
            resolved.unwrap().to_string(),
            "https://example.com/caf%C3%A9"
        );

        // Case 2: IDNA (Unicode in domain)
        let relative_domain = "https://münchen.com/";
        let resolved_domain = resolve_uri(relative_domain, &base);
        assert!(
            resolved_domain.is_some(),
            "Should resolve URI with unicode domain"
        );
        // München is encoded as punycode: xn--mnchen-3ya
        assert_eq!(
            resolved_domain.unwrap().to_string(),
            "https://xn--mnchen-3ya.com/"
        );
    }
}
