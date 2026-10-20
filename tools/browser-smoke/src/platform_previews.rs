//! Shared production-WASM UI viewport previews, NOT native-platform screenshots.
//! File selection and server responses remain real. Only delivery timing of one
//! original upload Response is delayed; its bytes/status/headers are not replaced.
use super::*;

const VIEWS: [(&str, u32, u32); 6] = [
    ("linux", 1366, 900),
    ("windows", 1440, 960),
    ("macos", 1512, 982),
    ("android", 412, 915),
    ("ios", 390, 844),
    ("ipados", 1024, 1366),
];

struct UploadGate<'a>(&'a Browser);
impl Drop for UploadGate<'_> {
    fn drop(&mut self) {
        let _ = self.0.script("if(window.__zxsPreviewUpload){window.__zxsPreviewUpload.release();window.__zxsPreviewUpload.restore();}return true");
    }
}
fn pause_upload(browser: &Browser) -> Result<UploadGate<'_>> {
    browser.script(r#"
        const originalFetch = window.fetch;
        let release;
        const gate = new Promise(resolve => release = resolve);
        const state = window.__zxsPreviewUpload = { waiting:false, release, restore:()=>{window.fetch=originalFetch;} };
        window.fetch = async function(...args) {
            const response = await originalFetch.apply(this,args);
            const request = args[0];
            const url = typeof request === 'string' ? request : request.url;
            if (new URL(url,location.href).pathname.match(/^\/v1\/workspaces\/[^/]+\/files\/[^/]+\/content$/)) {
                window.fetch = originalFetch;
                state.waiting = true;
                await gate;
            }
            return response;
        };
        return true;
    "#)?;
    Ok(UploadGate(browser))
}

pub(super) fn run(browser: &Browser, temp: &Path, output: Option<&Path>) -> Result<()> {
    // A sparse real file exercises the product's real browser-size validation.
    // It is never uploaded, and no application model or API reply is injected.
    let too_large = temp.join("超过网页上传上限的合成测试文件.bin");
    File::create(&too_large)?.set_len(64 * 1024 * 1024 + 1)?;
    browser.upload_file(&too_large, false)?;
    browser.click(&browser.button("Transfers", "传输任务")?)?;
    browser.css(".zxs-transfer-card[data-status='rejected']")?;
    browser.css(".zxs-transfer-card[data-status='published']")?;
    browser.click(&browser.css(".zxs-transfer-filters button[data-status='rejected']")?)?;
    if browser.script(
        "return document.querySelectorAll('.zxs-transfer-card:not([data-status=rejected])').length",
    )? != 0
    {
        return Err("transfer filter exposed a nonmatching record".into());
    }
    browser.click(&browser.css(".zxs-transfer-filters button:not([data-status])")?)?;
    browser.click(&browser.button("Files", "文件")?)?;
    let pending = temp.join("跨平台预览正在传输的长中文文件名用于验证窄屏换行与目标路径显示.txt");
    fs::write(&pending, "真实合成内容，用于生产 WASM 上传与布局验收。\n")?;
    let gate = pause_upload(browser)?;
    browser.upload_file(&pending, false)?;
    wait_for("real upload response paused", || {
        if browser.script("return window.__zxsPreviewUpload.waiting")? == true {
            Ok(())
        } else {
            Err("upload has not reached its original server response".into())
        }
    })?;
    browser.click(&browser.button("Transfers", "传输任务")?)?;
    browser.css(".zxs-transfer-card[data-status='uploading']")?;
    let result = capture_views(browser, output);
    // Always restore metrics and release the unmodified response, even on error.
    let cleanup = browser.cdp("Emulation.clearDeviceMetricsOverride", json!({}));
    drop(gate);
    result?;
    cleanup?;
    wait_for("paused upload reaches real published state", || {
        if browser.script("return !document.querySelector('.zxs-transfer-card[data-status=uploading]') && Array.from(document.querySelectorAll('.zxs-transfer-card[data-status=published]')).some(e=>e.textContent.includes('跨平台预览'))")? == true { Ok(()) }
        else { Err("released upload has not published".into()) }
    })?;
    browser.click(&browser.button("Files", "文件")?)?;
    println!(
        "PASS transfer records, real preflight error, status filter and six responsive viewports"
    );
    Ok(())
}

