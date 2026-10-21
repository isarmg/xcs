#[cfg(all(test, feature = "ring"))]
mod tests {
    use crate::crypto::hmac::Hmac;
    // nb: crypto::aws_lc_rs provider doesn't provide (or need) hmac,
    // so cannot be used for this test.
    use crate::crypto::ring::hmac;

    // Below known answer tests come from https://mailarchive.ietf.org/arch/msg/tls/fzVCzk-z3FShgGJ6DOXqM1ydxms/

    #[test]
    fn check_sha256() {
        let secret = b"\x9b\xbe\x43\x6b\xa9\x40\xf0\x17\xb1\x76\x52\x84\x9a\x71\xdb\x35";
        let seed = b"\xa0\xba\x9f\x93\x6c\xda\x31\x18\x27\xa6\xf7\x96\xff\xd5\x19\x8c";
        let label = b"test label";
        let expect = include_bytes!("../testdata/prf-result.1.bin");
        let mut output = [0u8; 100];

        super::prf(
            &mut output,
            &*hmac::HMAC_SHA256.with_key(secret),
            label,
            seed,
        );
        assert_eq!(expect.len(), output.len());
        assert_eq!(expect.to_vec(), output.to_vec());
    }

    #[test]
    fn check_sha512() {
        let secret = b"\xb0\x32\x35\x23\xc1\x85\x35\x99\x58\x4d\x88\x56\x8b\xbb\x05\xeb";
        let seed = b"\xd4\x64\x0e\x12\xe4\xbc\xdb\xfb\x43\x7f\x03\xe6\xae\x41\x8e\xe5";
        let label = b"test label";
        let expect = include_bytes!("../testdata/prf-result.2.bin");
        let mut output = [0u8; 196];

        super::prf(
            &mut output,
            &*hmac::HMAC_SHA512.with_key(secret),
            label,
            seed,
        );
        assert_eq!(expect.len(), output.len());
        assert_eq!(expect.to_vec(), output.to_vec());
    }

    #[test]
    fn check_sha384() {
        let secret = b"\xb8\x0b\x73\x3d\x6c\xee\xfc\xdc\x71\x56\x6e\xa4\x8e\x55\x67\xdf";
        let seed = b"\xcd\x66\x5c\xf6\xa8\x44\x7d\xd6\xff\x8b\x27\x55\x5e\xdb\x74\x65";
        let label = b"test label";
        let expect = include_bytes!("../testdata/prf-result.3.bin");
        let mut output = [0u8; 148];

        super::prf(
            &mut output,
            &*hmac::HMAC_SHA384.with_key(secret),
            label,
            seed,
        );
        assert_eq!(expect.len(), output.len());
        assert_eq!(expect.to_vec(), output.to_vec());
    }
}
