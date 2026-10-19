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

#[tokio::test]
async fn truncated_download_removes_partial_file() {
    let mut response = content_headers(b"a complete file");
    response.extend(b"a partial");
    let (client, server) = reply(response).await;
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        client
            .download_file("version_test", &directory.path().join("file"))
            .await,
        Err(Error::Transport(_))
    ));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    server.await.unwrap();
}

#[tokio::test]
async fn cancelling_an_in_progress_download_removes_its_private_tempfile() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let (release, wait) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        request_headers(&mut stream).await;
        stream
            .write_all(&content_headers(b"unfinished download"))
            .await
            .unwrap();
        stream.write_all(b"unfin").await.unwrap();
        let _ = wait.await;
    });
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("file");
    let download = tokio::spawn(async move { client.download_file("version_test", &target).await });
    // Wait for bytes on disk, rather than guessing when the transfer started.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if std::fs::read_dir(directory.path())
                .unwrap()
                .any(|entry| entry.unwrap().metadata().unwrap().len() > 0)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("download never wrote its partial file");
    download.abort();
    assert!(download.await.unwrap_err().is_cancelled());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    drop(release);
    server.await.unwrap();
}

#[tokio::test]
async fn existing_destination_is_preserved_before_any_network_request() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("file");
    std::fs::write(&target, b"keep me").unwrap();
    assert!(
        matches!(client.download_file("version_test", &target).await,
        Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists)
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"keep me");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn missing_or_weak_integrity_headers_never_publish_downloads() {
    for headers in [
        "Content-Length: 4\r\n".to_owned(),
        format!(
            "Content-Length: 4\r\nETag: W/\"{}\"\r\n",
            blake3::hash(b"file")
        ),
        "Content-Length: 4\r\nETag: \"not-a-hash\"\r\n".to_owned(),
        format!("ETag: \"{}\"\r\n", blake3::hash(b"file")),
    ] {
        let (client, server) = reply(
            format!("HTTP/1.1 200 OK\r\n{headers}Connection: close\r\n\r\nfile").into_bytes(),
        )
        .await;
        let directory = tempfile::tempdir().unwrap();
        assert!(matches!(
            client
                .download_file("version_test", &directory.path().join("file"))
                .await,
            Err(Error::Response(_))
        ));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        server.await.unwrap();
    }
}

#[tokio::test]
async fn upload_sends_raw_bytes_with_matching_length_hash_and_bearer() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client =
        Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    client.set_token(Some("synthetic-token".into()));
    let content = b"\0binary\xff\r\nbytes";
    let source = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(source.path(), content).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
            assert!(headers.len() < 8192);
        }
        let headers = String::from_utf8(headers).unwrap().to_lowercase();
        assert!(headers.starts_with("put /v1/workspaces/workspace_test/files/item_test/content?path=folder%2fa+b.bin http/1.1\r\n"));
        for header in [
            "authorization: bearer synthetic-token".to_owned(),
            "content-type: application/octet-stream".to_owned(),
            format!("content-length: {}", content.len()),
            format!("x-xcs-logical-size: {}", content.len()),
            format!("x-xcs-content-hash: {}", blake3::hash(content)),
            "x-xcs-upload-generation: 1".to_owned(),
        ] {
            assert!(
                headers.contains(&format!("{header}\r\n")),
                "missing {header}"
            );
        }
        let mut body = vec![0; content.len()];
        stream.read_exact(&mut body).await.unwrap();
        assert_eq!(body, content);
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"staged\":true}").await.unwrap();
    });
    // The envelope is checked as well as the request wire format.
    assert_eq!(
        client
            .upload_file(
                "workspace_test",
                "item_test",
                "folder/a b.bin",
                source.path()
            )
            .await
            .unwrap(),
        serde_json::json!({"staged":true})
    );
    server.await.unwrap();
}

#[tokio::test]
async fn upload_integrity_rejections_preserve_server_error_details() {
    for code in ["SIZE_MISMATCH", "CHECKSUM_MISMATCH"] {
        let body = serde_json::json!({"error":{"code":code,"message":"upload rejected","retryable":false,"request_id":"req_synthetic"}}).to_string();
        let (client, server) = reply(format!("HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()).await;
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"source is preserved").unwrap();
        assert!(
            matches!(client.upload_file("workspace_test", "item_test", "file", source.path()).await,
            Err(Error::Api {status:422, code:actual, message, retryable:false})
            if actual == code && message == "upload rejected (request req_synthetic)")
        );
        assert_eq!(
            std::fs::read(source.path()).unwrap(),
            b"source is preserved"
        );
        server.await.unwrap();
    }
}

#[tokio::test]
async fn interrupted_upload_is_a_transport_error_and_preserves_source() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = Client::connect(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let source = tempfile::NamedTempFile::new().unwrap();
    let content = vec![0x5a; 1024 * 1024];
    std::fs::write(source.path(), &content).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(stream.read_u8().await.unwrap());
            assert!(headers.len() < 8192);
        }
        let mut prefix = [0; 16];
        stream.read_exact(&mut prefix).await.unwrap();
        assert_eq!(prefix, [0x5a; 16]);
        // Close before consuming the body or producing any response.
    });
    assert!(matches!(
        client
            .upload_file("workspace_test", "item_test", "file", source.path())
            .await,
        Err(Error::Transport(_))
    ));
    assert_eq!(std::fs::read(source.path()).unwrap(), content);
    server.await.unwrap();
}
