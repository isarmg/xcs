//! Real filesystem File -> bounded browser hash -> real server publication.
//! Transparent observers forward native operations; no upload responses are forged.
use super::*;

pub(super) struct Probe<'a>(&'a Browser);
impl Drop for Probe<'_> {
    fn drop(&mut self) {
        let _ = self
            .0
            .script("window.__zxsLargeUpload?.restore(); return true");
    }
}
pub(super) fn observe(browser: &Browser) -> Result<Probe<'_>> {
    browser.script(r#"
        const originalRequest = window.Request, slice = Blob.prototype.slice;
        const arrayBuffer = Blob.prototype.arrayBuffer, fetch = window.fetch;
        const chunks = new WeakSet();
        const state = window.__zxsLargeUpload = { slices:0,maxSlice:0,maxRead:0,workspaces:0,originalFileBody:false };
        const selected = event => { if(event.target instanceof HTMLInputElement && event.target.type==='file') state.file=event.target.files[0]; };
        document.addEventListener('change',selected,true);
        Blob.prototype.slice=function(start,end,...rest) {
            if(this===state.file) { state.slices++;state.maxSlice=Math.max(state.maxSlice,end-start); }
            const result = slice.call(this,start,end,...rest);
            if(this===state.file) chunks.add(result);
            return result;
        };
        Blob.prototype.arrayBuffer=function() {if(this===state.file||chunks.has(this)) state.maxRead=Math.max(state.maxRead,this.size);return arrayBuffer.call(this);};
        window.Request=new Proxy(originalRequest,{construct(target,args,newTarget) {
            const path=new URL(typeof args[0]==='string'?args[0]:args[0].url,location.href).pathname;
            if(/^\/api\/workspaces\/[^/]+\/files\/[^/]+\/content$/.test(path)) state.originalFileBody=args[1]?.body===state.file && state.file instanceof File;
            return Reflect.construct(target,args,newTarget);
        }});
        window.fetch=function(...args) {
            const r=args[0],path=new URL(typeof r==='string'?r:r.url,location.href).pathname;
            const method=(args[1]?.method||r.method||'GET').toUpperCase();
            if(method==='POST' && /\/(workspaces|write-sessions)$/.test(path)) state.workspaces++;
            return fetch.apply(this,args);
        };
        state.restore=()=>{window.Request=originalRequest;window.fetch=fetch;Blob.prototype.slice=slice;Blob.prototype.arrayBuffer=arrayBuffer;document.removeEventListener('change',selected,true);};
        return true;
    "#)?;
    Ok(Probe(browser))
}

pub(super) fn run(browser: &Browser, temp: &Path) -> Result<()> {
    let file = temp.join("large-upload-65MiB.bin");
    File::create(&file)?.set_len(65 * 1024 * 1024)?;
    let probe = observe(browser)?;
    browser.upload_file(&file, false)?;
    browser.element(
        "xpath",
        "//button[contains(@class,'zxs-file-name')][normalize-space(.)='large-upload-65MiB.bin']",
    )?;
    let state = browser.script("const s=window.__zxsLargeUpload;return {slices:s.slices,maxSlice:s.maxSlice,maxRead:s.maxRead,originalFileBody:s.originalFileBody,workspaces:s.workspaces}")?;
    if state["slices"] != 65
        || state["maxSlice"] != 1024 * 1024
        || state["maxRead"].as_u64().unwrap_or(u64::MAX) > 1024 * 1024
        || state["originalFileBody"] != true
        || state["workspaces"] != 1
    {
        return Err(format!(
            "large File upload violated bounded-read/original-body contract: {state}"
        )
        .into());
    }
    drop(probe);
    println!("PASS real 65 MiB File upload published; 1 MiB reads and original File request body");
    Ok(())
}
