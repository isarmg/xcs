//! Frozen behavioral fingerprint captured from base64 0.22.1 before consolidation.
//! FNV-1a here is a deterministic test checksum, not a security primitive.
use base64 as codec;
use codec::Engine as _;
use std::io::Write;
struct Fingerprint(u64, usize);
impl Fingerprint {
    fn bytes(&mut self, bytes: &[u8]) {
        for b in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
    }
    fn decode(&mut self, engine: &codec::engine::GeneralPurpose, input: &[u8]) {
        self.1 += 1;
        match engine.decode(input) {
            Ok(bytes) => {
                self.bytes(&[0]);
                self.bytes(&bytes);
            }
            Err(error) => {
                let (tag, offset, symbol) = match error {
                    codec::DecodeError::InvalidByte(i, b) => (1, i, b),
                    codec::DecodeError::InvalidLength(n) => (2, n, 0),
                    codec::DecodeError::InvalidLastSymbol { offset, symbol, .. } => {
                        (3, offset, symbol)
                    }
                    codec::DecodeError::InvalidPadding => (4, 0, 0),
                };
                // 0.23 adds decoded symbol bits and changes diagnostic formatting.
                // Consumers depend on rejection, category, offset and offending byte.
                self.bytes(&[tag, symbol]);
                self.bytes(&(offset as u64).to_le_bytes());
            }
        }
    }
}
#[test]
fn scalar_base64_matches_retired_contract() {
    let mut fp = Fingerprint(0xcbf29ce484222325, 0);
    let engines = [
        codec::engine::general_purpose::STANDARD,
        codec::engine::general_purpose::STANDARD_NO_PAD,
        codec::engine::general_purpose::URL_SAFE,
        codec::engine::general_purpose::URL_SAFE_NO_PAD,
    ];
    for engine in &engines {
        let alphabet = b"ABZaz09+/-_= \n\0\xff";
        for len in 0..=5 {
            for mut n in 0..alphabet.len().pow(len) {
                let mut input = vec![0; len as usize];
                for x in &mut input {
                    *x = alphabet[n % alphabet.len()];
                    n /= alphabet.len();
                }
                fp.decode(engine, &input);
            }
        }
        for len in 0..=1024 {
            let input: Vec<u8> = (0..len)
                .map(|i| ((i * 157 + len * 31) % 256) as u8)
                .collect();
            let encoded = engine.encode(&input);
            fp.bytes(encoded.as_bytes());
            fp.decode(engine, encoded.as_bytes());
            for pos in [0, encoded.len() / 2, encoded.len().saturating_sub(1)] {
                if !encoded.is_empty() {
                    for bad in [b'=', b'!', b' ', b'\n', 0, 255] {
                        let mut bytes = encoded.as_bytes().to_vec();
                        bytes[pos] = bad;
                        fp.decode(engine, &bytes);
                    }
                }
            }
            let mut streamed = Vec::new();
            {
                let mut writer = codec::write::EncoderWriter::new(&mut streamed, engine);
                for chunk in input.chunks(7) {
                    writer.write_all(chunk).unwrap();
                }
                writer.finish().unwrap();
            }
            assert_eq!(streamed, encoded.as_bytes());
            fp.bytes(&streamed);
        }
    }
    assert_eq!(fp.1, 4_551_752);
    assert_eq!(fp.0, 0x4283_e9f0_20b5_6287);
}

#[test]
fn standard_scalar_caller_protocol_vectors() {
    let standard = codec::engine::general_purpose::STANDARD;
    // reqwest writes credentials in chunks, relying on EncoderWriter::drop.
    let mut authorization = b"Basic ".to_vec();
    {
        let mut writer = codec::write::EncoderWriter::new(&mut authorization, &standard);
        write!(writer, "Aladdin:").unwrap();
        write!(writer, "open sesame").unwrap();
    }
    assert_eq!(authorization, b"Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==");
    // RFC 6455 section 1.3's SHA-1 output and Sec-WebSocket-Accept encoding.
    let websocket_digest = [
        0xb3, 0x7a, 0x4f, 0x2c, 0xc0, 0x62, 0x4f, 0x16, 0x90, 0xf6, 0x46, 0x06, 0xcf, 0x38, 0x59,
        0x45, 0xb2, 0xbe, 0xc4, 0xea,
    ];
    assert_eq!(
        standard.encode(websocket_digest),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
    // Browser screenshot decoding must reject corrupt bytes and noncanonical tails.
    for invalid in ["a", "Zg", "Zh==", "Zm9=", "Zg===", "Z g==", "Zg==\n"] {
        assert!(standard.decode(invalid).is_err(), "accepted {invalid:?}");
    }
}
