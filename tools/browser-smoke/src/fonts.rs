//! Real font transport faults: delay original bytes or return a font-only 404.
//! Every other resource and API response passes through the real server unchanged.
use super::*;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct FontProxy {
    origin: String,
    gate: Arc<(Mutex<bool>, Condvar)>,
    requested: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl FontProxy {
    fn start(upstream: &str, missing: bool) -> Result<Self> {
        let upstream: u16 = upstream
            .rsplit(':')
            .next()
            .ok_or("missing server port")?
            .parse()?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let origin = format!("http://{}", listener.local_addr()?);
        listener.set_nonblocking(true)?;
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let requested = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (g, r, s) = (gate.clone(), requested.clone(), stop.clone());
        let worker = thread::spawn(move || {
            let mut connections = Vec::new();
            while !s.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut client, _)) => {
                        let (gate, requested) = (g.clone(), r.clone());
                        connections.push(thread::spawn(move || -> std::io::Result<()> {
                            client.set_read_timeout(Some(Duration::from_secs(5)))?;
                            client.set_write_timeout(Some(Duration::from_secs(5)))?;
                            let mut request = Vec::new();
                            while !request.ends_with(b"\r\n\r\n") {
                                if request.len() >= 64 * 1024 { return Ok(()); }
                                let mut byte = [0];
                                client.read_exact(&mut byte)?;
                                request.push(byte[0]);
                            }
                            let request = String::from_utf8_lossy(&request);
                            let path = request.split_whitespace().nth(1).unwrap_or("");
                            if path.starts_with("/fonts/") && path.ends_with(".woff2") {
                                requested.fetch_add(1, Ordering::SeqCst);
                                if missing {
                                    client.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")?;
                                    return Ok(());
                                }
                                let (lock, signal) = &*gate;
                                let _guard = signal.wait_timeout_while(lock.lock().unwrap(), Duration::from_secs(90), |released| !*released).unwrap();
                            }
                            let mut server = TcpStream::connect(("127.0.0.1", upstream))?;
                            server.set_read_timeout(Some(Duration::from_secs(10)))?;
                            // Force EOF framing without modifying response headers/body.
                            for line in request.split("\r\n").filter(|line| !line.is_empty()) {
                                if !line.to_ascii_lowercase().starts_with("connection:") {
                                    write!(server, "{line}\r\n")?;
                                }
                            }
                            server.write_all(b"Connection: close\r\n\r\n")?;
                            std::io::copy(&mut server, &mut client)?;
                            Ok(())
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
            for connection in connections {
                let _ = connection.join();
            }
        });
        Ok(Self {
            origin,
            gate,
            requested,
            stop,
            worker: Some(worker),
        })
    }
    fn release(&self) {
        *self.gate.0.lock().unwrap() = true;
        self.gate.1.notify_all();
    }
}
impl Drop for FontProxy {
    fn drop(&mut self) {
        self.release();
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) fn run(browser: &Browser, origin: &str) -> Result<()> {
    browser.cdp("Network.setCacheDisabled", json!({"cacheDisabled":true}))?;
    with_cleanup(
        || {
            for mode in ["delayed", "missing", "disposed"] {
                let proxy = FontProxy::start(origin, mode == "missing")?;
                // WebDriver navigation waits for window load, which may include fonts.
                // CDP starts navigation without conflating it with app readiness.
                browser.cdp(
                    "Page.navigate",
                    json!({"url":format!("{}/?lang=en", proxy.origin)}),
                )?;
                let input = browser.css("input[autocomplete='username']")?;
                browser.keys(&input, "fallback-is-usable")?;
                wait_for("both original font HTTP requests", || {
                    if proxy.requested.load(Ordering::SeqCst) >= 2 {
                        Ok(())
                    } else {
                        Err("font requests have not reached proxy".into())
                    }
                })?;
                let state = browser.script("return {startup:document.documentElement.dataset.zxsStartup,fonts:document.documentElement.dataset.zxsFonts,value:document.querySelector('input[autocomplete=username]').value,error:!!document.getElementById('zxs-startup-error')}")?;
                if state["startup"] != "ready"
                    || state["value"] != "fallback-is-usable"
                    || state["error"] != false
                    || (mode != "missing" && state["fonts"] != "pending")
                {
                    return Err(format!("{mode} font blocked functional readiness: {state}").into());
                }
                if mode == "disposed" {
                    browser.script("window.dispatchEvent(new PageTransitionEvent('pagehide',{persisted:false})); return true")?;
                }
                proxy.release();
                wait_for("font transport and decoding settled", || {
                    if browser.script("return document.fonts.status === 'loaded'")? == true {
                        Ok(())
                    } else {
                        Err("fonts still loading".into())
                    }
                })?;
                if mode == "disposed" {
                    // Permit queued native FontFaceSet/Promise callbacks to run.
                    thread::sleep(Duration::from_millis(100));
                    if browser.script("return document.documentElement.dataset.zxsFonts === 'pending' && document.getElementById('root').childNodes.length === 0 && window.__zxsSmokeErrors.length === 0")? != true {
                    return Err("late font completion mutated disposed app".into());
                }
                } else {
                    let expected = if mode == "missing" { "failed" } else { "ready" };
                    wait_for("independent font result", || {
                        if browser.script(&format!("return document.documentElement.dataset.zxsFonts === '{expected}' && document.documentElement.dataset.zxsStartup === 'ready' && !document.getElementById('zxs-startup-error')"))? == true { Ok(()) }
                    else { Err("font result or functional readiness incorrect".into()) }
                    })?;
                    browser.keys(
                        &browser.css("input[autocomplete='username']")?,
                        "-still-usable",
                    )?;
                    if mode == "delayed" {
                        wait_for("font-reflow contour geometry", || {
                            if browser.script(r#"return document.fonts.check('16px ZxsUI') && document.fonts.check('bold 16px ZxsUI') && Array.from(document.querySelectorAll('[data-zxs-contour-state="paint"], [data-zxs-contour-state="clip"]')).some(node => {
                                const box=node.getBoundingClientRect();
                                const decoration=node.querySelector(':scope > div[data-zxs-contour-decoration]');
                                if (!decoration) return false;
                                const css=getComputedStyle(decoration);
                                const size=css.backgroundSize.split(',')[0].split(' ').map(parseFloat);
                                const position=css.backgroundPosition.split(',')[0].split(' ').map(parseFloat);
                                return size.length===2 && Math.abs(size[0]+2*position[0]-box.width)<1 && Math.abs(size[1]+2*position[1]-box.height)<1;
                            })"#)? == true { Ok(()) }
                        else { Err("original fonts or reflow contour geometry not ready".into()) }
                        })?;
                    }
                }
                println!(
                    "PASS real {mode} font transport: usable fallback, independent settlement and lifetime fencing"
                );
                browser.cdp("Page.navigate", json!({"url":"about:blank"}))?;
            }
            Ok(())
        },
        || {
            browser.cdp("Network.setCacheDisabled", json!({"cacheDisabled":false}))?;
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .write_all(b"GET /fonts/zxs-regular.woff2 HTTP/1.1\r\nHost: localhost\r\n\r\n")
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
        let (status, body) = http(port, "GET", "/fonts/zxs-bold.woff2", None).unwrap();
        assert_eq!(status, 404);
        assert!(body.is_empty());
        assert_eq!(proxy.requested.load(Ordering::SeqCst), 1);
    }
}
