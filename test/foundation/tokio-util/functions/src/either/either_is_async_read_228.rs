#[tokio::test]
    async fn either_is_async_read() {
        let mut buffer = [0; 3];
        let mut either: Either<Repeat, Repeat> = Either::Right(repeat(0b101));

        either.read_exact(&mut buffer).await.unwrap();
        assert_eq!(buffer, [0b101, 0b101, 0b101]);
    }
