#[cfg(all(test, feature = "tokio"))]
mod test {
    use bytes::BytesMut;
    use tokio::io::AsyncWriteExt;

    use super::v5::messages::{AuthMethod, AuthenticationRes, NegotiationRes, ProxyRes, Status};
    use super::{SocksError, read_message};
    use crate::adapters::rt::TokioIo;

    // A SOCKS5 ProxyRes message. Successful, bound to 127.0.0.1:8080.
    const SEG1: [u8; 4] = [0x05, 0x00, 0x00, 0x01];
    const SEG2: [u8; 6] = [0x7F, 0x00, 0x00, 0x01, 0x1F, 0x90];

    // A SOCKS5 NegotiationRes message: username/password method selected.
    const NEG_RES: [u8; 2] = [0x05, 0x02];
    // A SOCKS5 AuthenticationRes message: success.
    const AUTH_RES: [u8; 2] = [0x01, 0x00];

    #[tokio::test]
    async fn it_works_in_one_read() {
        let (client, mut server) = tokio::io::duplex(SEG1.len() + SEG2.len());
        server.write_all(&SEG1).await.unwrap();
        server.write_all(&SEG2).await.unwrap();

        let mut conn = TokioIo::new(client);
        let mut buf = BytesMut::new();

        let m: Result<ProxyRes, SocksError<()>> = read_message(&mut conn, &mut buf).await;
        assert!(m.is_ok());
        assert_eq!(m.unwrap(), ProxyRes(Status::Success));
    }

    #[tokio::test]
    async fn it_works_in_multiple_reads() {
        // Bounded stream ensures message arrives in two reads
        let (client, mut server) = tokio::io::duplex(SEG1.len());
        let _writer = tokio::spawn(async move {
            server.write_all(&SEG1).await.unwrap();
            server.write_all(&SEG2).await.unwrap();
        });

        let mut conn = TokioIo::new(client);
        let mut buf = BytesMut::new();

        let m: Result<ProxyRes, SocksError<()>> = read_message(&mut conn, &mut buf).await;
        assert!(m.is_ok());
        assert_eq!(m.unwrap(), ProxyRes(Status::Success));

        _writer.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn optimistic_sending_works_in_single_read() {
        // Messages will arrive in a single read
        let message = [&NEG_RES[..], &AUTH_RES[..], &SEG1[..], &SEG2[..]].concat();
        let (client, mut server) = tokio::io::duplex(message.len());
        server.write_all(&message).await.unwrap();

        let mut conn = TokioIo::new(client);
        let mut buf = BytesMut::new();

        let m: Result<NegotiationRes, SocksError<()>> = read_message(&mut conn, &mut buf).await;
        assert_eq!(m.unwrap(), NegotiationRes(AuthMethod::UserPass));

        let m: Result<AuthenticationRes, SocksError<()>> = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            read_message(&mut conn, &mut buf),
        )
        .await
        .expect("second message should be parsed from the buffer, not read from the socket");
        assert_eq!(m.unwrap(), AuthenticationRes(true));

        let m: Result<ProxyRes, SocksError<()>> = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            read_message(&mut conn, &mut buf),
        )
        .await
        .expect("third message should be parsed from the buffer, not read from the socket");
        assert_eq!(m.unwrap(), ProxyRes(Status::Success));

        assert!(buf.is_empty(), "all handshake bytes should be consumed");
        drop(server);
    }

    #[tokio::test]
    async fn optimistic_sending_works_in_multiple_reads() {
        // Bounded stream ensures message arrive in multiple reads
        let message = [&NEG_RES[..], &AUTH_RES[..], &SEG1[..], &SEG2[..]].concat();
        let (client, mut server) = tokio::io::duplex(message.len() / 4);
        let _writer = tokio::spawn(async move {
            server.write_all(&message).await.unwrap();
            server
        });

        let mut conn = TokioIo::new(client);
        let mut buf = BytesMut::new();

        let m: Result<NegotiationRes, SocksError<()>> = read_message(&mut conn, &mut buf).await;
        assert_eq!(m.unwrap(), NegotiationRes(AuthMethod::UserPass));

        let m: Result<AuthenticationRes, SocksError<()>> = read_message(&mut conn, &mut buf).await;
        assert_eq!(m.unwrap(), AuthenticationRes(true));

        let m: Result<ProxyRes, SocksError<()>> = read_message(&mut conn, &mut buf).await;
        assert_eq!(m.unwrap(), ProxyRes(Status::Success));

        assert!(buf.is_empty(), "all handshake bytes should be consumed");

        let server = _writer.await.unwrap();
        drop(server);
    }
}
