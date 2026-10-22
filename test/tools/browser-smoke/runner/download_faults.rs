//! CI-wired production WASM fault regression using actual OPFS writables.
//! Only response bytes, configured short deadlines and fault conditions are injected.
use super::*;
struct Restore<'a>(&'a Browser);
impl Drop for Restore<'_> {
    fn drop(&mut self) {
        let _ = self.0.script("window.fetch=window.__downloadOriginalFetch;window.showSaveFilePicker=window.__downloadOriginalPicker;navigator.storage.estimate=window.__downloadOriginalEstimate;navigator.storage.getDirectory=window.__downloadOriginalDirectory;window.__downloadProbe.releaseRemove?.();if(window.__downloadOriginalRemove)FileSystemDirectoryHandle.prototype.removeEntry=window.__downloadOriginalRemove;return window.__downloadProbe.cleanup?.().catch(()=>{})");
    }
}
pub(super) fn run(browser: &Browser, temp: &Path) -> Result<()> {
    browser.script(include_str!("download_faults.js"))?;
    let _restore = Restore(browser);
    for mode in [
        "valid",
        "valid-large",
        "corruption",
        "truncated",
        "overrun",
        "quota",
        "close-failure",
        "picker-cancel",
        "cancel",
        "idle-timeout",
        "total-timeout",
    ] {
        browser.script(&format!("window.__downloadProbe={{mode:{mode:?},picked:0,writes:0,maxChunk:0,inflight:0,maxInflight:0,bytes:0,closeCalls:0,committed:false,aborted:0,fetched:0}};return true"))?;
        browser.click(&browser.button("Download", "下载")?)?;
        if mode == "cancel" {
            wait_for("actual writable blocked before cancel", || {
                if browser.script("return window.__downloadProbe.writes===1")? == true {
                    Ok(())
                } else {
                    Err("write has not started".into())
                }
            })?;
            browser.click(&browser.button("Cancel download", "取消下载")?)?;
        }
        wait_for("download fault outcome", || {
            if browser.script("return !Array.from(document.querySelectorAll('button')).some(b=>/^(Cancel download|取消下载)$/.test(b.textContent.trim()))")? == true { Ok(()) } else { Err("download remains pending".into()) }
        })?;
        let state = browser
            .script("const {releaseWrite,releaseAbort,cleanup,...state}=window.__downloadProbe;return state")?;
        let valid = matches!(mode, "valid" | "valid-large");
        let close = valid || mode == "close-failure";
        if state["committed"] != valid
            || state["closeCalls"] != u64::from(close)
            || state["maxInflight"].as_u64().unwrap_or(u64::MAX) > 1
            || state["maxChunk"].as_u64().unwrap_or(u64::MAX) > 256 * 1024
        {
            return Err(
                format!("download {mode} violated integrity/commit/backpressure: {state}").into(),
            );
        }
        if mode == "valid-large" && state["savedSize"] != 65 * 1024 * 1024 {
            return Err("large OPFS write size differs".into());
        }
        if mode == "valid" && state["savedText"] != "abc" {
            return Err("OPFS saved bytes differ".into());
        }
        if mode == "picker-cancel" && state["fetched"] != 0 {
            return Err("cancelled picker fetched content".into());
        }
        if (mode == "cancel" || mode.ends_with("-timeout"))
            && (state["fetchAborted"] != true || state["aborted"].as_u64().unwrap_or(0) == 0)
        {
            return Err(format!("{mode} did not abort network and writable: {state}").into());
        }
        browser.script("return window.__downloadProbe.cleanup?.().then(()=>true)??true")?;
    }
    // Cancellation must not rely on abort making an outstanding write settle.
    browser.script("window.__downloadProbe={mode:'uncooperative-write',picked:0,writes:0,maxChunk:0,inflight:0,maxInflight:0,bytes:0,closeCalls:0,committed:false,aborted:0,fetched:0};return true")?;
    browser.click(&browser.button("Download", "下载")?)?;
    wait_for("real OPFS write acknowledgement held", || {
        if browser.script("return window.__downloadProbe.writeWaiting")? == true {
            Ok(())
        } else {
            Err("write not pending".into())
        }
    })?;
    browser.script("window.__downloadProbe.cancelStarted=performance.now();return true")?;
    browser.click(&browser.button("Cancel download", "取消下载")?)?;
    wait_for(
        "responsive foreground cancellation with cleanup pending",
        || {
            if browser.script("return document.body.textContent.includes('DOWNLOAD_CANCELLED')&&window.__downloadProbe.writeWaiting")?==true { Ok(()) } else { Err("cancel waits for pending write".into()) }
        },
    )?;
    if browser.script("return performance.now()-window.__downloadProbe.cancelStarted<2000")? != true
    {
        return Err("foreground cancellation exceeded two seconds while write was pending".into());
    }
    browser.click(&browser.button("Download", "下载")?)?;
    wait_for("quarantine blocks second job", || {
        if browser.script("return document.body.textContent.includes('DOWNLOAD_BUSY')&&window.__downloadProbe.fetched===1&&window.__downloadProbe.picked===1")?==true { Ok(()) } else { Err("second job escaped quarantine".into()) }
    })?;
    browser.script("window.__downloadProbe.releaseWrite();return true")?;
    wait_for("abort independently held after write settles", || {
        if browser.script("return window.__downloadProbe.abortWaiting")? == true {
            Ok(())
        } else {
            Err("cleanup abort not pending".into())
        }
    })?;
    browser.click(&browser.button("Download", "下载")?)?;
    if browser.script("return window.__downloadProbe.fetched===1&&window.__downloadProbe.closeCalls===0&&window.__downloadProbe.abortCalls===1")?!=true { return Err("pending cleanup admitted work, closed cancelled sink, or queued repeated aborts".into()); }
    browser.script("window.__downloadProbe.releaseAbort();return true")?;
    wait_for("real writable abort settled", || {
        if browser.script("return window.__downloadProbe.aborted===1")? == true {
            Ok(())
        } else {
            Err("abort remains pending".into())
        }
    })?;
    browser.script("return window.__downloadProbe.cleanup().then(()=>{window.__downloadProbe={mode:'valid',picked:0,writes:0,maxChunk:0,inflight:0,maxInflight:0,bytes:0,closeCalls:0,committed:false,aborted:0,fetched:0};return true;})")?;
    wait_for("admission reopens after actual cleanup", || {
        browser.click(&browser.button("Download", "下载")?)?;
        if browser.script("return window.__downloadProbe.picked===1")? == true {
            Ok(())
        } else {
            Err("quarantine not cleared yet".into())
        }
    })?;
    wait_for("new job commits after quarantined cleanup", || {
        if browser.script("return window.__downloadProbe.committed")? == true {
            Ok(())
        } else {
            Err("new job not complete".into())
        }
    })?;
    browser.script("return window.__downloadProbe.cleanup().then(()=>true)")?;
    browser.script("window.showSaveFilePicker=undefined;navigator.storage.getDirectory=undefined;window.__downloadProbe={mode:'unsupported',fetched:0,capabilities:0};return true")?;
    browser.click(&browser.button("Download", "下载")?)?;
    wait_for("explicit unsupported disk capability error", || {
        if browser
            .script("return document.body.textContent.includes('DOWNLOAD_STORAGE_UNSUPPORTED')")?
            == true
        {
            Ok(())
        } else {
            Err("missing actionable unsupported-capability error".into())
        }
    })?;
    if browser.script(
        "return window.__downloadProbe.fetched===0&&window.__downloadProbe.capabilities===0",
    )? != true
    {
        return Err("unsupported disk capability requested transfer data".into());
    }
    browser
        .script("navigator.storage.getDirectory=window.__downloadOriginalDirectory;return true")?;
    for mode in [
        "fallback-corruption",
        "fallback-quota",
        "fallback-cancel",
        "fallback-export",
    ] {
        let body_mode = if matches!(mode, "fallback-cancel" | "fallback-export") {
            "fallback-large"
        } else {
            mode
        };
        browser.script(&format!("window.showSaveFilePicker=undefined;window.__downloadProbe={{mode:{body_mode:?},fetched:0}};return true"))?;
        if mode == "fallback-quota" {
            browser
                .script("navigator.storage.estimate=async()=>({usage:0,quota:1});return true")?;
        }
        browser.click(&browser.button("Download", "下载")?)?;
        if matches!(mode, "fallback-corruption" | "fallback-quota") {
            wait_for("failed private download cleaned up", || {
                if browser.script("return !Array.from(document.querySelectorAll('button')).some(b=>/^(Cancel download|取消下载|Save to device|保存到设备)$/.test(b.textContent.trim()))")? == true { Ok(()) } else { Err("failed download remains exportable/pending".into()) }
            })?;
        } else {
            let save = browser.button("Save to device", "保存到设备")?;
            browser.script("for(const b of document.querySelectorAll('button'))if(/^(Download|下载)$/.test(b.textContent.trim()))b.click();return true")?;
            if browser.script("return window.__downloadProbe.fetched")? != 1 {
                return Err("retained copy admitted competing download".into());
            }
            if mode == "fallback-export" {
                browser.click(&save)?;
                wait_for("actual browser exports retained native File", || {
                    if fs::metadata(temp.join("downloads/browser-smoke.txt"))?.len()
                        == 65 * 1024 * 1024
                    {
                        Ok(())
                    } else {
                        Err("browser export not complete".into())
                    }
                })?;
                if spool_count(browser)? != 1 {
                    return Err("export prematurely removed its backing file".into());
                }
                verify_pending_retained_cleanup(browser)?;
            } else {
                browser.click(&browser.button("Cancel and clean up", "取消并清理")?)?;
            }
        }
        wait_for("private spool explicitly removed", || {
            if spool_count(browser)? == 0 {
                Ok(())
            } else {
                Err("private job still exists".into())
            }
        })?;
        browser
            .script("navigator.storage.estimate=window.__downloadOriginalEstimate;return true")?;
    }
    println!(
        "PASS controlled download corruption/length/quota/cancel/deadline faults with actual OPFS writable"
    );
    Ok(())
}

