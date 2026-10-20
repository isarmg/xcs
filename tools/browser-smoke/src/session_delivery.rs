//! Timing-only fixture around real server responses. The browser still runs the
//! production WASM download handler and receives the unmodified response bytes.
use super::*;

pub(super) struct Fixture<'a>(&'a Browser);
impl Drop for Fixture<'_> {
    fn drop(&mut self) {
        // Restore interception even when a browser command/assertion fails.
        let _ = self.0.script("if(window.__zxsDelivery){window.__zxsDelivery.release(); window.__zxsDelivery.restore();} return true");
    }
}

pub(super) fn begin_paused_download(browser: &Browser) -> Result<Fixture<'_>> {
    browser.script(r#"
        const originalFetch = window.fetch;
        const originalClick = HTMLAnchorElement.prototype.click;
        const state = window.__zxsDelivery = { waiting: false, finished: false, clicks: 0, bytes: 0, error: null };
        let release;
        const gate = new Promise(resolve => release = resolve);
        state.release = release;
        state.restore = () => {
            window.fetch = originalFetch;
            HTMLAnchorElement.prototype.click = originalClick;
        };
        HTMLAnchorElement.prototype.click = function() {
            if (this.download === 'browser-smoke.txt') state.clicks++;
            return originalClick.call(this);
        };
        window.fetch = async function(...args) {
            const response = await originalFetch.apply(this, args);
            const request = args[0];
            const url = typeof request === 'string' ? request : request.url;
            if (!new URL(url, location.href).pathname.match(/^\/api\/versions\/[^/]+\/content$/)) return response;
            // Intercept exactly this response, not logout or any future request.
            window.fetch = originalFetch;
            const reader = response.body.getReader();
            const stream = new ReadableStream({
                async start(controller) {
                    state.waiting = true;
                    await gate;
                    try {
                        while (true) {
                            const part = await reader.read();
                            if (part.done) break;
                            state.bytes += part.value.byteLength;
                            controller.enqueue(part.value);
                        }
                        controller.close();
                    } catch (error) { state.error = String(error); controller.error(error); }
                    finally {
                        reader.releaseLock();
                        // Let the production Rust future consume the terminal
                        // chunk and run its synchronous verification/delivery.
                        setTimeout(() => { state.finished = true; }, 500);
                    }
                }
            });
            return new Response(stream, {status: response.status, statusText: response.statusText, headers: response.headers});
        };
    "#)?;
    let fixture = Fixture(browser);
    let button = browser.button("Download", "下载")?;
    wait_for("download control ready", || {
        if browser.command("GET", &format!("/element/{button}/enabled"), None)? == true {
            Ok(())
        } else {
            Err("download control is disabled".into())
        }
    })?;
    browser.click(&button)?;
    wait_for("real download paused before body delivery", || {
        if browser.script("return window.__zxsDelivery.waiting")? == true {
            Ok(())
        } else {
            Err("download has not reached response body".into())
        }
    })?;
    Ok(fixture)
}

pub(super) fn finish(
    fixture: Fixture<'_>,
    expected_clicks: u64,
    expected_bytes: usize,
) -> Result<()> {
    let browser = fixture.0;
    browser.script("window.__zxsDelivery.release(); return true")?;
    wait_for("paused download continuation", || {
        if browser.script("return window.__zxsDelivery.finished")? == true {
            Ok(())
        } else {
            Err("download continuation is still pending".into())
        }
    })?;
    let transfer = browser
        .script("return {bytes:window.__zxsDelivery.bytes,error:window.__zxsDelivery.error}")?;
    if transfer["bytes"] != expected_bytes || !transfer["error"].is_null() {
        return Err(format!(
            "paused download did not deliver its complete response body: {transfer}"
        )
        .into());
    }
    let clicks = browser.script("return window.__zxsDelivery.clicks")?;
    browser.script("window.__zxsDelivery.restore(); return true")?;
    if clicks != expected_clicks {
        return Err(format!(
            "session download admission: expected {expected_clicks} click(s), observed {clicks}"
        )
        .into());
    }
    Ok(())
}
