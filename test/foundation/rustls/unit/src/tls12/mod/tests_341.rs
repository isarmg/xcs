#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use super::provider::kx_group::X25519;
    use super::*;
    use crate::common_state::{CommonState, Side};
    use crate::msgs::handshake::{ServerEcdhParams, ServerKeyExchangeParams};

    #[test]
    fn server_ecdhe_remaining_bytes() {
        let key = X25519.start().unwrap();
        let server_params = ServerEcdhParams::new(&*key);
        let mut server_buf = Vec::new();
        server_params.encode(&mut server_buf);
        server_buf.push(34);

        let mut common = CommonState::new(Side::Client);
        assert!(
            decode_kx_params::<ServerKeyExchangeParams>(
                KeyExchangeAlgorithm::ECDHE,
                &mut common,
                &server_buf
            )
            .is_err()
        );
    }

    #[test]
    fn client_ecdhe_invalid() {
        let mut common = CommonState::new(Side::Server);
        assert!(
            decode_kx_params::<ServerKeyExchangeParams>(
                KeyExchangeAlgorithm::ECDHE,
                &mut common,
                &[34],
            )
            .is_err()
        );
    }
}
