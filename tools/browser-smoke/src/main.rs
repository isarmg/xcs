//! Real Chrome/WebDriver smoke test; no mock server or generated application JS.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    env,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::{
        fs::{DirBuilderExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
mod accessibility;
mod demo;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ELEMENT: &str = "element-6066-11e4-a52e-4f735466cecf";

// Each process owns a new group, so failed browser startup cannot leave Chrome children behind.
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let group = format!("-{}", self.0.id());
        let _ = Command::new("kill")
            .args(["-TERM", "--", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        thread::sleep(Duration::from_millis(100));
        let _ = Command::new("kill")
            .args(["-KILL", "--", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(command: &mut Command, log: &Path) -> Result<Process> {
    let output = File::create(log)?;
    Ok(Process(
        command
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .spawn()?,
    ))
}
fn port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

// ChromeDriver speaks HTTP/1.1 with Content-Length. Keep this tiny client local-only.
fn http(port: u16, method: &str, path: &str, body: Option<Value>) -> Result<(u16, Vec<u8>)> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse()?,
        Duration::from_secs(2),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let body = body.map(|v| v.to_string()).unwrap_or_default();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    read_http(&mut stream)
}

// HTTP framing, not socket EOF, marks the end of a response. ChromeDriver keeps
// its socket open even when the request asks for Connection: close.
fn read_http(reader: &mut impl Read) -> Result<(u16, Vec<u8>)> {
    const MAX_HEADERS: usize = 64 * 1024;
    const MAX_BODY: usize = 64 * 1024 * 1024;
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= MAX_HEADERS {
            return Err("HTTP headers exceed size limit".into());
        }
        let mut byte = [0];
        reader.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let header = std::str::from_utf8(&header)?;
    let mut lines = header.split("\r\n");
    let mut status_line = lines
        .next()
        .ok_or("missing HTTP status")?
        .split_whitespace();
    if !matches!(status_line.next(), Some("HTTP/1.1" | "HTTP/1.0")) {
        return Err("invalid HTTP version".into());
    }
    let status: u16 = status_line.next().ok_or("missing HTTP status")?.parse()?;
    let mut content_length = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or("malformed HTTP header")?;
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err("unsupported transfer-encoding in local HTTP response".into());
        }
        if name.eq_ignore_ascii_case("content-length") {
            let length: usize = value.trim().parse()?;
            if content_length.replace(length).is_some() {
                return Err("duplicate HTTP Content-Length".into());
            }
        }
    }
    let length = match content_length {
        Some(length) => length,
        None if status == 204 || status == 304 => 0,
        None => return Err("local HTTP response requires Content-Length".into()),
    };
    if length > MAX_BODY {
        return Err("HTTP response body exceeds size limit".into());
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok((status, body))
}
#[cfg(test)]
fn decode_http(response: &[u8]) -> Result<(u16, Vec<u8>)> {
    read_http(&mut std::io::Cursor::new(response))
}

fn wait_for<T>(label: &str, mut check: impl FnMut() -> Result<T>) -> Result<T> {
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        match check() {
            Ok(value) => return Ok(value),
            Err(error) if Instant::now() >= deadline => {
                return Err(format!("timed out waiting for {label}: {error}").into());
            }
            Err(_) => thread::sleep(Duration::from_millis(150)),
        }
    }
}
struct Browser {
    port: u16,
    session: String,
}
impl Browser {
    fn command(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value> {
        let (status, bytes) = http(
            self.port,
            method,
            &format!("/session/{}{path}", self.session),
            body,
        )?;
        let response: Value = serde_json::from_slice(&bytes)?;
        if status >= 400 || response["value"]["error"].is_string() {
            return Err(format!("WebDriver {method} {path}: {}", response["value"]).into());
        }
        Ok(response["value"].clone())
    }
    fn element(&self, using: &str, selector: &str) -> Result<String> {
        wait_for(selector, || {
            let v = self.command(
                "POST",
                "/element",
                Some(json!({"using":using,"value":selector})),
            )?;
            Ok(v[ELEMENT].as_str().ok_or("missing element id")?.to_owned())
        })
    }
    fn css(&self, selector: &str) -> Result<String> {
        self.element("css selector", selector)
    }
    fn button(&self, en: &str, zh: &str) -> Result<String> {
        self.element("xpath", &format!("//button[normalize-space(.)='{en}' or normalize-space(.)='{zh}' or @aria-label='{en}' or @aria-label='{zh}']"))
    }
    fn click(&self, element: &str) -> Result<()> {
        wait_for("enabled clickable element", || {
            self.command(
                "POST",
                &format!("/element/{element}/click"),
                Some(json!({})),
            )
        })?;
        Ok(())
    }
    fn keys(&self, element: &str, text: &str) -> Result<()> {
        self.command(
            "POST",
            &format!("/element/{element}/value"),
            Some(json!({"text":text})),
        )?;
        Ok(())
    }
    fn cdp(&self, cmd: &str, params: Value) -> Result<Value> {
        self.command(
            "POST",
            "/goog/cdp/execute",
            Some(json!({"cmd":cmd,"params":params})),
        )
    }
    fn script(&self, script: &str) -> Result<Value> {
        self.command(
            "POST",
            "/execute/sync",
            Some(json!({"script":script,"args":[]})),
        )
    }
}
impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.command("DELETE", "", None);
    }
}

fn exercise(
    browser: &Browser,
    origin: &str,
    password: &str,
    temp: &Path,
    screenshot: Option<&Path>,
) -> Result<()> {
    // Observation only. No fetch interception, synthetic responses or application state injection.
    browser.cdp("Page.addScriptToEvaluateOnNewDocument", json!({"source":"window.__zxsSmokeErrors=[];addEventListener('error',e=>window.__zxsSmokeErrors.push(String(e.message)));addEventListener('unhandledrejection',e=>window.__zxsSmokeErrors.push(String(e.reason)));"}))?;
    let demo_capture = env::var_os("ZXS_DEMO_CAPTURE").map(PathBuf::from);
    let url = if screenshot.is_some() || demo_capture.is_some() {
        format!("{origin}/?lang=zh-CN")
    } else {
        origin.to_owned()
    };
    browser.command("POST", "/url", Some(json!({"url":url})))?;
    browser.keys(
        &browser.css("input[autocomplete='username']")?,
        "smoke-admin",
    )?;
    browser.keys(
        &browser.css("input[autocomplete='current-password']")?,
        password,
    )?;
    browser.click(&browser.button("Sign in", "登录")?)?;
    let disk = browser.css(".zxs-disk-entry[data-slot='P01']:not([disabled])")?;
    let disks = browser.command(
        "POST",
        "/elements",
        Some(json!({"using":"css selector","value":".zxs-disk-entry"})),
    )?;
    if disks.as_array().map(Vec::len) != Some(3) {
        return Err("expected exactly three development disk directories".into());
    }
    println!("PASS login and three real disk directories");
    browser.click(&disk)?;
    let upload = browser.button("Upload files", "上传文件")?;
    wait_for("writable disk", || {
        if browser.command("GET", &format!("/element/{upload}/enabled"), None)? == true {
            Ok(())
        } else {
            Err("upload disabled".into())
        }
    })?;
    if let Some(dir) = demo_capture {
        demo::run(browser, temp, &dir)?;
    }
    let payload =
        b"ZXS real browser smoke: complete file content\nUTF-8: \xe6\xb5\x8b\xe8\xaf\x95\n";
    let input = temp.join("browser-smoke.txt");
    fs::write(&input, payload)?;
    browser.keys(
        &browser.css("input[type='file']")?,
        input.to_str().ok_or("non-UTF8 temp path")?,
    )?;
    let file = browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']",
    )?;
    println!("PASS browser file upload and directory publication");
    if let Some(path) = screenshot {
        // Small, public demo fixtures go through the same real browser upload flow.
        // No DOM edits, fake responses, or user data are used for the screenshot.
        for (name, content) in [
            (
                "项目说明.md",
                "# 示例项目\n这些文件仅用于展示 ZXS 文件管理界面。\n",
            ),
            (
                "备份计划.txt",
                "示例计划：主盘 P01 → 备份盘 B01 → 归档盘 C01。\n",
            ),
            (
                "容量记录.csv",
                "盘位,用途\nP01,主存储\nB01,备份\nC01,归档\n",
            ),
        ] {
            let fixture = temp.join(name);
            fs::write(&fixture, content)?;
            browser.keys(
                &browser.css("input[type='file']")?,
                fixture.to_str().ok_or("non-UTF8 fixture path")?,
            )?;
            browser.element(
                "xpath",
                &format!("//button[contains(@class,'zxs-file-name')][normalize-space(.)='{name}']"),
            )?;
        }
        wait_for("loaded screenshot fonts", || {
            if browser.script("return document.fonts.status === 'loaded'")? == true {
                Ok(())
            } else {
                Err("fonts still loading".into())
            }
        })?;
        let encoded = browser.command("GET", "/screenshot", None)?;
        let png = STANDARD.decode(encoded.as_str().ok_or("missing screenshot data")?)?;
        if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err("WebDriver screenshot is not a PNG".into());
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, png)?;
        println!("PASS real browser screenshot saved to {}", path.display());
    }
    // Demo uploads may rerender the table; resolve the original smoke file again.
    let file = if screenshot.is_some() {
        browser.element(
            "xpath",
            "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']",
        )?
    } else {
        file
    };
    browser.click(&file)?;
    browser.css(".zxs-entry-version-history")?;
    browser.click(&browser.button("Download", "下载")?)?;
    let downloaded = temp.join("downloads/browser-smoke.txt");
    wait_for("verified byte-for-byte browser download", || {
        if fs::read(&downloaded)? == payload {
            Ok(())
        } else {
            Err("download content differs".into())
        }
    })?;
    println!("PASS file details, version history and byte-identical download");
    browser.click(&browser.button("History", "历史记录")?)?;
    wait_for("uploaded file in activity history", || {
        if browser.script("return Array.from(document.querySelectorAll('.zxs-history-path')).some(e=>e.textContent.includes('browser-smoke.txt'))")? == true { Ok(()) } else { Err("history missing uploaded file".into()) }
    })?;
    accessibility::run(browser)?;
    browser.click(&browser.button("Sign out", "退出")?)?;
    browser.css("input[autocomplete='current-password']")?;
    if browser.script("return window.__zxsSmokeErrors")? != json!([]) {
        return Err("uncaught error before logout/reload".into());
    }
    // Reload exercises server-side session revocation, rather than merely a UI reset.
    browser.command("POST", "/refresh", Some(json!({})))?;
    browser.css("input[autocomplete='current-password']")?;
    println!("PASS history, logout and anonymous reload");
    let errors = browser.script("return window.__zxsSmokeErrors")?;
    if errors != json!([]) {
        return Err(format!("uncaught browser errors: {errors}").into());
    }
    let logs = browser.command("POST", "/log", Some(json!({"type":"browser"})))?;
    for entry in logs.as_array().ok_or("invalid browser log")? {
        if entry["level"] == "SEVERE" && entry["source"] == "javascript" {
            return Err(format!("browser JavaScript error: {}", entry["message"]).into());
        }
    }
    let logs = browser.command("POST", "/log", Some(json!({"type":"performance"})))?;
    let mut responses = Vec::new();
    for entry in logs.as_array().ok_or("invalid performance log")? {
        let event: Value = serde_json::from_str(
            entry["message"]
                .as_str()
                .ok_or("missing network log message")?,
        )?;
        if event["message"]["method"] == "Network.responseReceived" {
            let r = &event["message"]["params"]["response"];
            let url = r["url"].as_str().unwrap_or_default();
            let status = r["status"].as_f64().unwrap_or(0.0) as u64;
            if url.starts_with(origin) {
                if status >= 400
                    && !(url.ends_with("/v1/auth/me") && status == 401)
                    && !(url.ends_with("/favicon.ico") && status == 404)
                {
                    return Err(format!("unexpected HTTP {status}: {url}").into());
                }
                responses.push((url.to_owned(), status));
            }
        }
    }
    for route in [
        "/v1/auth/login",
        "/v1/file-manager/disks",
        "/v1/file-manager/history",
        "/v1/auth/logout",
    ] {
        if !responses
            .iter()
            .any(|(url, status)| url.ends_with(route) && (200..300).contains(status))
        {
            return Err(format!("no successful real browser response for {route}").into());
        }
    }
    if !responses
        .iter()
        .any(|(url, status)| url.contains(".wasm") && *status == 200)
    {
        return Err("browser did not load the built WASM bundle".into());
    }
    println!("PASS uncaught-error and real HTTP/WASM response checks");
    Ok(())
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut cli_binary = PathBuf::from("target/native/debug/zxs");
    let mut web = PathBuf::from("web/dist");
    let mut screenshot: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cli-bin" => cli_binary = args.next().ok_or("--cli-bin requires a path")?.into(),
            "--web-dir" => web = args.next().ok_or("--web-dir requires a path")?.into(),
            "--screenshot" => {
                screenshot = Some(args.next().ok_or("--screenshot requires a path")?.into());
            }
            _ => {
                return Err(format!(
                    "unknown argument {arg}; usage: zxs-browser-smoke [--cli-bin PATH] [--web-dir PATH] [--screenshot PNG]"
                )
                .into());
            }
        }
    }
    let cli_binary = cli_binary.canonicalize()?;
    let web = web.canonicalize()?;
    if !web.join("index.html").is_file() {
        return Err("build the real web/dist before running this test".into());
    }
    let temp = tempfile::Builder::new()
        .prefix("zxs-browser-smoke-")
        .tempdir()?;
    let root = temp.path();
    let pool = root.join("pool");
    let password_file = root.join("password");
    let mut entropy = [0u8; 24];
    File::open("/dev/urandom")?.read_exact(&mut entropy)?;
    let password: String = entropy.iter().map(|b| format!("{b:02x}")).collect();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&password_file)?
        .write_all(password.as_bytes())?;
    let disks: Vec<_> = ["P01", "B01", "C01"]
        .iter()
        .map(|name| root.join(name))
        .collect();
    for disk in &disks {
        fs::DirBuilder::new().mode(0o700).create(disk)?;
    }
    let init = Command::new(&cli_binary)
        .arg("--root")
        .arg(&pool)
        .args(["init", "--username", "smoke-admin", "--password-file"])
        .arg(&password_file)
        .arg("--development-disks")
        .args(&disks)
        .output()?;
    if !init.status.success() {
        return Err(format!(
            "development pool init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        )
        .into());
    }
    fs::remove_file(password_file)?;
    let server_port = port()?;
    let driver_port = port()?;
    let origin = format!("http://127.0.0.1:{server_port}");
    let server_log = root.join("server.log");
    let driver_log = root.join("driver.log");
    let _server = spawn(
        Command::new(cli_binary)
            .arg("--root")
            .arg(pool)
            .args([
                "serve",
                "--listen",
                &format!("127.0.0.1:{server_port}"),
                "--web-dir",
            ])
            .arg(web),
        &server_log,
    )?;
    let result = (|| -> Result<()> {
        wait_for("loopback ZXS server", || {
            let (status, _) = http(server_port, "GET", "/", None)?;
            if status == 200 {
                Ok(())
            } else {
                Err(format!("HTTP {status}").into())
            }
        })?;
        let driver = env::var_os("CHROMEDRIVER").unwrap_or_else(|| "chromedriver".into());
        let _driver = spawn(
            Command::new(driver).args([
                format!("--port={driver_port}"),
                "--allowed-ips=127.0.0.1".into(),
            ]),
            &driver_log,
        )?;
        wait_for("loopback ChromeDriver", || {
            let (status, _) = http(driver_port, "GET", "/status", None)?;
            if status == 200 {
                Ok(())
            } else {
                Err("driver not ready".into())
            }
        })?;
        let downloads = root.join("downloads");
        fs::create_dir(&downloads)?;
        let window_size = if screenshot.is_some() {
            "--window-size=1440,1000"
        } else {
            "--window-size=1280,900"
        };
        let mut chrome = json!({"args":["--headless=new","--disable-dev-shm-usage",window_size],"prefs":{"download.default_directory":downloads,"download.prompt_for_download":false}});
        if let Some(binary) = env::var_os("CHROME_BIN") {
            chrome["binary"] = binary.to_string_lossy().into_owned().into();
        }
        let (status, body) = http(
            driver_port,
            "POST",
            "/session",
            Some(
                json!({"capabilities":{"alwaysMatch":{"browserName":"chrome","goog:chromeOptions":chrome,"goog:loggingPrefs":{"browser":"ALL","performance":"ALL"}}}}),
            ),
        )?;
        let response: Value = serde_json::from_slice(&body)?;
        if status >= 400 {
            return Err(format!("Chrome startup failed (never skipped): {response}").into());
        }
        let browser = Browser {
            port: driver_port,
            session: response["value"]["sessionId"]
                .as_str()
                .ok_or("missing WebDriver session")?
                .to_owned(),
        };
        browser.cdp(
            "Browser.setDownloadBehavior",
            json!({"behavior":"allow","downloadPath":downloads}),
        )?;
        exercise(&browser, &origin, &password, root, screenshot.as_deref())
    })();
    if result.is_err() {
        for (name, path) in [("ZXS", server_log), ("ChromeDriver", driver_log)] {
            if let Ok(log) = fs::read_to_string(path) {
                eprintln!("--- {name} log ---\n{log}");
            }
        }
    }
    result
}
fn main() {
    if let Err(error) = run() {
        eprintln!("browser smoke FAILED: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn webdriver_http_keeps_binary_body_and_status() {
        let (status, body) =
            decode_http(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n\x00\xff\x01").unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, [0, 255, 1]);
        assert_eq!(
            decode_http(b"HTTP/1.1 500 Error\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap()
                .0,
            500
        );
    }
    #[test]
    fn malformed_or_unsupported_http_fails_closed() {
        assert!(decode_http(b"not HTTP").is_err());
        assert!(decode_http(b"HTTP/1.1 nope\r\n\r\n{}").is_err());
        assert!(
            decode_http(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n").is_err()
        );
    }
    #[test]
    fn content_length_completes_without_waiting_for_keep_alive_eof() {
        // Like a live keep-alive socket: partial reads are possible; reading past
        // the complete response would block instead of returning EOF.
        struct KeepAlive(std::io::Cursor<Vec<u8>>);
        impl Read for KeepAlive {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                if self.0.position() == self.0.get_ref().len() as u64 {
                    return Err(std::io::ErrorKind::WouldBlock.into());
                }
                let limit = output.len().min(3);
                self.0.read(&mut output[..limit])
            }
        }
        let response = b"HTTP/1.1 200 OK\r\ncontent-length: 14\r\nConnection: keep-alive\r\n\r\n{\"value\":true}";
        let mut socket = KeepAlive(std::io::Cursor::new(response.to_vec()));
        let (status, body) = read_http(&mut socket).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, br#"{"value":true}"#);
    }
    #[test]
    fn framing_rejects_truncated_missing_duplicate_or_excessive_lengths() {
        for response in [
            &b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n{}"[..],
            &b"HTTP/1.1 200 OK\r\n\r\n{}"[..],
            &b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\n{}"[..],
            &b"HTTP/1.1 200 OK\r\nContent-Length: 999999999\r\n\r\n"[..],
        ] {
            assert!(decode_http(response).is_err());
        }
        assert_eq!(
            decode_http(b"HTTP/1.1 204 No Content\r\n\r\n").unwrap(),
            (204, vec![])
        );
        assert_eq!(
            decode_http(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}next response")
                .unwrap()
                .1,
            b"{}"
        );
    }
}
