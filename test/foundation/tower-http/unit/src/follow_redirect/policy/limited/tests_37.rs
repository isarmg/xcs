#[cfg(test)]
mod tests {
    use http::{Method, Request, Uri};

    use super::*;

    #[test]
    fn works() {
        let uri = Uri::from_static("https://example.com/");
        let mut policy = Limited::new(2);

        for _ in 0..2 {
            let mut request = Request::builder().uri(uri.clone()).body(()).unwrap();
            Policy::<(), ()>::on_request(&mut policy, &mut request);

            let attempt = Attempt {
                status: Default::default(),
                method: &Method::GET,
                location: &uri,
                previous_method: &Method::GET,
                previous: &uri,
            };
            assert!(Policy::<(), ()>::redirect(&mut policy, &attempt)
                .unwrap()
                .is_follow());
        }

        let mut request = Request::builder().uri(uri.clone()).body(()).unwrap();
        Policy::<(), ()>::on_request(&mut policy, &mut request);

        let attempt = Attempt {
            status: Default::default(),
            method: &Method::GET,
            location: &uri,
            previous_method: &Method::GET,
            previous: &uri,
        };
        assert!(Policy::<(), ()>::redirect(&mut policy, &attempt)
            .unwrap()
            .is_stop());
    }
}
