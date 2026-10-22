//! Real response timing fixture with an actual OPFS transactional writable.
//! The controlled picker does not verify the native OS save dialog.
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
        const originalPicker = window.showSaveFilePicker;
        const state = window.__zxsDelivery = { waiting: false, finished: false, closed: 0, aborted: 0, bytes: 0, error: null, saved: null };
        let release;
        const gate = new Promise(resolve => release = resolve);
        state.release = release;
        state.restore = () => { window.fetch = originalFetch; window.showSaveFilePicker = originalPicker; };
        window.showSaveFilePicker = async () => {
            state.activation = navigator.userActivation.isActive;
            const root = await navigator.storage.getDirectory();
            const handle = await root.getFileHandle('zxs-smoke-picker-output', {create:true});
            return {createWritable: async () => {
                const writable = await handle.createWritable();
                return {
                    write: bytes => writable.write(bytes),
                    close: async () => {
                        await writable.close();
                        state.closed++;
                        state.saved = Array.from(new Uint8Array(await (await handle.getFile()).arrayBuffer()));
                        await root.removeEntry('zxs-smoke-picker-output');
                        state.finished = true;
                    },
                    abort: async () => {
                        await writable.abort();
                        state.aborted++;
                        await root.removeEntry('zxs-smoke-picker-output');
                        state.finished = true;
                    }
                };
            }};
        };
        window.fetch = async function(...args) {
            const response = await originalFetch.apply(this, args);
            const request = args[0];
            const url = typeof request === 'string' ? request : request.url;
            if (!new URL(url, location.href).pathname.match(/^\/api\/versions\/[^/]+\/content$/)) return response;
            // Intercept exactly this response, not logout or any future request.
            window.fetch = originalFetch;
            const reader = response.body.getReader();
            let cancelled = false;
            const stream = new ReadableStream({
                async start(controller) {
                    state.waiting = true;
                    await gate;
                    if (cancelled) return;
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
                        // Sink close/abort, not a timer, establishes the outcome.
                    }
                },
                cancel(reason) { cancelled = true; return reader.cancel(reason); }
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
    expected_closes: u64,
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
    // Logout now actively aborts the network body. No delivery is required;
    // allow the reader to reject rather than require a full stale body read.
    if expected_closes > 0 && (transfer["bytes"] != expected_bytes || !transfer["error"].is_null())
    {
        return Err(format!(
            "paused download did not deliver its complete response body: {transfer}"
        )
        .into());
    }
    let closes = browser.script("return window.__zxsDelivery.closed")?;
    if expected_closes == 0 && browser.script("return window.__zxsDelivery.aborted")? != 1 {
        return Err("stale-session writable was not aborted".into());
    }
    if browser.script("return window.__zxsDelivery.activation")? != true {
        return Err("save picker was not called during user activation".into());
    }
    browser.script("window.__zxsDelivery.restore(); return true")?;
    if closes != expected_closes {
        return Err(format!(
            "session download admission: expected {expected_closes} close(s), observed {closes}"
        )
        .into());
    }
    Ok(())
}
