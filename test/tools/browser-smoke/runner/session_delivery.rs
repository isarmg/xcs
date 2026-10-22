//! Delay one real file-history response, preserving its status, headers and body.
//! The production WASM must accept current metadata and fence it after logout.
use super::*;

pub(super) struct Fixture<'a>(&'a Browser);
impl Drop for Fixture<'_> {
    fn drop(&mut self) {
        let _ = self.0.script("if(window.__zxsDelivery){window.__zxsDelivery.release();window.__zxsDelivery.restore();}return true");
    }
}

pub(super) fn begin_paused_history(browser: &Browser) -> Result<Fixture<'_>> {
    browser.script(r#"
        const originalFetch = window.fetch;
        let release;
        const gate = new Promise(resolve => release = resolve);
        const state = window.__zxsDelivery = {waiting:false,finished:false,status:null,release,restore:()=>{window.fetch=originalFetch;}};
        window.fetch = async function(...args) {
            const response = await originalFetch.apply(this,args);
            const request = args[0];
            const url = typeof request === 'string' ? request : request.url;
            if (!/^\/api\/file-manager\/files\/[^/]+\/history$/.test(new URL(url,location.href).pathname)) return response;
            window.fetch = originalFetch;
            state.status = response.status;
            state.waiting = true;
            await gate;
            setTimeout(()=>{state.finished=true;},500);
            return response;
        };
        return true;
    "#)?;
    let fixture = Fixture(browser);
    browser.click(&browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']",
    )?)?;
    wait_for("real file history paused before delivery", || {
        if browser
            .script("return window.__zxsDelivery.waiting && window.__zxsDelivery.status === 200")?
            == true
        {
            Ok(())
        } else {
            Err("file history has not reached its original successful response".into())
        }
    })?;
    Ok(fixture)
}

pub(super) fn finish(fixture: Fixture<'_>, current_session: bool) -> Result<()> {
    let browser = fixture.0;
    browser.script("window.__zxsDelivery.release();return true")?;
    wait_for("file history continuation", || {
        if browser.script("return window.__zxsDelivery.finished")? == true {
            Ok(())
        } else {
            Err("file history continuation still pending".into())
        }
    })?;
    if current_session {
        wait_for("current-session real version tree", || {
            if browser.script("return document.querySelectorAll('.zxs-version-node').length >= 2")? == true { Ok(()) }
            else { Err("current metadata did not render its two seeded versions".into()) }
        })?;
    } else if browser.script("return !!document.querySelector('input[autocomplete=current-password]') && !document.querySelector('.zxs-entry-content,.zxs-version-node')")? != true {
        return Err("stale metadata response restored private file details after logout".into());
    }
    assert_metadata_only(browser)?;
    Ok(())
}
