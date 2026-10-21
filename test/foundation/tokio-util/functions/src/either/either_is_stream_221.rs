#[tokio::test]
    async fn either_is_stream() {
        let mut either: Either<Once<u32>, Once<u32>> = Either::Left(once(1));

        assert_eq!(Some(1u32), either.next().await);
    }
