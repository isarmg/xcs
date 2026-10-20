//! Actual product controls and computed styles, with bounded decoration fault injection.
use super::*;

pub(super) fn run(browser: &Browser) -> Result<()> {
    let original_theme = browser.script("return document.documentElement.dataset.zxsTheme")?;
    let result = check(browser);
    // Restore even on failure; these are isolated smoke-test browser settings.
    let cleanup = browser
        .script("document.getElementById('zxs-smoke-decoration-fault')?.remove(); return true");
    let media_cleanup = browser.cdp("Emulation.setEmulatedMedia", json!({"features":[]}));
    let theme_cleanup = select_theme(browser, original_theme.as_str().unwrap_or("light"));
    result?;
    cleanup?;
    media_cleanup?;
    theme_cleanup?;
    println!(
        "PASS real input keyboard focus and Select contour fallback visibility in light/dark and forced colors"
    );
    Ok(())
}

fn select_theme(browser: &Browser, theme: &str) -> Result<()> {
    if browser.script("return document.documentElement.dataset.zxsTheme")? != theme {
        let (en, zh) = if theme == "dark" {
            ("Switch to dark mode", "切换到深色模式")
        } else {
            ("Switch to light mode", "切换到浅色模式")
        };
        browser.click(&browser.button(en, zh)?)?;
    }
    wait_for("selected theme", || {
        if browser.script("return document.documentElement.dataset.zxsTheme")? == theme {
            Ok(())
        } else {
            Err("theme did not change".into())
        }
    })
}

fn check(browser: &Browser) -> Result<()> {
    for theme in ["light", "dark"] {
        select_theme(browser, theme)?;
        for forced in ["none", "active"] {
            browser.cdp(
                "Emulation.setEmulatedMedia",
                json!({"features":[{"name":"forced-colors","value":forced}]}),
            )?;
            let expected = forced == "active";
            if browser.script("return matchMedia('(forced-colors: active)').matches")? != expected {
                return Err("Chrome did not apply the requested forced-colors emulation".into());
            }
            browser.click(&browser.button("Account settings", "账号设置")?)?;
            let username = browser.css(".zxs-account-form input[name='username']")?;
            browser.click(&username)?;
            // Real WebDriver Tab input, not focus() or a synthetic DOM event.
            browser.keys(&username, "\u{e004}")?;
            wait_for("visible compact password keyboard focus", || {
                let focus = browser.script(
                    "const e=document.querySelector('.zxs-account-form input[name=current_password]');\
                     const s=getComputedStyle(e);\
                     return document.activeElement===e && e.matches(':focus-visible') && \
                     parseFloat(s.outlineWidth)>0 && s.outlineStyle!=='none'",
                )?;
                if focus == true {
                    Ok(())
                } else {
                    Err(format!("invisible compact input focus in {theme}/{forced}").into())
                }
            })?;
            browser.click(&browser.button("History", "历史记录")?)?;
            let trigger = browser.css(".zxs-history-toolbar .zxs-select-trigger")?;
            browser.click(&trigger)?;
            visible_menu(browser, forced == "active", theme, forced)?;
            browser.keys(&trigger, "\u{e00c}")?; // Escape.

            if forced == "none" {
                // Unsupported decoration is a supported square fallback. Inject
                // only CSS, let the real controller choose its own state, then
                // remove the rule before any further business/screenshot work.
                browser.script(
                    "const s=document.createElement('style');s.id='zxs-smoke-decoration-fault';\
                     s.textContent='.zxs-select-panel{box-shadow:inset 0 0 1px black!important}';\
                     document.head.appendChild(s);return true",
                )?;
                browser.click(&trigger)?;
                visible_menu(browser, true, theme, "unsupported inset shadow")?;
                browser.keys(&trigger, "\u{e00c}")?;
                browser.script(
                    "document.getElementById('zxs-smoke-decoration-fault').remove();return true",
                )?;
            }
        }
    }
    Ok(())
}

fn visible_menu(browser: &Browser, fallback: bool, theme: &str, mode: &str) -> Result<()> {
    wait_for("visible completed Select contour", || {
        let state = browser.script(
            "const p=document.querySelector('.zxs-select-panel:popover-open');\
             if(!p)return null;const s=getComputedStyle(p);const r=p.getBoundingClientRect();\
             return {state:p.dataset.zxsContourState,reason:p.dataset.zxsContourReason,\
                     visible:s.visibility==='visible'&&s.display!=='none'&&r.width>0&&r.height>0}",
        )?;
        let expected = if fallback { "fallback" } else { "clip" };
        if state["state"] == expected && state["visible"] == true {
            Ok(())
        } else {
            Err(format!("Select {theme}/{mode}: expected visible {expected}, got {state}").into())
        }
    })
}
