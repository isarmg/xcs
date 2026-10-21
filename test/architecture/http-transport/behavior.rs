use http_transport::{Request, Response, StatusCode, body::{Bytes, Incoming}, service::service_fn};
use http_transport::adapters::rt::TokioIo;
use http_body::{BodyExt, Empty, Full};
use std::{convert::Infallible, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn actual_http1_client_server_roundtrip() {
    let (client, server) = tokio::io::duplex(4096);
    let server = tokio::spawn(async move {
        http_transport::server::conn::http1::Builder::new().serve_connection(TokioIo::new(server), service_fn(|request: Request<Incoming>| async move {
            assert_eq!(request.method(), "POST");
            assert_eq!(request.uri(), "/upload?q=1");
            assert_eq!(request.headers()["x-check"], "present");
            let body = request.into_body().collect().await.unwrap().to_bytes();
            Ok::<_, Infallible>(Response::builder().status(StatusCode::ACCEPTED).body(Full::new(body)).unwrap())
        })).await.unwrap();
    });
    let (mut sender, connection) = http_transport::client::conn::http1::handshake(TokioIo::new(client)).await.unwrap();
    let client = tokio::spawn(connection);
    let response = sender.send_request(Request::builder().method("POST").uri("/upload?q=1").header("host", "local.test").header("x-check", "present").body(Full::new(Bytes::from_static(b"payload"))).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.into_body().collect().await.unwrap().to_bytes(), "payload");
    drop(sender);
    tokio::time::timeout(Duration::from_secs(5), client).await.unwrap().unwrap().unwrap();
    tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap();
}

#[tokio::test]
async fn actual_http1_parser_rejects_malformed_wire_request() {
    let (mut client, server) = tokio::io::duplex(4096);
    let connection = tokio::spawn(async move {
        http_transport::server::conn::http1::Builder::new().serve_connection(TokioIo::new(server), service_fn(|_: Request<Incoming>| async { Ok::<_, Infallible>(Response::new(Empty::<Bytes>::new())) })).await
    });
    client.write_all(b"GET / HTTP/1.1\r\ninvalid header\r\n\r\n").await.unwrap();
    client.shutdown().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), connection).await.unwrap().unwrap().unwrap_err();
    assert!(error.is_parse());
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await.unwrap();
    assert!(bytes.starts_with(b"HTTP/1.1 400"));
}

#[tokio::test]
async fn service_failure_is_propagated_without_success_response() {
    let (client, server) = tokio::io::duplex(4096);
    let server = tokio::spawn(async move {
        http_transport::server::conn::http1::Builder::new().serve_connection(TokioIo::new(server), service_fn(|_: Request<Incoming>| async {
            Err::<Response<Empty<Bytes>>, _>(std::io::Error::other("service failed"))
        })).await
    });
    let (mut sender, connection) = http_transport::client::conn::http1::handshake(TokioIo::new(client)).await.unwrap();
    let client = tokio::spawn(connection);
    assert!(sender.send_request(Request::builder().header("host", "local.test").body(Empty::<Bytes>::new()).unwrap()).await.is_err());
    assert!(tokio::time::timeout(Duration::from_secs(5), server).await.unwrap().unwrap().is_err());
    drop(sender);
    let _ = tokio::time::timeout(Duration::from_secs(5), client).await.unwrap();
}

#[tokio::test]
async fn service_adapter_preserves_shared_request_response_types() {
    use http_transport::service::Service;
    let inner = service_stack::service_fn(|request: Request<Empty<Bytes>>| async move { Ok::<_, Infallible>(Response::new(request.into_body())) });
    let adapted = http_transport::adapters::service::TowerToHyperService::new(inner);
    assert_eq!(adapted.call(Request::new(Empty::<Bytes>::new())).await.unwrap().status(), StatusCode::OK);
}
