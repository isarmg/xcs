#[cfg(test)]
mod tests {
    use super::*;

    fn assert_raw_stream<T: RawStream>()
    where
        crate::AutoStream<T>: std::io::Write,
    {
    }

    #[test]
    fn test() {
        assert_raw_stream::<Box<dyn std::io::Write>>();
        assert_raw_stream::<Box<dyn std::io::Write + 'static>>();
        assert_raw_stream::<Box<dyn std::io::Write + Send>>();
        assert_raw_stream::<Box<dyn std::io::Write + Send + Sync>>();

        assert_raw_stream::<&mut dyn std::io::Write>();
        assert_raw_stream::<&mut (dyn std::io::Write + 'static)>();
        assert_raw_stream::<&mut (dyn std::io::Write + Send)>();
        assert_raw_stream::<&mut (dyn std::io::Write + Send + Sync)>();

        assert_raw_stream::<Vec<u8>>();
        assert_raw_stream::<&mut Vec<u8>>();

        assert_raw_stream::<std::fs::File>();
        assert_raw_stream::<&mut std::fs::File>();
    }
}
