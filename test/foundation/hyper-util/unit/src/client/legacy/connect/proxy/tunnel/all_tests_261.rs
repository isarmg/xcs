#[cfg(all(test, feature = "tokio"))]
mod tests {
    use std::time::Duration;

    use tokio::io::AsyncWriteExt;

    use super::{Headers, TunnelError, tunnel};
    use crate::rt::TokioIo;

    async fn handshake(response: &'static [u8]) -> Result<(), TunnelError> {
        let (client, mut server) = tokio::io::duplex(1024);
        tokio::spawn(async move {
            server.write_all(response).await.unwrap();
        });

        tokio::time::timeout(
            Duration::from_secs(1),
            tunnel(TokioIo::new(client), "example.com", 443, &Headers::Empty),
        )
        .await
        .expect("handshake should not hang")
        .map(drop)
    }

    #[tokio::test]
    async fn established() {
        handshake(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .await
            .expect("200 response should establish the tunnel");
    }

    #[tokio::test]
    async fn established_with_early_data() {
        handshake(b"HTTP/1.1 200 OK\r\n\r\nHELLO")
            .await
            .expect("early data must not prevent establishing the tunnel");
    }

    #[tokio::test]
    async fn proxy_auth_required() {
        let err = handshake(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n")
            .await
            .unwrap_err();
        assert!(matches!(err, TunnelError::ProxyAuthRequired));
    }

    #[tokio::test]
    async fn non_200_is_unsuccessful() {
        let err = handshake(b"HTTP/1.1 500 Internal Server Error\r\n\r\n")
            .await
            .unwrap_err();
        assert!(matches!(err, TunnelError::TunnelUnsuccessful));
    }

    #[tokio::test]
    async fn malformed_status_is_rejected() {
        let err = handshake(b"HTTP/1.1 2000 OK\r\n\r\n").await.unwrap_err();
        assert!(matches!(err, TunnelError::TunnelUnsuccessful));
    }
}