fn spool_count(browser: &Browser) -> Result<u64> {
    browser.script("return navigator.storage.getDirectory().then(root=>root.getDirectoryHandle('zxs-verified-downloads')).then(async parent=>{let count=0;for await(const entry of parent.entries())count++;return count;})")?.as_u64().ok_or_else(||"invalid OPFS job count".into())
}

// Hold removal itself, independently of abort/write. Cancellation must return
// without releasing the single physical cleanup worker or scheduling another.
fn verify_pending_retained_cleanup(browser: &Browser) -> Result<()> {
    browser.script(
        r#"window.__downloadOriginalRemove=FileSystemDirectoryHandle.prototype.removeEntry;
        FileSystemDirectoryHandle.prototype.removeEntry=async function(name,options){
          const p=window.__downloadProbe;
          if(name.startsWith('download_')){
            p.removeCalls=(p.removeCalls||0)+1;p.removeWaiting=true;
            await new Promise(resolve=>{p.releaseRemove=()=>{p.removeWaiting=false;resolve();};});
          }
          return window.__downloadOriginalRemove.call(this,name,options);
        };return true"#,
    )?;
    browser.click(&browser.button("Clean up after saving", "保存完成后清理临时副本")?)?;
    wait_for("retained removal held", || {
        if browser.script("return window.__downloadProbe.removeWaiting")? == true {
            Ok(())
        } else {
            Err("removal not pending".into())
        }
    })?;
    browser.script("window.__downloadProbe.cancelStarted=performance.now();return true")?;
    browser.click(&browser.button("Cancel download", "取消下载")?)?;
    wait_for("retained cleanup foreground cancelled", || {
        if browser.script("return document.body.textContent.includes('DOWNLOAD_CANCELLED')&&window.__downloadProbe.removeWaiting")? == true { Ok(()) }
        else { Err("foreground awaits physical removal".into()) }
    })?;
    if browser.script("return performance.now()-window.__downloadProbe.cancelStarted<2000")? != true
    {
        return Err("retained cleanup cancellation exceeded two seconds".into());
    }
    browser.click(&browser.button("Cancel and clean up", "取消并清理")?)?;
    wait_for("retained cleanup rejects duplicate worker", || {
        if browser.script("return document.body.textContent.includes('DOWNLOAD_BUSY')&&window.__downloadProbe.removeCalls===1")? == true { Ok(()) }
        else { Err("duplicate cleanup admitted".into()) }
    })?;
    browser.script("window.__downloadProbe.releaseRemove();FileSystemDirectoryHandle.prototype.removeEntry=window.__downloadOriginalRemove;return true")?;
    wait_for("late retained removal actually settles", || {
        if spool_count(browser)? == 0 {
            Ok(())
        } else {
            Err("removal still pending".into())
        }
    })?;
    wait_for(
        "late cleanup automatically refreshes retained panel",
        || {
            if browser.script("return !document.querySelector('[aria-label=\"Temporary download copy\"],[aria-label=\"下载临时副本\"]')")? == true { Ok(()) }
        else { Err("stale retained panel remains".into()) }
        },
    )?;
    Ok(())
}
