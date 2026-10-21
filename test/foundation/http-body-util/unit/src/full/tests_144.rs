#[cfg(test)]
mod tests {
    use super::*;
    use crate::BodyExt;

    #[tokio::test]
    async fn full_returns_some() {
        let mut full = Full::new(&b"hello"[..]);
        assert_eq!(full.size_hint().exact(), Some(b"hello".len() as u64));
        assert_eq!(
            full.frame().await.unwrap().unwrap().into_data().unwrap(),
            &b"hello"[..]
        );
        assert!(full.frame().await.is_none());
    }

    #[tokio::test]
    async fn empty_full_returns_none() {
        assert!(Full::<&[u8]>::default().frame().await.is_none());
        assert!(Full::new(&b""[..]).frame().await.is_none());
    }

    #[tokio::test]
    async fn full_into_inner_returns_none_before_poll() {
        const DATA: &[u8] = b"hello";
        let full = Full::new(DATA);
        assert_eq!(
            full.into_inner(),
            Some(str::as_bytes("hello")),
            "`Full::into_inner()` returns `Some(_)` before poll"
        );
    }

    #[tokio::test]
    async fn full_into_inner_returns_none_after_poll() {
        const DATA: &[u8] = b"hello";
        let mut full = Full::new(DATA);
        full.frame().await.expect("a result").expect("a frame");
        assert_eq!(
            full.into_inner(),
            None,
            "`Full::into_inner()` returns `None` after poll"
        );
    }
}
