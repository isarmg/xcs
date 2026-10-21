#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeroes() {
        assert_eq!(adler32_slice(&[]), 1);
        assert_eq!(adler32_slice(&[0]), 1 | 1 << 16);
        assert_eq!(adler32_slice(&[0, 0]), 1 | 2 << 16);
        assert_eq!(adler32_slice(&[0; 100]), 0x00640001);
        assert_eq!(adler32_slice(&[0; 1024]), 0x04000001);
        assert_eq!(adler32_slice(&[0; 1024 * 1024]), 0x00f00001);
    }

    #[test]
    fn ones() {
        assert_eq!(adler32_slice(&[0xff; 1024]), 0x79a6fc2e);
        assert_eq!(adler32_slice(&[0xff; 1024 * 1024]), 0x8e88ef11);
    }

    #[test]
    fn mixed() {
        assert_eq!(adler32_slice(&[1]), 2 | 2 << 16);
        assert_eq!(adler32_slice(&[40]), 41 | 41 << 16);

        assert_eq!(adler32_slice(&[0xA5; 1024 * 1024]), 0xd5009ab1);
    }

    /// Example calculation from https://en.wikipedia.org/wiki/Adler-32.
    #[test]
    fn wiki() {
        assert_eq!(adler32_slice(b"Wikipedia"), 0x11E60398);
    }

    #[test]
    fn resume() {
        let mut adler = Adler32::new();
        adler.write_slice(&[0xff; 1024]);
        let partial = adler.checksum();
        assert_eq!(partial, 0x79a6fc2e); // from above
        adler.write_slice(&[0xff; 1024 * 1024 - 1024]);
        assert_eq!(adler.checksum(), 0x8e88ef11); // from above

        // Make sure that we can resume computing from the partial checksum via `from_checksum`.
        let mut adler = Adler32::from_checksum(partial);
        adler.write_slice(&[0xff; 1024 * 1024 - 1024]);
        assert_eq!(adler.checksum(), 0x8e88ef11); // from above
    }

    #[cfg(feature = "std")]
    #[test]
    fn bufread() {
        use std::io::BufReader;
        fn test(data: &[u8], checksum: u32) {
            // `BufReader` uses an 8 KB buffer, so this will test buffer refilling.
            let mut buf = BufReader::new(data);
            let real_sum = adler32(&mut buf).unwrap();
            assert_eq!(checksum, real_sum);
        }

        test(&[], 1);
        test(&[0; 1024], 0x04000001);
        test(&[0; 1024 * 1024], 0x00f00001);
        test(&[0xA5; 1024 * 1024], 0xd5009ab1);
    }
}
