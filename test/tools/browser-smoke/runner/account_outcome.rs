//! A list GET overtakes an account POST with an interrupted response. Neither
//! the old snapshot nor the eventually updated snapshot is a terminal receipt.
use super::*;

struct Fixture<'a>(&'a Browser);
impl Drop for Fixture<'_> {
    fn drop(&mut self) {
        let _ = self
            .0
            .script("if(window.__accountOutcome)window.__accountOutcome.restore(); return true");
    }
}

pub(super) fn run(browser: &Browser, password: &str) -> Result<()> {
    browser.click(&browser.button("Administration", "系统管理")?)?;
    let refresh = browser.button("刷新账户 Refresh accounts", "刷新账户 Refresh accounts")?;
    wait_for("initial account snapshot ready", || {
        if browser.command("GET", &format!("/element/{refresh}/enabled"), None)? == true {
            Ok(())
        } else {
            Err("initial accounts are still loading".into())
        }
    })?;
    browser.keys(
        &browser.element(
            "xpath",
            "//section[@aria-label='Administration']//label[span[text()='用户名 Username']]/input",
        )?,
        "outcome-reader",
    )?;
    browser.keys(
        &browser.element(
            "xpath",
            "//section[@aria-label='Administration']//input[@type='password']",
        )?,
        password,
    )?;
    browser.script(r#"
        const original = window.fetch;
        const state = window.__accountOutcome = { posts: 0, gets: 0, release: null, settled: false, status: null,
            restore: () => { window.fetch = original; } };
        window.fetch = async function(request, ...rest) {
            if (new URL(request.url, location.href).pathname === '/api/admin/accounts') {
                if (request.method === 'POST') {
                    state.posts++;
                    const held = request.clone();
                    state.release = async () => {
                        const response = await original.call(window, held);
                        state.status = response.status;
                        await response.text();
                        state.settled = true;
                    };
                    throw new TypeError('Browser smoke: account response interrupted');
                }
                state.gets++;
            }
            return original.call(this, request, ...rest);
        };
        return true;
    "#)?;
    let fixture = Fixture(browser);
    browser.click(&browser.button(
        "保存配置 / 创建预览 Save configuration / Create preview",
        "保存配置 / 创建预览 Save configuration / Create preview",
    )?)?;
    wait_for("unknown account mutation fenced", || assert_fenced(browser))?;
    browser.click(&browser.button("刷新账户 Refresh accounts", "刷新账户 Refresh accounts")?)?;
    wait_for("old account snapshot arrived before held POST", || {
        if browser.script("return window.__accountOutcome.gets > 0 && Array.from(document.querySelectorAll('button')).some(b=>b.textContent==='刷新账户 Refresh accounts'&&!b.disabled) && !document.querySelector('section[aria-label=Administration]').textContent.includes('outcome-reader ·')")? == true {
            assert_fenced(browser)
        } else { Err("old snapshot not observed".into()) }
    })?;
    // Try the opposite edit through actual DOM controls. The action must not
    // issue a second POST even after an apparently successful refresh.
    browser.script("const section=document.querySelector('section[aria-label=Administration]'); const checks=section.querySelectorAll('input[type=checkbox]'); checks[1].click(); for(const b of section.querySelectorAll('button'))if(b.textContent.includes('Save configuration'))b.click(); return true")?;
    assert_fenced(browser)?;
    browser.script("window.__accountOutcome.release(); return true")?;
    wait_for("original account request eventually settled", || {
        if browser.script(
            "return window.__accountOutcome.settled && window.__accountOutcome.status === 200",
        )? == true
        {
            Ok(())
        } else {
            Err("held account POST not settled".into())
        }
    })?;
    browser.click(&browser.button("刷新账户 Refresh accounts", "刷新账户 Refresh accounts")?)?;
    wait_for("updated snapshot still cannot unlock mutation", || {
        if browser.script("return document.querySelector('section[aria-label=Administration]').textContent.includes('outcome-reader · admin=false · disabled=false')")? == true { assert_fenced(browser) } else { Err("updated account snapshot not observed".into()) }
    })?;
    drop(fixture);
    println!(
        "PASS reordered account POST/list GET retains unknown fence and blocks opposite mutation"
    );
    Ok(())
}
fn assert_fenced(browser: &Browser) -> Result<()> {
    if browser.script("const section=document.querySelector('section[aria-label=Administration]'); const save=Array.from(section.querySelectorAll('button')).find(b=>b.textContent.includes('Save configuration')); return window.__accountOutcome.posts === 1 && save?.disabled && document.querySelector('[data-language-toggle]')?.disabled")? == true {
        Ok(())
    } else { Err("account snapshot released the unknown-outcome fence".into()) }
}
