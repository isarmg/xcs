//! Real DOM regression: checkbox state and filtering retain an existing row.
use super::*;

struct Probe<'a>(&'a Browser);
impl Drop for Probe<'_> {
    fn drop(&mut self) {
        let _ = self.0.script("delete window.__zxsFileRow;return true");
    }
}

pub(super) fn run(browser: &Browser) -> Result<()> {
    browser.script("const button=Array.from(document.querySelectorAll('.zxs-file-name')).find(e=>e.textContent.trim()==='browser-smoke.txt');const row=button.closest('tr');window.__zxsFileRow={row,button,check:row.querySelector('.zxs-entry-checkbox')};return true")?;
    let _probe = Probe(browser);
    let checkbox = browser.element("xpath", "//tr[.//button[contains(@class,'zxs-file-name')][normalize-space(.)='browser-smoke.txt']]//input[@type='checkbox']")?;
    browser.click(&checkbox)?;
    wait_for("checked file row retains its DOM and focus", || {
        if browser.script("const s=window.__zxsFileRow;return s.row.isConnected&&s.row.querySelector('.zxs-file-name')===s.button&&s.row.querySelector('.zxs-entry-checkbox')===s.check&&s.check.checked&&document.activeElement===s.check")? == true {
            Ok(())
        } else {
            Err("checking a file replaced row DOM or lost keyboard focus".into())
        }
    })?;
    // Changing query performs filtering without removing this matching key.
    let search = browser.css(".zxs-file-search input")?;
    browser.keys(&search, "browser-smoke")?;
    wait_for("matching filter retains checked row", || {
        if browser.script("const s=window.__zxsFileRow;return s.row.isConnected&&s.row.querySelector('.zxs-entry-checkbox')===s.check&&s.check.checked&&document.querySelectorAll('.zxs-file-table tbody tr').length===1")? == true {
            Ok(())
        } else {
            Err("matching filter replaced row or lost selection".into())
        }
    })?;
    browser.click(&checkbox)?;
    if browser.script("const s=window.__zxsFileRow;return s.row.isConnected&&!s.check.checked&&document.activeElement===s.check")? != true {
        return Err("unchecking a filtered file replaced its DOM or lost focus".into());
    }
    // Use the normal input event path, retaining all subsequent smoke fixtures.
    browser.script("const input=document.querySelector('.zxs-file-search input');input.value='';input.dispatchEvent(new Event('input',{bubbles:true}));return true")?;
    wait_for("cleared file query", || {
        if browser.script("return document.querySelector('.zxs-file-search input').value===''&&window.__zxsFileRow.row.isConnected")? == true {
            Ok(())
        } else {
            Err("clearing query did not retain the row".into())
        }
    })?;
    println!("PASS keyed file-row DOM, checkbox selection, focus and matching-filter retention");
    Ok(())
}
