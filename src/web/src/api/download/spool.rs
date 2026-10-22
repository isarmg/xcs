//! Origin-private disk staging. Export never deletes or rewrites its backing file.
use super::*;
use wasm_bindgen::closure::Closure;

thread_local! {
    static RETAINED: RefCell<Option<Rc<Spool>>> = const { RefCell::new(None) };
}

pub(super) struct Spool {
    parent: JsValue,
    directory: String,
    handle: JsValue,
    pub(super) writer: JsValue,
    file: RefCell<Option<web_sys::File>>,
    url: RefCell<Option<String>>,
    verified: Cell<bool>,
    name: String,
    admission: SessionAdmission,
    estimated_free: Option<f64>,
    lock: RefCell<Option<JobLock>>,
}
impl Spool {
    /// Returns None only when the required API is unavailable. Permission/quota
    /// failures are errors, never invitations to buffer an arbitrarily large file.
    pub(super) async fn create(
        name: &str,
        admission: SessionAdmission,
        control: &Control,
    ) -> Result<Option<Rc<Self>>, Error> {
        let global = js_sys::global();
        if Reflect::get(&global, &"isSecureContext".into())
            .ok()
            .and_then(|v| v.as_bool())
            != Some(true)
        {
            return Ok(None);
        }
        let navigator = Reflect::get(&global, &"navigator".into()).map_err(download_error)?;
        let storage = Reflect::get(&navigator, &"storage".into()).map_err(download_error)?;
        if !has_method(&storage, "getDirectory") {
            return Ok(None);
        }
        let root = JsFuture::from(call(&storage, "getDirectory", None)?)
            .await
            .map_err(download_error)?;
        control.check(admission)?;
        // Quota is only an estimate, not a reservation or a promise. If present,
        // retain it for conservative admission; real write/close errors still win.
        let estimated_free = if has_method(&storage, "estimate") {
            match call(&storage, "estimate", None) {
                Ok(promise) => JsFuture::from(promise).await.ok().and_then(|estimate| {
                    let quota = Reflect::get(&estimate, &"quota".into()).ok()?.as_f64()?;
                    let usage = Reflect::get(&estimate, &"usage".into()).ok()?.as_f64()?;
                    (quota.is_finite() && usage.is_finite() && quota >= 0.0 && usage >= 0.0)
                        .then_some((quota - usage).max(0.0))
                }),
                Err(_) => None,
            }
        } else {
            None
        };
        control.check(admission)?;
        let options = create_options()?;
        let parent = JsFuture::from(call2(
            &root,
            "getDirectoryHandle",
            &"zxs-verified-downloads".into(),
            &options,
        )?)
        .await
        .map_err(download_error)?;
        control.check(admission)?;
        let directory = fresh_id("download");
        if directory.is_empty() {
            return Err(invalid_response());
        }
        let lock = JobLock::acquire(&navigator, &directory).await?;
        control.check(admission)?;
        let job = JsFuture::from(call2(
            &parent,
            "getDirectoryHandle",
            &directory.clone().into(),
            &options,
        )?)
        .await
        .map_err(download_error)?;
        let mut pending_directory = PendingDirectory {
            parent: parent.clone(),
            name: directory.clone(),
            armed: true,
        };
        // Once the directory exists every subsequent error must remove our own
        // random job directory, including a session change during file creation.
        let result = async {
            control.check(admission)?;
            let handle = JsFuture::from(call2(&job, "getFileHandle", &"content".into(), &options)?)
                .await
                .map_err(download_error)?;
            control.check(admission)?;
            if !has_method(&handle, "createWritable") {
                return Ok(None);
            }
            let writer = JsFuture::from(call(&handle, "createWritable", None)?)
                .await
                .map_err(download_error)?;
            // The caller installs the writer in the cancellation guard before
            // checking session admission again.
            Ok(Some((handle, writer)))
        }
        .await;
        pending_directory.armed = false;
        match result {
            Ok(Some((handle, writer))) => Ok(Some(Rc::new(Self {
                parent,
                directory,
                handle,
                writer,
                file: RefCell::new(None),
                url: RefCell::new(None),
                verified: Cell::new(false),
                name: name.to_owned(),
                admission,
                estimated_free,
                lock: RefCell::new(lock),
            }))),
            other => {
                let cleanup = remove(&parent, &directory).await;
                // No complete Spool exists to offer a retry. Retain just the
                // bounded admission record, not a polling task, after failure.
                if cleanup.is_err() {
                    control.cleanup_failed.set(true);
                }
                let result = match other {
                    Err(error) => Err(error),
                    Ok(None) => cleanup.map(|()| None),
                    _ => unreachable!(),
                };
                result.map_err(|mut error| {
                    if control.cleanup_failed.get() {
                        error.message.push_str(&t(
                            " 临时目录清理也失败。已阻止新下载；请检查浏览器下载状态，清除此网站的存储后重新加载。重新加载可能中断保存，不代表保存已撤销。",
                            " Temporary-directory cleanup also failed. New downloads are blocked; check browser downloads, clear this website's storage, then reload. Reload may interrupt saving and does not prove rollback.",
                        ));
                    }
                    error
                })
            }
        }
    }
    pub(super) fn admit_size(&self, length: u64) -> Result<(), Error> {
        if self
            .estimated_free
            .is_some_and(|available| available < length as f64)
        {
            return Err(error(
                "DOWNLOAD_QUOTA_ESTIMATE",
                t(
                    "浏览器估计可用暂存空间不足。请清理此网站的存储后重试，或使用支持保存文件选择器的浏览器。",
                    "The browser estimates insufficient temporary storage. Clear this website's storage and retry, or use a browser with a save-file picker.",
                ),
            ));
        }
        Ok(())
    }
    pub(super) async fn verify_stored(
        &self,
        expected: u64,
        hash: &str,
        control: &Control,
    ) -> Result<(), Error> {
        control.check(self.admission)?;
        let file: web_sys::File = JsFuture::from(call(&self.handle, "getFile", None)?)
            .await
            .map_err(download_error)?
            .dyn_into()
            .map_err(download_error)?;
        control.check(self.admission)?;
        if file.size() != expected as f64 {
            return Err(mismatch());
        }
        let reader = file
            .stream()
            .get_reader()
            .dyn_into::<web_sys::ReadableStreamDefaultReader>()
            .map_err(|v| download_error(v.into()))?;
        *control.reader.borrow_mut() = Some(reader.clone());
        let mut integrity = Integrity::new(expected);
        let result = async {
            loop {
                let part = JsFuture::from(reader.read())
                    .await
                    .map_err(download_error)?;
                control.check(self.admission)?;
                control.activity.set(control.now_ms());
                if Reflect::get(&part, &"done".into())
                    .map_err(download_error)?
                    .as_bool()
                    == Some(true)
                {
                    break;
                }
                let value = Reflect::get(&part, &"value".into()).map_err(download_error)?;
                if !value.is_instance_of::<Uint8Array>() {
                    return Err(invalid_response());
                }
                let chunk = Uint8Array::new(&value);
                let mut offset = 0;
                while offset < chunk.length() {
                    let end = offset.saturating_add(HASH_CHUNK).min(chunk.length());
                    integrity.update(&chunk.subarray(offset, end).to_vec())?;
                    offset = end;
                }
            }
            integrity.finish_for_session(hash, self.admission)?;
            control.check(self.admission)?;
            *self.file.borrow_mut() = Some(file);
            self.verified.set(true);
            Ok(())
        }
        .await;
        if result.is_err() && control.reader_cancel.borrow().is_none() {
            *control.reader_cancel.borrow_mut() = Some(JsFuture::from(reader.cancel()));
        }
        control.reader.borrow_mut().take();
        reader.release_lock();
        result
    }
    fn revoke(&self) {
        self.verified.set(false);
        if let Some(url) = self.url.borrow_mut().take() {
            let _ = web_sys::Url::revoke_object_url(&url);
        }
        self.file.borrow_mut().take();
    }
    pub(super) async fn cleanup(&self) -> Result<(), Error> {
        self.revoke();
        // Aborting a closed stream can reject harmlessly. Never reopen or write
        // the file while an exported snapshot could still be reading it.
        if let Ok(promise) = call(&self.writer, "abort", None) {
            let _ = JsFuture::from(promise).await;
        }
        self.cleanup_aborted().await
    }
    pub(super) async fn cleanup_aborted(&self) -> Result<(), Error> {
        self.revoke();
        remove(&self.parent, &self.directory).await.map_err(|_| error(
            "DOWNLOAD_CLEANUP_FAILED",
            t("临时副本未能清理，已禁用导出。请重试清理，或通过浏览器清除此网站的存储数据。", "The temporary copy could not be removed; export is disabled. Retry cleanup or clear this website's storage in browser settings."),
        ))?;
        self.lock.borrow_mut().take();
        Ok(())
    }
}
// Best-effort cleanup if the in-progress creation future is dropped after its
// directory is known. An interrupted browser process is documented separately.
struct PendingDirectory {
    parent: JsValue,
    name: String,
    armed: bool,
}
impl Drop for PendingDirectory {
    fn drop(&mut self) {
        if self.armed {
            let parent = self.parent.clone();
            let name = self.name.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = remove(&parent, &name).await;
            });
        }
    }
}
fn create_options() -> Result<JsValue, Error> {
    let options = js_sys::Object::new();
    Reflect::set(&options, &"create".into(), &true.into()).map_err(download_error)?;
    Ok(options.into())
}
fn has_method(target: &JsValue, name: &str) -> bool {
    Reflect::get(target, &name.into())
        .ok()
        .is_some_and(|v| v.is_function())
}
fn call2(
    target: &JsValue,
    name: &str,
    first: &JsValue,
    second: &JsValue,
) -> Result<js_sys::Promise, Error> {
    Reflect::get(target, &name.into())
        .map_err(download_error)?
        .dyn_into::<js_sys::Function>()
        .map_err(download_error)?
        .call2(target, first, second)
        .map_err(download_error)?
        .dyn_into()
        .map_err(download_error)
}
async fn remove(parent: &JsValue, name: &str) -> Result<(), Error> {
    let options = js_sys::Object::new();
    Reflect::set(&options, &"recursive".into(), &true.into()).map_err(download_error)?;
    if let Err(value) = JsFuture::from(call2(parent, "removeEntry", &name.into(), &options)?).await
        && Reflect::get(&value, &"name".into())
            .ok()
            .and_then(|v| v.as_string())
            .as_deref()
            != Some("NotFoundError")
    {
        return Err(download_error(value));
    }
    Ok(())
}

