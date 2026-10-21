#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_encrypt_action_checks_limits() {
        let action = |write_seq_max, write_seq, add| {
            let mut record_layer = RecordLayer::new();
            record_layer.write_seq_max = write_seq_max;
            record_layer.write_seq = write_seq;
            record_layer.pre_encrypt_action(add)
        };

        assert_eq!(action(10, 8, 1), PreEncryptAction::Nothing);
        assert_eq!(action(10, 8, 2), PreEncryptAction::RefreshOrClose);
        assert_eq!(action(10, 11, 0), PreEncryptAction::RefreshOrClose);
        assert_eq!(
            action(SEQ_SOFT_LIMIT, SEQ_HARD_LIMIT - 1, 0),
            PreEncryptAction::RefreshOrClose
        );
        assert_eq!(
            action(SEQ_SOFT_LIMIT, SEQ_HARD_LIMIT - 1, 1),
            PreEncryptAction::Refuse
        );
    }

    #[test]
    fn test_has_decrypted() {
        use crate::{ContentType, ProtocolVersion};

        struct PassThroughDecrypter;
        impl MessageDecrypter for PassThroughDecrypter {
            fn decrypt<'a>(
                &mut self,
                m: InboundOpaqueMessage<'a>,
                _: u64,
            ) -> Result<InboundPlainMessage<'a>, Error> {
                Ok(m.into_plain_message())
            }
        }

        // A record layer starts out invalid, having never decrypted.
        let mut record_layer = RecordLayer::new();
        assert!(matches!(
            record_layer.decrypt_state,
            DirectionState::Invalid
        ));
        assert_eq!(record_layer.read_seq, 0);
        assert!(!record_layer.has_decrypted());

        // Preparing the record layer should update the decrypt state, but shouldn't affect whether it
        // has decrypted.
        record_layer.prepare_message_decrypter(Box::new(PassThroughDecrypter));
        assert!(matches!(
            record_layer.decrypt_state,
            DirectionState::Prepared
        ));
        assert_eq!(record_layer.read_seq, 0);
        assert!(!record_layer.has_decrypted());

        // Starting decryption should update the decrypt state, but not affect whether it has decrypted.
        record_layer.start_decrypting();
        assert!(matches!(record_layer.decrypt_state, DirectionState::Active));
        assert_eq!(record_layer.read_seq, 0);
        assert!(!record_layer.has_decrypted());

        // Decrypting a message should update the read_seq and track that we have now performed
        // a decryption.
        record_layer
            .decrypt_incoming(InboundOpaqueMessage::new(
                ContentType::Handshake,
                ProtocolVersion::TLSv1_2,
                &mut [0xC0, 0xFF, 0xEE],
            ))
            .unwrap();
        assert!(matches!(record_layer.decrypt_state, DirectionState::Active));
        assert_eq!(record_layer.read_seq, 1);
        assert!(record_layer.has_decrypted());

        // Resetting the record layer message decrypter (as if a key update occurred) should reset
        // the read_seq number, but not our knowledge of whether we have decrypted previously.
        record_layer.set_message_decrypter(Box::new(PassThroughDecrypter));
        assert!(matches!(record_layer.decrypt_state, DirectionState::Active));
        assert_eq!(record_layer.read_seq, 0);
        assert!(record_layer.has_decrypted());
    }
}
