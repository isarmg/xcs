#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_methods() {
        let options = HttpOptions::new()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(true)
            .add_root_certificate("/path/to/cert.pem");

        assert_eq!(options.connect_timeout, Some(Duration::from_secs(10)));
        assert_eq!(options.timeout, Some(Duration::from_secs(30)));
        assert!(options.danger_accept_invalid_certs);
        assert_eq!(
            options.root_certificate,
            Some(PathBuf::from("/path/to/cert.pem"))
        );
    }
}
