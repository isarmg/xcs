#[cfg(test)]
mod tests {
    use super::Redirect;
    use axum_core::response::IntoResponse;
    use http::StatusCode;

    const EXAMPLE_URL: &str = "https://example.com";

    // Tests to make sure Redirect has the correct status codes
    // based on the way it was constructed.
    #[test]
    fn correct_status() {
        assert_eq!(
            StatusCode::SEE_OTHER,
            Redirect::to(EXAMPLE_URL).status_code()
        );

        assert_eq!(
            StatusCode::TEMPORARY_REDIRECT,
            Redirect::temporary(EXAMPLE_URL).status_code()
        );

        assert_eq!(
            StatusCode::PERMANENT_REDIRECT,
            Redirect::permanent(EXAMPLE_URL).status_code()
        );
    }

    #[test]
    fn correct_location() {
        assert_eq!(EXAMPLE_URL, Redirect::permanent(EXAMPLE_URL).location());

        assert_eq!("/redirect", Redirect::permanent("/redirect").location())
    }

    #[test]
    fn test_internal_error() {
        let response = Redirect::permanent("Axum is awesome, \n but newlines aren't allowed :(")
            .into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
