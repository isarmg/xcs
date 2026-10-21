#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn test_compilation() {
        use crate::io::read;
        use core::mem::MaybeUninit;

        // We need to obtain input stream, so open our own source file.
        let input = std::fs::File::open("src/buffer.rs").unwrap();

        let mut buf = vec![0_u8; 3];
        buf.reserve(32);
        let _x: usize = read(&input, spare_capacity(&mut buf)).unwrap();
        let _x: (&mut [u8], &mut [MaybeUninit<u8>]) =
            read(&input, buf.spare_capacity_mut()).unwrap();
        let _x: usize = read(&input, &mut buf).unwrap();
        let _x: usize = read(&input, &mut *buf).unwrap();
        let _x: usize = read(&input, &mut buf[..]).unwrap();
        let _x: usize = read(&input, &mut (*buf)[..]).unwrap();

        let mut buf = [0, 0, 0];
        let _x: usize = read(&input, &mut buf).unwrap();
        let _x: usize = read(&input, &mut buf[..]).unwrap();

        let mut buf = [
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
        ];
        let _x: (&mut [u8], &mut [MaybeUninit<u8>]) = read(&input, &mut buf).unwrap();
        let _x: (&mut [u8], &mut [MaybeUninit<u8>]) = read(&input, &mut buf[..]).unwrap();

        let mut buf = vec![
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
        ];
        let _x: (&mut [u8], &mut [MaybeUninit<u8>]) = read(&input, &mut buf).unwrap();
        let _x: (&mut [u8], &mut [MaybeUninit<u8>]) = read(&input, &mut buf[..]).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn test_slice() {
        use crate::io::read;
        use std::io::{Seek, SeekFrom};

        // We need to obtain input stream with contents that we can compare
        // against, so open our own source file.
        let mut input = std::fs::File::open("src/buffer.rs").unwrap();

        let mut buf = [0_u8; 64];
        let nread = read(&input, &mut buf).unwrap();
        assert_eq!(nread, buf.len());
        assert_eq!(
            &buf[..58],
            b"//! Utilities for functions that return data via buffers.\n"
        );
        input.seek(SeekFrom::End(-1)).unwrap();
        let nread = read(&input, &mut buf).unwrap();
        assert_eq!(nread, 1);
        input.seek(SeekFrom::End(0)).unwrap();
        let nread = read(&input, &mut buf).unwrap();
        assert_eq!(nread, 0);
    }

    #[cfg(not(windows))]
    #[test]
    fn test_slice_uninit() {
        use crate::io::read;
        use core::mem::MaybeUninit;
        use std::io::{Seek, SeekFrom};

        // We need to obtain input stream with contents that we can compare
        // against, so open our own source file.
        let mut input = std::fs::File::open("src/buffer.rs").unwrap();

        let mut buf = [MaybeUninit::<u8>::uninit(); 64];
        let (init, uninit) = read(&input, &mut buf).unwrap();
        assert_eq!(uninit.len(), 0);
        assert_eq!(
            &init[..58],
            b"//! Utilities for functions that return data via buffers.\n"
        );
        assert_eq!(init.len(), buf.len());
        assert_eq!(
            unsafe { core::mem::transmute::<&mut [MaybeUninit<u8>], &mut [u8]>(&mut buf[..58]) },
            b"//! Utilities for functions that return data via buffers.\n"
        );
        input.seek(SeekFrom::End(-1)).unwrap();
        let (init, uninit) = read(&input, &mut buf).unwrap();
        assert_eq!(init.len(), 1);
        assert_eq!(uninit.len(), buf.len() - 1);
        input.seek(SeekFrom::End(0)).unwrap();
        let (init, uninit) = read(&input, &mut buf).unwrap();
        assert_eq!(init.len(), 0);
        assert_eq!(uninit.len(), buf.len());
    }

    #[cfg(not(windows))]
    #[test]
    fn test_spare_capacity() {
        use crate::io::read;
        use std::io::{Seek, SeekFrom};

        // We need to obtain input stream with contents that we can compare
        // against, so open our own source file.
        let mut input = std::fs::File::open("src/buffer.rs").unwrap();

        let mut buf = Vec::with_capacity(64);
        let nread = read(&input, spare_capacity(&mut buf)).unwrap();
        assert_eq!(nread, buf.capacity());
        assert_eq!(nread, buf.len());
        assert_eq!(
            &buf[..58],
            b"//! Utilities for functions that return data via buffers.\n"
        );
        buf.clear();
        input.seek(SeekFrom::End(-1)).unwrap();
        let nread = read(&input, spare_capacity(&mut buf)).unwrap();
        assert_eq!(nread, 1);
        assert_eq!(buf.len(), 1);
        buf.clear();
        input.seek(SeekFrom::End(0)).unwrap();
        let nread = read(&input, spare_capacity(&mut buf)).unwrap();
        assert_eq!(nread, 0);
        assert!(buf.is_empty());
    }
}
