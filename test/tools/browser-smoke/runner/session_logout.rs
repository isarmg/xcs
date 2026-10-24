//! Exercise a slow, failed sign-out before retrying against the real server.
//! Only the first logout transport is interrupted. Hold the retry's real
//! Response before delivery to verify that recovery stays locked until settled.
use super::*;

struct Fixture<'a>(&'a Browser);
impl Drop for Fixture<'_> {
    fn drop(&mut self) {
        let _ = self.0.script(
            "if(window.__zxsLogout){window.__zxsLogout.release(); window.__zxsLogout.releaseRetry(); window.__zxsLogout.restore();} return true",
        );
    }
}

pub(super) fn run(browser: &Browser, password: &str) -> Result<()> {
    browser.click(&browser.button("Files", "文件")?)?;
    let create = browser.button("New folder", "新建文件夹")?;
    wait_for("write controls enabled before sign out", || {
        if browser.command("GET", &format!("/element/{create}/enabled"), None)? != true {
            return Err("write control was already disabled before sign out".into());
        }
        Ok(())
    })?;
    browser.script(
        r#"
        const originalFetch = window.fetch;
        let release;
        const gate = new Promise(resolve => release = resolve);
        let releaseRetry;
        const retryGate = new Promise(resolve => releaseRetry = resolve);
        const state = window.__zxsLogout = {
            waiting: false, attempts: 0, writes: [], loginAttempts: 0, revocationHeaders: [], release,
            retryWaiting: false, retryStatus: null, releaseRetry,
            restore: () => { window.fetch = originalFetch; }
        };
        window.fetch = async function(...args) {
            const request = args[0];
            const url = typeof request === 'string' ? request : request.url;
            const path = new URL(url, location.href).pathname;
            const method = (args[1]?.method || request.method || 'GET').toUpperCase();
            if (path === '/api/auth/logout') {
                state.attempts++;
                state.revocationHeaders.push([request.headers.get('Authorization'), request.headers.get('X-CSRF-Token')]);
                if (state.attempts === 1) {
                    state.waiting = true;
                    await gate;
                    throw new TypeError('Browser smoke: interrupted logout transport');
                }
                const response = await originalFetch.apply(this, args);
                state.retryStatus = response.status;
                state.retryWaiting = true;
                await retryGate;
                return response;
            } else if (path === '/api/auth/login' || path === '/api/auth/me') {
                state.loginAttempts++;
            } else if (path.startsWith('/api/') && !path.startsWith('/api/auth/')
                && !['GET', 'HEAD'].includes(method)) {
                state.writes.push({path, method});
            }
            return originalFetch.apply(this, args);
        };
        return true;
        "#,
    )?;
    let fixture = Fixture(browser);
    browser.click(&browser.button("Sign out", "退出")?)?;
    wait_for("logout paused with business controls disabled", || {
        if browser.script("return window.__zxsLogout.waiting")? != true {
            return Err("logout has not started".into());
        }
        if browser.command("GET", &format!("/element/{create}/enabled"), None)? != false {
            return Err("business write control remains enabled during logout".into());
        }
        Ok(())
    })?;
    assert_language_reload_blocked(browser)?;
    // Click the real retained DOM controls, rather than bypassing the action
    // with a direct API call. Disabled controls must not open either UI.
    browser.script(
        "for (const button of document.querySelectorAll('button')) { if (['New folder','新建文件夹'].includes(button.textContent.trim())) button.click(); } return true",
    )?;
    if browser.script("return window.__zxsLogout.writes")? != json!([]) {
        return Err("a business mutation was sent after logout began".into());
    }
    if browser.script("return document.querySelector('.zxs-inline-input') === null")? != true {
        return Err("sign out admitted a new namespace editor".into());
    }
    browser.click(&browser.button("Details", "详情")?)?;
    let note = browser.button("Edit file note", "编辑文件备注")?;
    if browser.command("GET", &format!("/element/{note}/enabled"), None)? != false {
        return Err("metadata editor remains enabled during logout".into());
    }
    browser.script("for(const b of document.querySelectorAll('button')){if(['Edit file note','编辑文件备注'].includes(b.getAttribute('aria-label')))b.click();}return true")?;
    if browser.script("return document.querySelector('.zxs-inline-input') === null")? != true {
        return Err("logout admitted a file note editor".into());
    }
    assert_metadata_only(browser)?;
    browser.script("window.__zxsLogout.release(); return true")?;
    let retry = browser.button("Retry sign out", "重试退出")?;
    assert_language_reload_blocked(browser)?;
    if browser.script("return !document.querySelector(\"input[autocomplete='current-password']\") && !document.querySelector('[data-setup-key]') && window.__zxsLogout.loginAttempts === 0")? != true {
        return Err("unconfirmed logout exposed login/setup or restored a session".into());
    }
    browser.click(&retry)?;
    wait_for("real logout retry response paused before delivery", || {
        if browser.script(
            "return window.__zxsLogout.retryWaiting && window.__zxsLogout.retryStatus === 200",
        )? == true
        {
            Ok(())
        } else {
            Err("logout retry has not received its original successful response".into())
        }
    })?;
    assert_language_reload_blocked(browser)?;
    if browser.script("const retry=Array.from(document.querySelectorAll('button')).find(button => ['Retry sign out','重试退出'].includes(button.textContent.trim())); return retry?.disabled && !document.querySelector(\"input[autocomplete='current-password']\") && !document.querySelector('[data-setup-key]') && window.__zxsLogout.loginAttempts === 0")? != true {
        return Err("pending logout retry exposed login/setup or released its recovery shell".into());
    }
    browser.script("window.__zxsLogout.releaseRetry(); return true")?;
    wait_for("real logout retry settled", || {
        if browser.script(
            "return window.__zxsLogout.attempts === 2 && window.__zxsLogout.retryStatus === 200 && !!document.querySelector(\"input[autocomplete='current-password']:not([readonly])\") && !Array.from(document.querySelectorAll('button')).some(button => ['Retry sign out','重试退出'].includes(button.textContent.trim()))",
        )? == true
        {
            Ok(())
        } else {
            Err("logout retry has not completed".into())
        }
    })?;
    if browser.script("return window.__zxsLogout.writes")? != json!([]) {
        return Err("a business mutation was sent during the failed logout/retry".into());
    }
    if browser
        .script("return document.querySelector('[data-language-toggle]')?.disabled === false")?
        != true
    {
        return Err("settled logout retained the language reload fence".into());
    }
    if browser.script("const h=window.__zxsLogout.revocationHeaders; return h.length===2 && !!h[0][1] && JSON.stringify(h[0])===JSON.stringify(h[1]) && window.__zxsLogout.loginAttempts===0")? != true {
        return Err("logout retry lost the original revocation credentials".into());
    }
    drop(fixture);
    browser.keys(
        &browser.css("input[autocomplete='username']")?,
        "smoke-admin",
    )?;
    browser.keys(
        &browser.css("input[autocomplete='current-password']")?,
        password,
    )?;
    browser.click(&browser.button("Sign in", "登录")?)?;
    browser.click(&browser.css(".zxs-disk-entry[data-slot='P01']:not([disabled])")?)?;
    open_fixture_directory(browser)?;
    let create = browser.button("New folder", "新建文件夹")?;
    wait_for("replacement session restores write admission", || {
        if browser.command("GET", &format!("/element/{create}/enabled"), None)? == true {
            Ok(())
        } else {
            Err("new session still has stale logout/write lock".into())
        }
    })?;
    browser.click(&browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']",
    )?)?;
    browser.css(".zxs-entry-version-history")?;
    println!(
        "PASS slow logout blocks new writes, failed revocation retries, and login restores admission"
    );
    Ok(())
}

fn assert_language_reload_blocked(browser: &Browser) -> Result<()> {
    if browser.script("const button = document.querySelector('[data-language-toggle]'); if (!button?.disabled) return false; button.dispatchEvent(new MouseEvent('click', {bubbles:true})); return true")? != true {
        return Err("language reload remained enabled while logout was pending or unconfirmed".into());
    }
    if browser.script("return !!window.__zxsLogout && !!document.querySelector('[data-language-toggle]:disabled')")? != true {
        return Err("language click discarded the original operation/session fence".into());
    }
    Ok(())
}
