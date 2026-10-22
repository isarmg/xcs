//! Exercise a slow, failed sign-out before retrying against the real server.
//! Only the first logout transport is interrupted; all business requests and
//! the successful retry continue through the production WASM and HTTP server.
use super::*;

struct Fixture<'a>(&'a Browser);
impl Drop for Fixture<'_> {
    fn drop(&mut self) {
        let _ = self.0.script(
            "if(window.__zxsLogout){window.__zxsLogout.release(); window.__zxsLogout.restore();} return true",
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
        const state = window.__zxsLogout = {
            waiting: false, attempts: 0, writes: [], release,
            restore: () => { window.fetch = originalFetch; }
        };
        window.fetch = async function(...args) {
            const request = args[0];
            const url = typeof request === 'string' ? request : request.url;
            const path = new URL(url, location.href).pathname;
            const method = (args[1]?.method || request.method || 'GET').toUpperCase();
            if (path === '/api/auth/logout') {
                state.attempts++;
                if (state.attempts === 1) {
                    state.waiting = true;
                    await gate;
                    throw new TypeError('Browser smoke: interrupted logout transport');
                }
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
    browser.click(&browser.button("Retry sign out", "重试退出")?)?;
    browser.css("input[autocomplete='current-password']")?;
    wait_for("real logout retry settled", || {
        if browser.script(
            "return window.__zxsLogout.attempts === 2 && !Array.from(document.querySelectorAll('button')).some(button => ['Retry sign out','重试退出'].includes(button.textContent.trim()))",
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
