#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyExt, Empty, Full};

    #[tokio::test]
    async fn data_left() {
        let full = Full::new(&b"hello"[..]);

        let mut value: Either<_, Empty<&[u8]>> = Either::Left(full);

        assert_eq!(value.size_hint().exact(), Some(b"hello".len() as u64));
        assert_eq!(
            value.frame().await.unwrap().unwrap().into_data().unwrap(),
            &b"hello"[..]
        );
        assert!(value.frame().await.is_none());
    }

    #[tokio::test]
    async fn data_right() {
        let full = Full::new(&b"hello!"[..]);

        let mut value: Either<Empty<&[u8]>, _> = Either::Right(full);

        assert_eq!(value.size_hint().exact(), Some(b"hello!".len() as u64));
        assert_eq!(
            value.frame().await.unwrap().unwrap().into_data().unwrap(),
            &b"hello!"[..]
        );
        assert!(value.frame().await.is_none());
    }

    #[test]
    fn into_inner() {
        let a = Either::<i32, i32>::Left(2);
        assert_eq!(a.into_inner(), 2)
    }
}
