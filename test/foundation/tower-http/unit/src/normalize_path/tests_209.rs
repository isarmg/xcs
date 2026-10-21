#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use service_stack::{ServiceBuilder, ServiceExt};

    #[tokio::test]
    async fn trim_works() {
        async fn handle(request: Request<()>) -> Result<Response<String>, Infallible> {
            Ok(Response::new(request.uri().to_string()))
        }

        let mut svc = ServiceBuilder::new()
            .layer(NormalizePathLayer::trim_trailing_slash())
            .service_fn(handle);

        let body = svc
            .ready()
            .await
            .unwrap()
            .call(Request::builder().uri("/foo/").body(()).unwrap())
            .await
            .unwrap()
            .into_body();

        assert_eq!(body, "/foo");
    }

    #[test]
    fn is_noop_if_no_trailing_slash() {
        let mut uri = "/foo".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo");
    }

    #[test]
    fn maintains_query() {
        let mut uri = "/foo/?a=a".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo?a=a");
    }

    #[test]
    fn removes_multiple_trailing_slashes() {
        let mut uri = "/foo////".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo");
    }

    #[test]
    fn removes_multiple_trailing_slashes_even_with_query() {
        let mut uri = "/foo////?a=a".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo?a=a");
    }

    #[test]
    fn is_noop_on_index() {
        let mut uri = "/".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/");
    }

    #[test]
    fn removes_multiple_trailing_slashes_on_index() {
        let mut uri = "////".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/");
    }

    #[test]
    fn removes_multiple_trailing_slashes_on_index_even_with_query() {
        let mut uri = "////?a=a".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/?a=a");
    }

    #[test]
    fn removes_multiple_preceding_slashes_even_with_query() {
        let mut uri = "///foo//?a=a".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo?a=a");
    }

    #[test]
    fn removes_multiple_preceding_slashes() {
        let mut uri = "///foo".parse::<Uri>().unwrap();
        trim_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo");
    }

    #[tokio::test]
    async fn append_works() {
        async fn handle(request: Request<()>) -> Result<Response<String>, Infallible> {
            Ok(Response::new(request.uri().to_string()))
        }

        let mut svc = ServiceBuilder::new()
            .layer(NormalizePathLayer::append_trailing_slash())
            .service_fn(handle);

        let body = svc
            .ready()
            .await
            .unwrap()
            .call(Request::builder().uri("/foo").body(()).unwrap())
            .await
            .unwrap()
            .into_body();

        assert_eq!(body, "/foo/");
    }

    #[test]
    fn is_noop_if_trailing_slash() {
        let mut uri = "/foo/".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo/");
    }

    #[test]
    fn append_maintains_query() {
        let mut uri = "/foo?a=a".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo/?a=a");
    }

    #[test]
    fn append_only_keeps_one_slash() {
        let mut uri = "/foo////".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo/");
    }

    #[test]
    fn append_only_keeps_one_slash_even_with_query() {
        let mut uri = "/foo////?a=a".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo/?a=a");
    }

    #[test]
    fn append_is_noop_on_index() {
        let mut uri = "/".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/");
    }

    #[test]
    fn append_removes_multiple_trailing_slashes_on_index() {
        let mut uri = "////".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/");
    }

    #[test]
    fn append_removes_multiple_trailing_slashes_on_index_even_with_query() {
        let mut uri = "////?a=a".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/?a=a");
    }

    #[test]
    fn append_removes_multiple_preceding_slashes_even_with_query() {
        let mut uri = "///foo//?a=a".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo/?a=a");
    }

    #[test]
    fn append_removes_multiple_preceding_slashes() {
        let mut uri = "///foo".parse::<Uri>().unwrap();
        append_trailing_slash(&mut uri);
        assert_eq!(uri, "/foo/");
    }
}
