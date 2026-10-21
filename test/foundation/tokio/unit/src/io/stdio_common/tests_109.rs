#[cfg(test)]

mod tests {
    use crate::io::blocking::DEFAULT_MAX_BUF_SIZE;
    use crate::io::AsyncWriteExt;
    use std::io;
    use std::pin::Pin;
    use std::task::Context;
    use std::task::Poll;

    struct TextMockWriter;

    impl crate::io::AsyncWrite for TextMockWriter {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, io::Error>> {
            assert!(buf.len() <= DEFAULT_MAX_BUF_SIZE);
            assert!(std::str::from_utf8(buf).is_ok());
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    struct LoggingMockWriter {
        write_history: Vec<usize>,
    }

    impl LoggingMockWriter {
        fn new() -> Self {
            LoggingMockWriter {
                write_history: Vec::new(),
            }
        }
    }

    impl crate::io::AsyncWrite for LoggingMockWriter {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, io::Error>> {
            assert!(buf.len() <= DEFAULT_MAX_BUF_SIZE);
            self.write_history.push(buf.len());
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)] // takes a really long time with miri
    fn test_splitter() {
        let data = str::repeat("█", DEFAULT_MAX_BUF_SIZE);
        let mut wr = super::SplitByUtf8BoundaryIfWindows::new(TextMockWriter);
        let fut = async move {
            wr.write_all(data.as_bytes()).await.unwrap();
        };
        crate::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(fut);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // takes a really long time with miri
    fn test_pseudo_text() {
        // In this test we write a piece of binary data, whose beginning is
        // text though. We then validate that even in this corner case buffer
        // was not shrunk too much.
        let checked_count = super::MAGIC_CONST * super::MAX_BYTES_PER_CHAR;
        let mut data: Vec<u8> = str::repeat("a", checked_count).into();
        data.extend(std::iter::repeat(0b1010_1010).take(DEFAULT_MAX_BUF_SIZE - checked_count + 1));
        let mut writer = LoggingMockWriter::new();
        let mut splitter = super::SplitByUtf8BoundaryIfWindows::new(&mut writer);
        crate::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                splitter.write_all(&data).await.unwrap();
            });
        // Check that at most two writes were performed
        assert!(writer.write_history.len() <= 2);
        // Check that all has been written
        assert_eq!(
            writer.write_history.iter().copied().sum::<usize>(),
            data.len()
        );
        // Check that at most MAX_BYTES_PER_CHAR + 1 (i.e. 5) bytes were shrunk
        // from the buffer: one because it was outside of DEFAULT_MAX_BUF_SIZE boundary, and
        // up to one "utf8 code point".
        assert!(data.len() - writer.write_history[0] <= super::MAX_BYTES_PER_CHAR + 1);
    }
}
