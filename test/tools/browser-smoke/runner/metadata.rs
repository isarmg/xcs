//! Real-browser assertions for file browsing and metadata management.
use super::*;

pub(super) fn open_fixture_directory(browser: &Browser) -> Result<()> {
    browser.click(&browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-fixtures']",
    )?)?;
    browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']",
    )?;
    Ok(())
}

pub(super) fn assert_metadata_only(browser: &Browser) -> Result<()> {
    let forbidden = browser.script(r#"
        const labels = ['Upload files','上传文件','Write to current branch','写入当前分支','Download','下载','Preview','预览','Transfers','传输任务'];
        return { controls: Array.from(document.querySelectorAll('button,a')).filter(e => labels.includes(e.textContent.trim()) || labels.includes(e.getAttribute('aria-label'))).map(e=>e.textContent),
            surfaces: document.querySelectorAll('input[type=file],a[download],.zxs-transfers-page,.zxs-transfer-card,.zxs-preview-dialog,.zxs-file-preview').length };
    "#)?;
    if forbidden["controls"] != json!([]) || forbidden["surfaces"] != 0 {
        return Err(format!("removed Web file-transfer/preview UI remains: {forbidden}").into());
    }
    Ok(())
}

pub(super) fn metadata_edit(browser: &Browser) -> Result<()> {
    browser.click(&browser.button("Edit file note", "编辑文件备注")?)?;
    let input = browser.css(".zxs-inline-input")?;
    browser.keys(&input, "browser metadata note")?;
    browser.keys(&input, "\u{e007}")?;
    wait_for("file note transaction completed", || {
        if browser.script("return !document.querySelector('.zxs-inline-input') && document.querySelector('.zxs-entry-content').textContent.includes('browser metadata note')")? == true { Ok(()) }
        else { Err("file note not saved".into()) }
    })?;
    // Navigate away and reload the real metadata from the server.
    browser.click(&browser.button("Files", "文件")?)?;
    browser.click(&browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']",
    )?)?;
    wait_for("persisted file note and version tree", || {
        if browser.script("return document.querySelectorAll('.zxs-version-node').length >= 2 && document.querySelector('.zxs-entry-content').textContent.includes('browser metadata note')")? == true { Ok(()) }
        else { Err("persisted note or seeded version tree missing".into()) }
    })?;
    println!("PASS metadata edit persists across navigation and two-version tree remains visible");
    Ok(())
}

/// Limit this assertion to the current service, not browser assets or other origins.
pub(super) fn is_file_body_request(url: &str, origin: &str) -> bool {
    let Some(path) = url.strip_prefix(origin) else {
        return false;
    };
    let path = path.split(['?', '#']).next().unwrap_or_default();
    path.starts_with("/api/") && (path.ends_with("/content") || path.ends_with("/write-sessions"))
}
