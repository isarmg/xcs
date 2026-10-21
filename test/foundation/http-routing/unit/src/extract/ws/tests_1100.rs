#[cfg(test)]
mod tests {
    use std::future::ready;

    use super::*;
    use crate::{routing::any, test_helpers::spawn_service, Router};
    use http::{Request, Version};
    use http_body::BodyExt as _;
    use hyper_util::rt::TokioExecutor;
    use tokio::io::{AsyncRead, AsyncWrite};
    use tokio::net::TcpStream;
    use tokio_tungstenite::tungstenite;
    use service_stack::ServiceExt;

    #[crate::test]
    async fn rejects_http_1_0_requests() {
        let svc = any(|ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>| {
            let rejection = ws.unwrap_err();
            assert!(matches!(
                rejection,
                WebSocketUpgradeRejection::ConnectionNotUpgradable(_)
            ));
            std::future::ready(())
        });

        let req = Request::builder()
            .version(Version::HTTP_10)
            .method(Method::GET)
            .header("upgrade", "websocket")
            .header("connection", "Upgrade")
            .header("sec-websocket-key", "6D69KGBOr4Re+Nj6zx9aQA==")
            .header("sec-websocket-version", "13")
            .body(Body::empty())
            .unwrap();

        let res = svc.oneshot(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[allow(dead_code)]
    fn default_on_failed_upgrade() {
        async fn handler(ws: WebSocketUpgrade) -> Response {
            ws.on_upgrade(|_| async {})
        }
        let _: Router = Router::new().route("/", any(handler));
    }

    #[allow(dead_code)]
    fn on_failed_upgrade() {
        async fn handler(ws: WebSocketUpgrade) -> Response {
            ws.on_failed_upgrade(|_error: Error| println!("oops!"))
                .on_upgrade(|_| async {})
        }
        let _: Router = Router::new().route("/", any(handler));
    }

    #[crate::test]
    async fn integration_test() {
        let addr = spawn_service(echo_app());
        let uri = format!("ws://{addr}/echo").try_into().unwrap();
        let req = tungstenite::client::ClientRequestBuilder::new(uri)
            .with_sub_protocol(TEST_ECHO_APP_REQ_SUBPROTO);
        let (socket, response) = tokio_tungstenite::connect_async(req).await.unwrap();
        test_echo_app(socket, response.headers()).await;
    }

    #[crate::test]
    #[cfg(feature = "http2")]
    async fn http2() {
        let addr = spawn_service(echo_app());
        let io = TokioIo::new(TcpStream::connect(addr).await.unwrap());
        let (mut send_request, conn) =
            hyper::client::conn::http2::Builder::new(TokioExecutor::new())
                .handshake(io)
                .await
                .unwrap();

        // Wait a little for the SETTINGS frame to go through…
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        assert!(conn.is_extended_connect_protocol_enabled());
        tokio::spawn(async {
            conn.await.unwrap();
        });

        let req = Request::builder()
            .method(Method::CONNECT)
            .extension(hyper::ext::Protocol::from_static("websocket"))
            .uri("/echo")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-protocol", TEST_ECHO_APP_REQ_SUBPROTO)
            .header("Host", "server.example.com")
            .body(Body::empty())
            .unwrap();

        let mut response = send_request.send_request(req).await.unwrap();
        let status = response.status();
        if status != 200 {
            let body = response.into_body().collect().await.unwrap().to_bytes();
            let body = std::str::from_utf8(&body).unwrap();
            panic!("response status was {status}: {body}");
        }
        let upgraded = hyper::upgrade::on(&mut response).await.unwrap();
        let upgraded = TokioIo::new(upgraded);
        let socket = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Client, None).await;
        test_echo_app(socket, response.headers()).await;
    }

    fn echo_app() -> Router {
        async fn handle_socket(mut socket: WebSocket) {
            assert_eq!(socket.protocol().unwrap(), "echo");
            while let Some(Ok(msg)) = socket.recv().await {
                match msg {
                    Message::Text(_) | Message::Binary(_) | Message::Close(_) => {
                        if socket.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Message::Ping(_) | Message::Pong(_) => {
                        // tungstenite will respond to pings automatically
                    }
                }
            }
        }

        Router::new().route(
            "/echo",
            any(|ws: WebSocketUpgrade| {
                let ws = ws.protocols(["echo2", "echo"]);
                assert_eq!(ws.selected_protocol().unwrap(), "echo");
                ready(ws.on_upgrade(handle_socket))
            }),
        )
    }

    const TEST_ECHO_APP_REQ_SUBPROTO: &str = "echo3, echo";
    async fn test_echo_app<S: AsyncRead + AsyncWrite + Unpin>(
        mut socket: WebSocketStream<S>,
        headers: &http::HeaderMap,
    ) {
        assert_eq!(headers[http::header::SEC_WEBSOCKET_PROTOCOL], "echo");

        let input = tungstenite::Message::Text(tungstenite::Utf8Bytes::from_static("foobar"));
        socket.send(input.clone()).await.unwrap();
        let output = socket.next().await.unwrap().unwrap();
        assert_eq!(input, output);

        socket
            .send(tungstenite::Message::Ping(Bytes::from_static(b"ping")))
            .await
            .unwrap();
        let output = socket.next().await.unwrap().unwrap();
        assert_eq!(
            output,
            tungstenite::Message::Pong(Bytes::from_static(b"ping"))
        );
    }
}
