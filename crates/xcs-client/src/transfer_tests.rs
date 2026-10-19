use super::{Client, Error};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

async fn request_headers(stream: &mut TcpStream) {
    let mut request = Vec::new();
    while !request.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).await.unwrap(), 1);
        request.push(byte[0]);
        assert!(request.len() < 8192);
    }
    let length = std::str::from_utf8(&request)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>().unwrap())
        .unwrap_or(0);
    assert!(length < 8192);
    stream.read_exact(&mut vec![0; length]).await.unwrap();
}

async fn reply(response: Vec<u8>) -> (Client, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        request_headers(&mut stream).await;
        stream.write_all(&response).await.unwrap();
    });
    (client, task)
}

fn content_headers(content: &[u8]) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"{}\"\r\nConnection: close\r\n\r\n",
        content.len(),
        blake3::hash(content)
    )
    .into_bytes()
}

#[tokio::test]
async fn verified_download_uses_private_tempfile_and_supports_long_destination_names() {
    let content = b"the complete immutable version";
    let mut response = content_headers(content);
    response.extend(content);
    let (client, server) = reply(response).await;
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("a".repeat(240));
    client.download_file("version_test", &target).await.unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), content);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(target).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    server.await.unwrap();
}

#[tokio::test]
async fn checksum_failure_removes_temporary_data_without_publishing_a_destination() {
    let mut response = content_headers(b"good");
    response.extend(b"bad!");
    let (client, server) = reply(response).await;
    let directory = tempfile::tempdir().unwrap();
    let result = client
        .download_file("version_test", &directory.path().join("file.txt"))
        .await;
    assert!(
        matches!(result, Err(Error::Response(message)) if message.contains("CHECKSUM_MISMATCH"))
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    server.await.unwrap();
}

#[tokio::test]
async fn a_concurrent_creator_keeps_its_file_and_download_cleans_up() {
    let content = b"the complete immutable version";
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let (sent, received) = oneshot::channel();
    let (release, wait) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        request_headers(&mut stream).await;
        stream.write_all(&content_headers(content)).await.unwrap();
        stream.write_all(&content[..4]).await.unwrap();
        sent.send(()).unwrap();
        wait.await.unwrap();
        stream.write_all(&content[4..]).await.unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("file.txt");
    let download_target = target.clone();
    let download =
        tokio::spawn(async move { client.download_file("version_test", &download_target).await });
    received.await.unwrap();
    std::fs::write(&target, b"concurrent creator").unwrap();
    release.send(()).unwrap();
    assert!(
        matches!(download.await.unwrap(), Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists)
    );
    assert_eq!(std::fs::read(target).unwrap(), b"concurrent creator");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    server.await.unwrap();
}

#[tokio::test]
async fn empty_success_envelopes_and_non_json_http_errors_keep_their_meaning() {
    let (client, server) =
        reply(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_vec()).await;
    assert!(
        client
            .request("POST", "/v1/auth/logout", None)
            .await
            .unwrap()
            .is_null()
    );
    server.await.unwrap();
    let (client, server) = reply(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 7\r\nConnection: close\r\n\r\nblocked".to_vec()).await;
    assert!(matches!(
        client.request("GET", "/v1/system/status", None).await,
        Err(Error::Api {
            status: 503,
            retryable: true,
            ..
        })
    ));
    server.await.unwrap();
    let (client, server) =
        reply(b"HTTP/1.1 200 OK\r\nContent-Length: 42\r\nConnection: close\r\n\r\n".to_vec()).await;
    assert!(
        client
            .request("HEAD", "/v1/system/status", None)
            .await
            .unwrap()
            .is_null()
    );
    server.await.unwrap();
}

#[tokio::test]
async fn a_malformed_seal_response_is_an_error_instead_of_a_panic_or_cross_workspace_commit() {
    for body in ["[]", "{\"workspace_id\":\"other_workspace\"}", "{}"] {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (client, server) = reply(response.into_bytes()).await;
        assert!(matches!(
            client
                .prepare_commit("workspace_test", "op_test", "save", None)
                .await,
            Err(Error::Response(_))
        ));
        server.await.unwrap();
    }
}
