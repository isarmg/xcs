#[cfg(all(test, feature = "alloc", feature = "password-hash"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Algorithm, Argon2, Params, PasswordHasher, Salt, Version};

    /// Example password only: don't use this as a real password!!!
    const EXAMPLE_PASSWORD: &[u8] = b"hunter42";

    /// Example salt value. Don't use a static salt value!!!
    const EXAMPLE_SALT: &str = "examplesaltvalue";

    #[test]
    fn decoded_salt_too_short() {
        let argon2 = Argon2::default();

        // Too short after decoding
        let salt = Salt::from_b64("somesalt").unwrap();

        let res =
            argon2.hash_password_customized(EXAMPLE_PASSWORD, None, None, Params::default(), salt);
        assert_eq!(
            res,
            Err(password_hash::Error::SaltInvalid(
                password_hash::errors::InvalidValue::TooShort
            ))
        );
    }

    #[test]
    fn hash_simple_retains_configured_params() {
        // Non-default but valid parameters
        let t_cost = 4;
        let m_cost = 2048;
        let p_cost = 2;
        let version = Version::V0x10;

        let params = Params::new(m_cost, t_cost, p_cost, None).unwrap();
        let hasher = Argon2::new(Algorithm::default(), version, params);
        let salt = Salt::from_b64(EXAMPLE_SALT).unwrap();
        let hash = hasher.hash_password(EXAMPLE_PASSWORD, salt).unwrap();

        assert_eq!(hash.version.unwrap(), version.into());

        for &(param, value) in &[("t", t_cost), ("m", m_cost), ("p", p_cost)] {
            assert_eq!(
                hash.params
                    .get(param)
                    .and_then(|p| p.decimal().ok())
                    .unwrap(),
                value,
            );
        }
    }
}
