#[cfg(test)]
mod tests {
    use super::*;
    use http::{Method, Uri};

    #[test]
    fn works() {
        let mut policy = FilterCredentials::default();

        let initial = Uri::from_static("http://example.com/old");
        let same_origin = Uri::from_static("http://example.com/new");
        let cross_origin = Uri::from_static("https://example.com/new");

        let mut request = Request::builder()
            .uri(initial)
            .header(header::COOKIE, "42")
            .body(())
            .unwrap();
        Policy::<(), ()>::on_request(&mut policy, &mut request);
        assert!(request.headers().contains_key(header::COOKIE));

        let attempt = Attempt {
            status: Default::default(),
            method: &Method::GET,
            location: &same_origin,
            previous_method: &Method::GET,
            previous: request.uri(),
        };
        assert!(Policy::<(), ()>::redirect(&mut policy, &attempt)
            .unwrap()
            .is_follow());

        let mut request = Request::builder()
            .uri(same_origin)
            .header(header::COOKIE, "42")
            .body(())
            .unwrap();
        Policy::<(), ()>::on_request(&mut policy, &mut request);
        assert!(request.headers().contains_key(header::COOKIE));

        let attempt = Attempt {
            status: Default::default(),
            method: &Method::GET,
            location: &cross_origin,
            previous_method: &Method::GET,
            previous: request.uri(),
        };
        assert!(Policy::<(), ()>::redirect(&mut policy, &attempt)
            .unwrap()
            .is_follow());

        let mut request = Request::builder()
            .uri(cross_origin)
            .header(header::COOKIE, "42")
            .body(())
            .unwrap();
        Policy::<(), ()>::on_request(&mut policy, &mut request);
        assert!(!request.headers().contains_key(header::COOKIE));
    }
}