fn capture_views(browser: &Browser, output: Option<&Path>) -> Result<()> {
    if let Some(dir) = output {
        fs::create_dir_all(dir)?;
    }
    let mut previews = Vec::new();
    for (platform, width, height) in VIEWS {
        browser.cdp(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":height,"deviceScaleFactor":1,"mobile":false}),
        )?;
        wait_for("settled transfer viewport and fonts", || {
            if browser.script(&format!("return innerWidth==={width} && innerHeight==={height} && document.fonts.status==='loaded' && !!document.querySelector('.zxs-transfers-page')"))? == true { Ok(()) }
            else { Err("viewport or fonts have not settled".into()) }
        })?;
        thread::sleep(Duration::from_millis(250));
        let geometry = browser.script(r#"
            const page=document.querySelector('.zxs-transfers-page');
            return { width:innerWidth,height:innerHeight,dpr:devicePixelRatio,
                documentWidth:document.documentElement.scrollWidth,
                pageWidth:page.getBoundingClientRect().width,
                overflow:Array.from(page.querySelectorAll('h2,dd,p,button')).filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.tagName),
                controls:Array.from(page.querySelectorAll('button')).every(e=>{const r=e.getBoundingClientRect();return r.height>=36&&r.width>0&&r.left>=0&&r.right<=innerWidth+1;}),
                statuses:Array.from(new Set(Array.from(page.querySelectorAll('.zxs-transfer-card')).map(e=>e.dataset.status))) };
        "#)?;
        if geometry["documentWidth"].as_u64().unwrap_or(u64::MAX) > u64::from(width) + 1
            || geometry["overflow"]
                .as_array()
                .is_none_or(|a| !a.is_empty())
            || geometry["controls"] != true
        {
            return Err(format!(
                "{platform} transfer layout overflow or unreachable control: {geometry}"
            )
            .into());
        }
        // Real Tab movement must retain the product's visible keyboard focus.
        let first = browser.css(".zxs-transfer-filters button:not([data-status])")?;
        browser.click(&first)?;
        browser.keys(&first, "\u{e004}")?;
        if browser.script("const e=document.activeElement;const s=getComputedStyle(e);return e.matches('.zxs-transfer-filters button:focus-visible')&&parseFloat(s.outlineWidth)>0&&s.outlineStyle!=='none'")? != true {
            return Err(format!("{platform} transfer filter keyboard focus is invisible").into());
        }
        // WebDriver scrolls controls into view; return to the real page heading.
        browser.script("window.scrollTo(0,0);return true")?;
        let filename = format!("{platform}-shared-ui-preview.png");
        if let Some(dir) = output {
            let data = browser.command("GET", "/screenshot", None)?;
            let png = STANDARD.decode(data.as_str().ok_or("missing preview PNG")?)?;
            if png.len() < 24 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err("invalid preview PNG".into());
            }
            if u32::from_be_bytes(png[16..20].try_into()?) != width
                || u32::from_be_bytes(png[20..24].try_into()?) != height
            {
                return Err("preview PNG dimensions differ from actual viewport".into());
            }
            fs::write(dir.join(&filename), png)?;
        }
        previews.push(json!({"platform_label":platform,"file":filename,"kind":"shared production WASM UI viewport preview; not native OS/WebView validation","viewport":geometry}));
    }
    if let Some(dir) = output {
        let commit = Command::new("git").args(["rev-parse", "HEAD"]).output()?;
        if !commit.status.success() {
            return Err("cannot record preview source commit".into());
        }
        let dirty = Command::new("git")
            .args(["status", "--porcelain"])
            .output()?;
        let browser_version = browser.cdp("Browser.getVersion", json!({}))?;
        fs::write(
            dir.join("source.json"),
            serde_json::to_vec_pretty(&json!({
                "schema":1,"source_commit":String::from_utf8(commit.stdout)?.trim(),
                "source_worktree_dirty":!dirty.stdout.is_empty(),"browser":browser_version,
                "renderer":"real Chrome rendering production WASM against a real temporary ZXS service",
                "not_native_platform_screenshots":true,"user_agent_override":false,"mobile_emulation":false,
                "fixtures":"synthetic files selected through real upload UI; >64 MiB file rejected by actual preflight; one original upload Response delayed until after capture, without replacing bytes/status/headers",
                "previews":previews
            }))?,
        )?;
        println!(
            "PASS six real shared-UI preview PNGs saved to {}",
            dir.display()
        );
    }
    Ok(())
}
