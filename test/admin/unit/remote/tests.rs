use super::*;
#[test]
fn login_output_preserves_u64_precision_and_excludes_all_credentials() {
    let session = storage_client::Session {
        token: "fixture-session-secret".into(),
        csrf_token: "fixture-csrf-secret".into(),
        principal: "admin".into(),
        admin: true,
        expires_at: u64::MAX,
    };
    let output = login_summary(&session, std::path::Path::new("token"));
    assert_eq!(output["expires_at"], u64::MAX.to_string());
    assert!(output.get("token").is_none());
    assert!(output.get("csrf_token").is_none());
    assert!(!output.to_string().contains("fixture-"));
}
#[tokio::test]
async fn failed_token_publication_revokes_only_the_new_fixture_session() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let token = authentication::random_token().unwrap();
    let csrf = authentication::derive_csrf_token(&token).unwrap();
    let server = tokio::spawn(async move {
        let mut paths = Vec::new();
        for payload in [
            json!({"token":token,"csrf_token":csrf,"principal":"admin","admin":true,"expires_at":u64::MAX.to_string()}),
            json!({"logged_out":true}),
        ] {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let header_end = loop {
                let mut buffer = [0_u8; 1024];
                let read = stream.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&buffer[..read]);
                assert!(bytes.len() <= 8192);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
            paths.push(
                headers
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned(),
            );
            let content_length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            while bytes.len() < header_end + content_length {
                let mut buffer = [0_u8; 1024];
                let read = stream.read(&mut buffer).await.unwrap();
                assert!(read > 0);
                bytes.extend_from_slice(&buffer[..read]);
            }
            let payload = json_codec::to_vec(&payload).unwrap();
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                payload.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(&payload).await.unwrap();
        }
        paths
    });
    let root = tempfile::tempdir().unwrap();
    let password = root.path().join("password");
    let output = root.path().join("already-exists");
    crate::private_io::write_new(&password, b"fixture-password").unwrap();
    crate::private_io::write_new(&output, b"preserved").unwrap();
    let args = Args {
        root: root.path().into(),
        principal: "admin".into(),
        server: Some(address.clone()),
        token_file: None,
        command: Command::Login {
            username: "admin".into(),
            password_file: Some(password),
            output: output.clone(),
        },
    };
    assert!(run(&args, &address).await.is_err());
    let paths = tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(paths, ["/api/auth/login", "/api/auth/logout"]);
    assert_eq!(std::fs::read(output).unwrap(), b"preserved");
}
