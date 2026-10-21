#[cfg(all(test, feature = "alloc"))]
mod test {
    use super::{inflate, InflateState};
    use crate::codec::{DataFormat, MZFlush, MZStatus};
    use alloc::vec;

    #[test]
    fn test_state() {
        let encoded = [
            120u8, 156, 243, 72, 205, 201, 201, 215, 81, 168, 202, 201, 76, 82, 4, 0, 27, 101, 4,
            19,
        ];
        let mut out = vec![0; 50];
        let mut state = InflateState::new_boxed(DataFormat::Zlib);
        let res = inflate(&mut state, &encoded, &mut out, MZFlush::Finish);
        let status = res.status.expect("Failed to decompress!");
        assert_eq!(status, MZStatus::StreamEnd);
        assert_eq!(out[..res.bytes_written as usize], b"Hello, zlib!"[..]);
        assert_eq!(res.bytes_consumed, encoded.len());

        state.reset_as(super::ZeroReset);
        out.iter_mut().map(|x| *x = 0).count();
        let res = inflate(&mut state, &encoded, &mut out, MZFlush::Finish);
        let status = res.status.expect("Failed to decompress!");
        assert_eq!(status, MZStatus::StreamEnd);
        assert_eq!(out[..res.bytes_written as usize], b"Hello, zlib!"[..]);
        assert_eq!(res.bytes_consumed, encoded.len());

        state.reset_as(super::MinReset);
        out.iter_mut().map(|x| *x = 0).count();
        let res = inflate(&mut state, &encoded, &mut out, MZFlush::Finish);
        let status = res
            .status
            .map_err(|err| panic!("Decompress failed with {:?}", err))
            .unwrap();
        assert_eq!(status, MZStatus::StreamEnd);
        assert_eq!(out[..res.bytes_written as usize], b"Hello, zlib!"[..]);
        assert_eq!(res.bytes_consumed, encoded.len());
        assert_eq!(state.decompressor().adler32(), Some(459605011));

        // Test state when not computing adler.
        state = InflateState::new_boxed(DataFormat::ZLibIgnoreChecksum);
        out.iter_mut().map(|x| *x = 0).count();
        let res = inflate(&mut state, &encoded, &mut out, MZFlush::Finish);
        let status = res.status.expect("Failed to decompress!");
        assert_eq!(status, MZStatus::StreamEnd);
        assert_eq!(out[..res.bytes_written as usize], b"Hello, zlib!"[..]);
        assert_eq!(res.bytes_consumed, encoded.len());
        // Not computed, so should be Some(1)
        assert_eq!(state.decompressor().adler32(), Some(1));
        // Should still have the checksum read from the header file.
        assert_eq!(state.decompressor().adler32_header(), Some(459605011))
    }

    #[test]
    fn test_partial_continue() {
        let encoded = [
            120u8, 156, 243, 72, 205, 201, 201, 215, 81, 168, 202, 201, 76, 82, 4, 0, 27, 101, 4,
            19,
        ];

        // Feed input bytes one at a time to the decompressor
        let mut out = vec![0; 50];
        let mut state = InflateState::new_boxed(DataFormat::Zlib);
        let mut part_in = 0;
        let mut part_out = 0;
        for i in 1..=encoded.len() {
            let res = inflate(
                &mut state,
                &encoded[part_in..i],
                &mut out[part_out..],
                MZFlush::None,
            );
            let status = res.status.expect("Failed to decompress!");
            if i == encoded.len() {
                assert_eq!(status, MZStatus::StreamEnd);
            } else {
                assert_eq!(status, MZStatus::Ok);
            }
            part_out += res.bytes_written as usize;
            part_in += res.bytes_consumed;
        }

        assert_eq!(out[..part_out as usize], b"Hello, zlib!"[..]);
        assert_eq!(part_in, encoded.len());
        assert_eq!(state.decompressor().adler32(), Some(459605011));
    }

    // Inflate part of a stream and clone the inflate state.
    // Discard the original state and resume the stream from the clone.
    #[test]
    fn test_rewind_and_resume() {
        let encoded = [
            120u8, 156, 243, 72, 205, 201, 201, 215, 81, 168, 202, 201, 76, 82, 4, 0, 27, 101, 4,
            19,
        ];
        let decoded = b"Hello, zlib!";

        // Feed partial input bytes to the decompressor
        let mut out = vec![0; 50];
        let mut state = InflateState::new_boxed(DataFormat::Zlib);
        let res1 = inflate(&mut state, &encoded[..10], &mut out, MZFlush::None);
        let status = res1.status.expect("Failed to decompress!");
        assert_eq!(status, MZStatus::Ok);

        // Clone the state and discard the original
        let mut resume = state.clone();
        drop(state);

        // Resume the stream using the cloned state
        let res2 = inflate(
            &mut resume,
            &encoded[res1.bytes_consumed..],
            &mut out[res1.bytes_written..],
            MZFlush::Finish,
        );
        let status = res2.status.expect("Failed to decompress!");
        assert_eq!(status, MZStatus::StreamEnd);

        assert_eq!(res1.bytes_consumed + res2.bytes_consumed, encoded.len());
        assert_eq!(res1.bytes_written + res2.bytes_written, decoded.len());
        assert_eq!(
            &out[..res1.bytes_written + res2.bytes_written as usize],
            decoded
        );
        assert_eq!(resume.decompressor().adler32(), Some(459605011));
    }
}
