//! Production-WASM file-browser screenshots at explicit responsive viewport sizes.
//! CLI-seeded server metadata remains real; no file-content UI or injected model.
use super::*;

const VIEWS: [(u32, u32); 6] = [
    (1366, 900),
    (1440, 960),
    (1512, 982),
    (412, 915),
    (390, 844),
    (1024, 1366),
];

pub(super) fn run(browser: &Browser, output: Option<&Path>) -> Result<()> {
    let result = capture_views(browser, output);
    let cleanup = browser.cdp("Emulation.clearDeviceMetricsOverride", json!({}));
    result?;
    cleanup?;
    println!(
        "PASS metadata-only file browser layout and keyboard focus at six responsive viewports"
    );
    Ok(())
}

fn capture_views(browser: &Browser, output: Option<&Path>) -> Result<()> {
    if let Some(dir) = output {
        fs::create_dir_all(dir)?;
    }
    let mut previews = Vec::new();
    for (width, height) in VIEWS {
        browser.cdp(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":height,"deviceScaleFactor":1,"mobile":false}),
        )?;
        wait_for("settled file-browser viewport and fonts", || {
            if browser.script(&format!("return innerWidth==={width} && innerHeight==={height} && document.fonts.status==='loaded' && !!document.querySelector('.zxs-file-manager')"))? == true { Ok(()) }
            else { Err("viewport or fonts have not settled".into()) }
        })?;
        thread::sleep(Duration::from_millis(250));
        let geometry = browser.script(r#"
            const page=document.querySelector('.zxs-file-manager');
            return { width:innerWidth,height:innerHeight,dpr:devicePixelRatio,
                documentWidth:document.documentElement.scrollWidth,
                pageWidth:page.getBoundingClientRect().width,
                overflow:Array.from(page.querySelectorAll('.zxs-file-toolbar button,.zxs-file-toolbar p')).filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.tagName),
                controls:Array.from(page.querySelectorAll('.zxs-file-toolbar button')).every(e=>{const r=e.getBoundingClientRect();return r.height>=36&&r.width>0&&r.left>=0&&r.right<=innerWidth+1;}) };
        "#)?;
        if geometry["documentWidth"].as_u64().unwrap_or(u64::MAX) > u64::from(width) + 1
            || geometry["overflow"]
                .as_array()
                .is_none_or(|a| !a.is_empty())
            || geometry["controls"] != true
        {
            return Err(format!(
                "{width}x{height} file-browser layout overflow or unreachable control: {geometry}"
            )
            .into());
        }
        // Real Tab movement must retain the product's visible keyboard focus.
        let first = browser.css(".zxs-file-search input")?;
        browser.click(&first)?;
        browser.keys(&first, "\u{e004}")?;
        if browser.script("const e=document.activeElement;const s=getComputedStyle(e);return e.matches(':focus-visible')&&parseFloat(s.outlineWidth)>0&&s.outlineStyle!=='none'")? != true {
            return Err(format!("{width}x{height} file-browser keyboard focus is invisible").into());
        }
        // WebDriver scrolls controls into view; return to the real page heading.
        browser.script("window.scrollTo(0,0);return true")?;
        let filename = format!("viewport-{width}x{height}.png");
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
        previews.push(
            json!({"file":filename,"kind":"production WASM browser viewport","viewport":geometry}),
        );
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
            json_codec::to_vec_pretty(&json!({
                "schema":1,"source_commit":String::from_utf8(commit.stdout)?.trim(),
                "source_worktree_dirty":!dirty.stdout.is_empty(),"browser":browser_version,
                "renderer":"real Chrome rendering production WASM against a real temporary ZXS service",
                "user_agent_override":false,"mobile_emulation":false,
                "fixtures":"two file versions seeded by the internal CLI before server startup; real file list and metadata controls, no upload/download/preview UI",
                "previews":previews
            }))?,
        )?;
        println!(
            "PASS six real browser viewport PNGs saved to {}",
            dir.display()
        );
    }
    Ok(())
}
