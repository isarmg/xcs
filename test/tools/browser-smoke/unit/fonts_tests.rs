use super::*;

#[test]
fn actual_compact_and_full_size_paint_geometry_are_distinct() {
    let compact = json!({"fonts":true,"stable":true,"state":"paint","width":380,"height":253.333,
        "extent":48,"imageWidth":101,"imageHeight":101,"slice":"49.504950495% fill",
        "borderWidth":50,"outset":2,"cornerAlpha":0,"curveCornerAlpha":0,"centerAlpha":255});
    assert!(validate_paint(&compact).is_ok());
    let mut stale = compact.clone();
    stale["extent"] = json!(40);
    assert!(validate_paint(&stale).is_err());
    let mut full = compact.clone();
    full["imageWidth"] = json!(384);
    full["imageHeight"] = json!(257);
    full["slice"] = json!("2 fill");
    full["borderWidth"] = json!(2);
    assert!(validate_paint(&full).is_ok());
    full["height"] = json!(300);
    assert!(validate_paint(&full).is_err());
    let mut undrawn = compact.clone();
    undrawn["centerAlpha"] = json!(0);
    assert!(validate_paint(&undrawn).is_err());
    let mut squared = compact.clone();
    squared["curveCornerAlpha"] = json!(255);
    assert!(validate_paint(&squared).is_err());
    let mut replaced = compact;
    replaced["stable"] = json!(false);
    assert!(validate_paint(&replaced).is_err());
}

#[test]
fn font_proxy_delays_original_bytes_and_headers() {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", server.local_addr().unwrap());
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: font/woff2\r\nContent-Length: 4\r\nConnection: close\r\n\r\n\0\xffab";
    let upstream = thread::spawn(move || {
        let (mut stream, _) = server.accept().unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        stream.write_all(response).unwrap();
    });
    let proxy = FontProxy::start(&origin, false).unwrap();
    let mut client = TcpStream::connect(proxy.origin.trim_start_matches("http://")).unwrap();
    client
        .write_all(b"GET /fonts/interface-regular.woff2 HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    client
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut byte = [0];
    assert!(client.read(&mut byte).is_err());
    assert_eq!(proxy.requested.load(Ordering::SeqCst), 1);
    proxy.release();
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut received = Vec::new();
    client.read_to_end(&mut received).unwrap();
    assert_eq!(received, response);
    upstream.join().unwrap();
}

#[test]
fn missing_font_does_not_need_upstream_response() {
    let proxy = FontProxy::start("http://127.0.0.1:1", true).unwrap();
    let port = proxy.origin.rsplit(':').next().unwrap().parse().unwrap();
    let (status, body) = http(port, "GET", "/fonts/interface-bold.woff2", None).unwrap();
    assert_eq!(status, 404);
    assert!(body.is_empty());
    assert_eq!(proxy.requested.load(Ordering::SeqCst), 1);
}
