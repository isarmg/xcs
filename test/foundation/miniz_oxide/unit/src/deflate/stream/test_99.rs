#[cfg(test)]
mod test {
    use super::deflate;
    use crate::deflate::CompressorOxide;
    use crate::inflate::decompress_to_vec_zlib;
    use crate::{MZFlush, MZStatus};
    use alloc::boxed::Box;
    use alloc::vec;

    #[test]
    fn test_state() {
        let data = b"Hello zlib!";
        let mut compressed = vec![0; 50];
        let mut compressor = Box::<CompressorOxide>::default();
        let res = deflate(&mut compressor, data, &mut compressed, MZFlush::Finish);
        let status = res.status.expect("Failed to compress!");
        let result = decompress_to_vec_zlib(&compressed);
        let decomp = result
            .map_err(|err| panic!("Decompress failed with {:}", err))
            .unwrap();

        assert_eq!(status, MZStatus::StreamEnd);
        assert_eq!(decomp[..], data[..]);
        assert_eq!(res.bytes_consumed, data.len());
    }
}