pub(super) fn retained_name() -> Option<String> {
    RETAINED.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|spool| retained_label(&spool.name, spool.admission.check().is_ok()))
    })
}
pub(super) fn retained_label(name: &str, current_session: bool) -> String {
    if current_session {
        name.to_owned()
    } else {
        t(
            "先前会话的临时副本",
            "Temporary copy from a previous session",
        )
    }
}
pub(super) fn export_ready() -> bool {
    RETAINED.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|spool| spool.verified.get() && spool.admission.check().is_ok())
    })
}
pub(super) fn retain(spool: Rc<Spool>) {
    RETAINED.with(|slot| *slot.borrow_mut() = Some(spool));
    notify();
}

/// Synchronous, fresh click. File is an OPFS-backed browser snapshot, not a
/// whole-file WASM vector, array buffer, or new Blob assembled from parts.
pub(super) fn export() -> Result<String, Error> {
    let spool = RETAINED
        .with(|slot| slot.borrow().clone())
        .ok_or_else(invalid_response)?;
    spool.admission.check()?;
    if !spool.verified.get() {
        return Err(invalid_response());
    }
    let file = spool.file.borrow().clone().ok_or_else(invalid_response)?;
    let existing_url = spool.url.borrow().clone();
    let url = if let Some(url) = existing_url {
        url
    } else {
        let url = web_sys::Url::create_object_url_with_blob(&file).map_err(download_error)?;
        *spool.url.borrow_mut() = Some(url.clone());
        url
    };
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(invalid_response)?;
    let anchor = document
        .create_element("a")
        .map_err(download_error)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|v| download_error(v.into()))?;
    anchor.set_href(&url);
    anchor.set_download(&spool.name);
    spool.admission.check()?;
    anchor.click();
    Ok(t(
        "已交给浏览器保存，临时副本仍保留。请确认浏览器保存完成后再清理。",
        "Handed to the browser; the temporary copy is retained. Clean it up only after confirming the browser has finished saving.",
    ))
}
pub(super) async fn cleanup_retained() -> Result<(), Error> {
    // A prior background cleanup may have completed since the UI snapshot.
    // Retrying an already completed cleanup is safe and refreshes that snapshot.
    let Some(spool) = RETAINED.with(|slot| slot.borrow().clone()) else {
        return Ok(());
    };
    spool.cleanup().await?;
    RETAINED.with(|slot| {
        if slot
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &spool))
        {
            slot.borrow_mut().take();
        }
    });
    notify();
    Ok(())
}
pub(super) fn invalidate() {
    RETAINED.with(|slot| {
        if let Some(spool) = slot.borrow().as_ref() {
            // A browser download can still be reading this backing file. Revoke
            // further exports, but only explicit cleanup may remove the copy.
            spool.revoke();
        }
    });
    notify();
}

