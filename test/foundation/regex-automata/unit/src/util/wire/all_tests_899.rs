#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        let mut buf = [0; 1024];

        let nwrite = write_label("fooba", &mut buf).unwrap();
        assert_eq!(nwrite, 8);
        assert_eq!(&buf[..nwrite], b"fooba\x00\x00\x00");

        let nread = read_label(&buf, "fooba").unwrap();
        assert_eq!(nread, 8);
    }

    #[test]
    #[should_panic]
    fn bad_label_interior_nul() {
        // interior NULs are not allowed
        write_label("foo\x00bar", &mut [0; 1024]).unwrap();
    }

    #[test]
    fn bad_label_almost_too_long() {
        // ok
        write_label(&"z".repeat(255), &mut [0; 1024]).unwrap();
    }

    #[test]
    #[should_panic]
    fn bad_label_too_long() {
        // labels longer than 255 bytes are banned
        write_label(&"z".repeat(256), &mut [0; 1024]).unwrap();
    }

    #[test]
    fn padding() {
        assert_eq!(0, padding_len(8));
        assert_eq!(3, padding_len(9));
        assert_eq!(2, padding_len(10));
        assert_eq!(1, padding_len(11));
        assert_eq!(0, padding_len(12));
        assert_eq!(3, padding_len(13));
        assert_eq!(2, padding_len(14));
        assert_eq!(1, padding_len(15));
        assert_eq!(0, padding_len(16));
    }
}
