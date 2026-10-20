//! Read-only loopback preview for this demo, independent of the storage server.
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    time::Duration,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn asset(root: &Path, target: &str) -> Option<PathBuf> {
    let target = target.split('?').next()?;
    if !target.starts_with('/') || target.contains(['%', '\\', '\0']) {
        return None;
    }
    let relative = Path::new(target.trim_start_matches('/'));
    if relative
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(relative);
    let path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };
    let canonical = path.canonicalize().ok()?;
    (canonical.starts_with(root) && canonical.is_file()).then_some(canonical)
}
fn reply(
    stream: &mut TcpStream,
    status: &str,
    mime: &str,
    bytes: &[u8],
    head: bool,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; font-src 'self'; img-src 'self' data: blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    if !head {
        stream.write_all(bytes)?;
    }
    Ok(())
}
fn connection(mut stream: TcpStream, root: &Path) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0; 2048];
    while bytes.len() < 8192 && !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
    let request = std::str::from_utf8(&bytes)?;
    let mut line = request.lines().next().unwrap_or("").split_whitespace();
    let method = line.next().unwrap_or("");
    let target = line.next().unwrap_or("");
    let head = method == "HEAD";
    if method != "GET" && !head {
        reply(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Read-only demo",
            false,
        )?;
        return Ok(());
    }
    let Some(path) = asset(root, target) else {
        reply(
            &mut stream,
            "404 Not Found",
            "text/plain",
            b"Not found",
            head,
        )?;
        return Ok(());
    };
    let mime = match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "json" => "application/json",
        "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    reply(&mut stream, "200 OK", mime, &fs::read(path)?, head)?;
    Ok(())
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let port = match args.as_slice() {
        [] => 7401,
        [flag, value] if flag == "--port" => value.parse::<u16>()?,
        _ => return Err("usage: zxs-demo-serve [--port PORT]".into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("dist")
        .canonicalize()?;
    if !root.join("build-manifest.json").is_file() {
        return Err("build demo assets first".into());
    }
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!(
        "ZXS process demo: http://127.0.0.1:{port}/\nZXS playground: http://127.0.0.1:{port}/playground.html"
    );
    for stream in listener.incoming() {
        let stream = stream?;
        let root = root.clone();
        std::thread::spawn(move || {
            if let Err(error) = connection(stream, &root) {
                eprintln!("Demo request: {error}");
            }
        });
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_paths_cannot_escape_distribution() {
        let root = std::env::temp_dir().canonicalize().unwrap();
        for target in [
            "/../etc/passwd",
            "/%2e%2e/etc/passwd",
            "/foo/../../etc/passwd",
            "/foo\\bar",
            "relative",
        ] {
            assert!(asset(&root, target).is_none());
        }
    }
}