/// Hold an exclusive per-job Web Lock where supported. There is no directory
/// sweep: without locks, cryptographically random names still avoid cross-tab
/// cleanup. Interrupted browser processes can leave private data for site-data
/// clearing; no expired-time heuristic is used to delete another live job.
struct JobLock {
    release: js_sys::Function,
    request: js_sys::Promise,
    callback: Option<Closure<dyn FnMut(JsValue) -> JsValue>>,
}
impl JobLock {
    async fn acquire(navigator: &JsValue, name: &str) -> Result<Option<Self>, Error> {
        let locks = Reflect::get(navigator, &"locks".into()).map_err(download_error)?;
        if !has_method(&locks, "request") {
            return Ok(None);
        }
        let mut release = None;
        let gate = js_sys::Promise::new(&mut |resolve, _| release = Some(resolve.clone()));
        let mut acquired = None;
        let ready = js_sys::Promise::new(&mut |resolve, _| acquired = Some(resolve.clone()));
        let acquired = acquired.ok_or_else(invalid_response)?;
        let callback = Closure::wrap(Box::new(move |lock: JsValue| -> JsValue {
            let _ = acquired.call1(&JsValue::UNDEFINED, &(!lock.is_null()).into());
            if lock.is_null() {
                JsValue::UNDEFINED
            } else {
                gate.clone().into()
            }
        }) as Box<dyn FnMut(JsValue) -> JsValue>);
        let options = js_sys::Object::new();
        Reflect::set(&options, &"mode".into(), &"exclusive".into()).map_err(download_error)?;
        Reflect::set(&options, &"ifAvailable".into(), &true.into()).map_err(download_error)?;
        let request: js_sys::Promise = Reflect::get(&locks, &"request".into())
            .map_err(download_error)?
            .dyn_into::<js_sys::Function>()
            .map_err(download_error)?
            .call3(
                &locks,
                &format!("zxs-verified-{name}").into(),
                &options,
                callback.as_ref(),
            )
            .map_err(download_error)?
            .dyn_into()
            .map_err(download_error)?;
        let lock = Self {
            release: release.ok_or_else(invalid_response)?,
            request: request.clone(),
            callback: Some(callback),
        };
        let race = js_sys::Array::new();
        race.push(&ready);
        race.push(&request);
        if JsFuture::from(js_sys::Promise::race(&race))
            .await
            .map_err(download_error)?
            .as_bool()
            != Some(true)
        {
            return Err(error(
                "DOWNLOAD_BUSY",
                "The private download job is already in use.",
            ));
        }
        Ok(Some(lock))
    }
}
impl Drop for JobLock {
    fn drop(&mut self) {
        let _ = self.release.call0(&JsValue::UNDEFINED);
        let request = self.request.clone();
        let callback = self.callback.take();
        // If acquisition itself was cancelled, retain the callback until the
        // browser has invoked it and released the lock. No freed WASM closure.
        wasm_bindgen_futures::spawn_local(async move {
            let _ = JsFuture::from(request).await;
            drop(callback);
        });
    }
}
