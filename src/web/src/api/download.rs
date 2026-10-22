//! Verified downloads require a transactional picker or origin-private disk sink.
//! Never close the disk sink until both the digest and length have been verified.
use super::*;
use std::{cell::Cell, rc::Rc};

mod spool;

const HASH_CHUNK: u32 = 256 * 1024;
const IDLE_TIMEOUT_MS: f64 = 120_000.0;
thread_local! {
    static NOTIFY_PENDING: Cell<bool> = const { Cell::new(false) };
    static OBSERVER: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    static ACTIVE: RefCell<Option<Rc<Control>>> = const { RefCell::new(None) };
}
pub(super) struct Observer(Rc<dyn Fn()>);
pub(super) fn observe(callback: impl Fn() + 'static) -> Observer {
    let callback: Rc<dyn Fn()> = Rc::new(callback);
    OBSERVER.with(|slot| *slot.borrow_mut() = Some(callback.clone()));
    Observer(callback)
}
impl Drop for Observer {
    fn drop(&mut self) {
        OBSERVER.with(|slot| {
            if slot
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &self.0))
            {
                slot.borrow_mut().take();
            }
        });
    }
}
fn notify() {
    if OBSERVER.with(|slot| slot.borrow().is_none()) {
        return;
    }
    // Coalesce to one microtask: invalidation can originate inside a UI state
    // mutation, so never reenter its signal lock synchronously.
    if NOTIFY_PENDING.with(|pending| pending.replace(true)) {
        return;
    }
    wasm_bindgen_futures::spawn_local(async {
        NOTIFY_PENDING.with(|pending| pending.set(false));
        notify_now();
    });
}
fn notify_now() {
    // Release every storage borrow before calling the UI; it reads fresh state.
    let callback = OBSERVER.with(|slot| slot.borrow().clone());
    if let Some(callback) = callback {
        callback();
    }
}
struct Control {
    clock: web_sys::Performance,
    controller: web_sys::AbortController,
    cancelled: Cell<bool>,
    cleanup_failed: Cell<bool>,
    wake: RefCell<Option<std::task::Waker>>,
    abort: RefCell<Option<JsFuture>>,
    reader_cancel: RefCell<Option<JsFuture>>,
    failure: RefCell<Option<Error>>,
    writer: RefCell<Option<JsValue>>,
    spool: RefCell<Option<Rc<spool::Spool>>>,
    reader: RefCell<Option<web_sys::ReadableStreamDefaultReader>>,
    activity: Cell<f64>,
    idle_timeout: Cell<f64>,
    total_deadline: Cell<f64>,
}
fn new_control(clock: web_sys::Performance) -> Result<Rc<Control>, Error> {
    Ok(Rc::new(Control {
        clock,
        controller: web_sys::AbortController::new().map_err(download_error)?,
        cancelled: Cell::new(false),
        cleanup_failed: Cell::new(false),
        wake: RefCell::new(None),
        abort: RefCell::new(None),
        reader_cancel: RefCell::new(None),
        failure: RefCell::new(None),
        writer: RefCell::new(None),
        spool: RefCell::new(None),
        reader: RefCell::new(None),
        activity: Cell::new(f64::INFINITY),
        idle_timeout: Cell::new(IDLE_TIMEOUT_MS),
        total_deadline: Cell::new(f64::INFINITY),
    }))
}
impl Control {
    // A single monotonic clock domain for the entire transfer.
    fn now_ms(&self) -> f64 {
        self.clock.now()
    }
    fn stop(&self) {
        if self.cancelled.replace(true) {
            return;
        }
        self.controller.abort();
        if let Some(wake) = self.wake.borrow_mut().take() {
            wake.wake();
        }
        // Request cancellation immediately, but do not delete storage or release
        // admission until the physical operation and abort have both settled.
        if let Some(reader) = self.reader.borrow().as_ref() {
            *self.reader_cancel.borrow_mut() = Some(JsFuture::from(reader.cancel()));
        }
        if let Some(writer) = self.writer.borrow().as_ref() {
            *self.abort.borrow_mut() = call(writer, "abort", None).ok().map(JsFuture::from);
        }
    }
    fn check(&self, admission: SessionAdmission) -> Result<(), Error> {
        admission.check()?;
        if self.now_ms() - self.activity.get() > self.idle_timeout.get()
            || self.now_ms() > self.total_deadline.get()
        {
            self.stop();
        }
        if self.cancelled.get() {
            return Err(error(
                "DOWNLOAD_CANCELLED",
                t(
                    "下载已取消或长时间无响应，未确认保存。",
                    "Download cancelled or stalled; saving was not confirmed.",
                ),
            ));
        }
        Ok(())
    }
}
struct Guard(Rc<Control>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.stop();
        ACTIVE.with(|slot| {
            if !self.0.cleanup_failed.get()
                && slot
                    .borrow()
                    .as_ref()
                    .is_some_and(|v| Rc::ptr_eq(v, &self.0))
            {
                slot.borrow_mut().take();
            }
        });
        notify();
    }
}
/// Also used for explicit user cancellation; the session watchdog handles logout.
pub(super) fn cancel() {
    spool::invalidate();
    ACTIVE.with(|slot| {
        if let Some(control) = slot.borrow().as_ref() {
            control.stop();
        }
    });
}
fn busy_error() -> Error {
    let failed = ACTIVE.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|control| control.cleanup_failed.get())
    });
    error(
        "DOWNLOAD_BUSY",
        if failed {
            t(
                "临时目录清理失败，已阻止新下载。请检查浏览器下载状态，清除此网站的存储后重新加载；这不代表先前保存已撤销。",
                "Temporary-directory cleanup failed; new downloads are blocked. Check browser downloads, clear this website's storage, then reload. This does not prove rollback of an earlier save.",
            )
        } else {
            t(
                "下载或清理仍在进行，或仍有临时副本。请等待完成或清理保留的副本。",
                "A download or cleanup is running, or a temporary copy remains. Wait for completion or clean up the retained copy.",
            )
        },
    )
}
fn call(
    target: &JsValue,
    name: &str,
    argument: Option<&JsValue>,
) -> Result<js_sys::Promise, Error> {
    let function = Reflect::get(target, &name.into())
        .map_err(download_error)?
        .dyn_into::<js_sys::Function>()
        .map_err(download_error)?;
    let result = match argument {
        Some(argument) => function.call1(target, argument),
        None => function.call0(target),
    }
    .map_err(download_error)?;
    result.dyn_into().map_err(download_error)
}
fn download_error(value: JsValue) -> Error {
    let name = Reflect::get(&value, &"name".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let (code, zh, en) = match name.as_str() {
        "AbortError" => (
            "DOWNLOAD_CANCELLED",
            "下载已取消，未确认保存。",
            "Download cancelled; saving was not confirmed.",
        ),
        "QuotaExceededError" => (
            "DOWNLOAD_STORAGE_FULL",
            "磁盘空间不足，下载未保存。",
            "Insufficient storage; the download was not saved.",
        ),
        "NotAllowedError" | "SecurityError" => (
            "DOWNLOAD_PERMISSION",
            "无法写入文件；请直接点击下载并允许保存。",
            "Cannot write the file. Click Download directly and allow saving.",
        ),
        _ => (
            "DOWNLOAD_IO_ERROR",
            "下载或写入失败，未确认保存；请检查网络和磁盘空间。",
            "Download or disk write failed; saving was not confirmed. Check the network and free disk space.",
        ),
    };
    error(code, t(zh, en))
}

/// Called synchronously from the click handler, not inside a spawned future:
/// the picker requires transient user activation before the first await.
pub(super) fn start(
    path: &str,
    filename: &str,
) -> impl std::future::Future<Output = Result<String, Error>> + 'static {
    let admission = SessionAdmission::capture();
    let prepared = (|| {
        validate_path(path)?;
        if ACTIVE.with(|slot| slot.borrow().is_some()) || spool::retained_name().is_some() {
            return Err(busy_error());
        }
        let window = web_sys::window().ok_or_else(invalid_response)?;
        let clock = window.performance().ok_or_else(|| {
            error(
                "DOWNLOAD_CLOCK_UNSUPPORTED",
                t(
                    "下载需要现代浏览器提供的单调时钟。请更新浏览器。",
                    "Downloads require a modern browser with a monotonic performance clock. Please update your browser.",
                ),
            )
        })?;
        let control = new_control(clock)?;
        ACTIVE.with(|slot| *slot.borrow_mut() = Some(control.clone()));
        let guard = Guard(control);
        let secure = Reflect::get(window.as_ref(), &"isSecureContext".into())
            .ok()
            .and_then(|v| v.as_bool())
            == Some(true);
        let picker = Reflect::get(window.as_ref(), &"showSaveFilePicker".into())
            .ok()
            .filter(JsValue::is_function);
        let picker = if secure && picker.is_some() {
            let options = js_sys::Object::new();
            Reflect::set(&options, &"suggestedName".into(), &filename.into())
                .map_err(download_error)?;
            Some(call(window.as_ref(), "showSaveFilePicker", Some(&options))?)
        } else {
            None
        };
        Ok((guard, picker))
    })();
    let path = path.to_owned();
    let filename = filename.to_owned();
    async move {
        let (guard, picker) = prepared?;
        let control = guard.0.clone();
        foreground(control, run(guard, picker, path, filename, admission)).await
    }
}
async fn foreground(
    control: Rc<Control>,
    worker: impl std::future::Future<Output = Result<String, Error>> + 'static,
) -> Result<String, Error> {
    let foreground = Foreground(control.clone());
    let outcome = Rc::new(RefCell::new(None));
    let completed = outcome.clone();
    let worker_control = control.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = worker.await;
        *completed.borrow_mut() = Some(result);
        if let Some(wake) = worker_control.wake.borrow_mut().take() {
            wake.wake();
        }
    });
    std::future::poll_fn(move |cx| {
        if let Some(result) = foreground_result(
            outcome.borrow_mut().take(),
            foreground.0.failure.borrow_mut().take(),
            foreground.0.cancelled.get(),
        ) {
            return std::task::Poll::Ready(result);
        }
        *foreground.0.wake.borrow_mut() = Some(cx.waker().clone());
        std::task::Poll::Pending
    })
    .await
}
// Pure foreground protocol: reporting cancellation does not signal physical
// completion or release the worker's admission guard.
fn foreground_result(
    completed: Option<Result<String, Error>>,
    failure: Option<Error>,
    cancelled: bool,
) -> Option<Result<String, Error>> {
    if completed.is_some() {
        return completed;
    }
    if let Some(mut failure) = failure {
        failure.message.push_str(&t(
            " 清理尚未完成；新下载暂时被阻止。",
            " Cleanup is pending; new downloads are temporarily blocked.",
        ));
        return Some(Err(failure));
    }
    cancelled.then(|| Err(error("DOWNLOAD_CANCELLED", t(
        "下载已取消或长时间无响应；清理尚未完成。已经开始提交的保存仍可能完成。清理完成前无法开始新下载。重新加载前请检查浏览器下载状态；重新加载可能中断保存，并不代表保存已撤销。",
        "Download cancelled or stalled; cleanup is pending. A save already being committed may still finish. New downloads remain blocked until cleanup finishes. Check browser downloads before reloading; reload may interrupt saving and does not prove rollback.",
    ))))
}
// Dropping the UI future cancels its worker without dropping physical I/O.
struct Foreground(Rc<Control>);
impl Drop for Foreground {
    fn drop(&mut self) {
        self.0.stop();
    }
}
async fn run(
    guard: Guard,
    picker: Option<js_sys::Promise>,
    path: String,
    filename: String,
    admission: SessionAdmission,
) -> Result<String, Error> {
    let control = guard.0.clone();
    // Do not impose the JSON API's 120-second total deadline on large files.
    // Abort stalled I/O or changed sessions, including while a write awaits.
    let watch_control = control.clone();
    let _watch = gloo_timers::callback::Interval::new(250, move || {
        if admission.check().is_err()
            || watch_control.now_ms() - watch_control.activity.get()
                > watch_control.idle_timeout.get()
            || watch_control.now_ms() > watch_control.total_deadline.get()
        {
            watch_control.stop();
        }
    });
    let mut result = async {
        let (disk, staged) = if let Some(picker) = picker {
            let handle = JsFuture::from(picker).await.map_err(download_error)?;
            control.check(admission)?;
            control.activity.set(control.now_ms());
            let writer = JsFuture::from(call(&handle, "createWritable", None)?)
                .await
                .map_err(download_error)?;
            *control.writer.borrow_mut() = Some(writer.clone());
            control.check(admission)?;
            (writer, None)
        } else {
            control.activity.set(control.now_ms());
            let Some(staged) = spool::Spool::create(&filename, admission, &control).await? else {
                return Err(unsupported_storage());
            };
            let writer = staged.writer.clone();
            *control.writer.borrow_mut() = Some(writer.clone());
            *control.spool.borrow_mut() = Some(staged.clone());
            control.check(admission)?;
            (writer, Some(staged))
        };
        let limits = super::upload_source::transfer_limits().await?;
        control.check(admission)?;
        control.idle_timeout.set(f64::from(limits.idle_timeout_ms));
        let now = control.now_ms();
        control
            .total_deadline
            .set(now + f64::from(limits.total_timeout_ms));
        control.activity.set(now);
        transfer(&path, &disk, staged.as_ref(), &control, admission).await
    }
    .await;
    // The worker, not the UI, owns admission until all physical I/O and
    // cleanup settle. A permanently stuck browser operation quarantines one
    // slot, never an unbounded collection of abandoned downloads.
    if let Err(failure) = &result {
        if !control.cancelled.get() {
            *control.failure.borrow_mut() = Some(failure.clone());
        }
        control.stop();
        let abort = control.abort.borrow_mut().take();
        let writer = control.writer.borrow().clone();
        let abort = abort.or_else(|| {
            writer
                .as_ref()
                .and_then(|w| call(w, "abort", None).ok())
                .map(JsFuture::from)
        });
        if let Some(promise) = abort {
            let _ = promise.await;
        }
        let reader_cancel = control.reader_cancel.borrow_mut().take();
        if let Some(promise) = reader_cancel {
            let _ = promise.await;
        }
        let staged = control.spool.borrow_mut().take();
        if let Some(staged) = staged
            && staged.cleanup_aborted().await.is_err()
            && let Err(error) = &mut result
        {
            spool::retain(staged);
            error.message.push_str(&t(
                " 临时副本清理失败；请通过浏览器清除此网站的存储数据。",
                " Temporary-copy cleanup failed; clear this website's storage in browser settings.",
            ));
        }
    }
    control.spool.borrow_mut().take();
    control.writer.borrow_mut().take();
    result
}

pub(super) struct Integrity {
    expected: u64,
    received: u64,
    hash: blake3::Hasher,
}
impl Integrity {
    pub(super) fn new(expected: u64) -> Self {
        Self {
            expected,
            received: 0,
            hash: blake3::Hasher::new(),
        }
    }
    pub(super) fn update(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let next = self
            .received
            .checked_add(bytes.len() as u64)
            .filter(|v| *v <= self.expected)
            .ok_or_else(mismatch)?;
        self.hash.update(bytes);
        self.received = next;
        Ok(())
    }
    /// Shared production commit gate, also exercised by delayed-session tests.
    pub(super) fn finish_for_session(
        &self,
        hash: &str,
        admission: SessionAdmission,
    ) -> Result<(), Error> {
        self.finish(hash)?;
        admission.check()
    }
    fn finish(&self, hash: &str) -> Result<(), Error> {
        if self.received != self.expected || self.hash.finalize().to_hex().as_str() != hash {
            Err(mismatch())
        } else {
            Ok(())
        }
    }
}
fn mismatch() -> Error {
    error(
        "CHECKSUM_MISMATCH",
        t(
            "下载长度或 BLAKE3 摘要不一致，未提交保存文件。",
            "Downloaded length or BLAKE3 does not match. The file was not committed.",
        ),
    )
}
fn validate_size(length: u64) -> Result<(), Error> {
    if length > 9_007_199_254_740_991 {
        return Err(error(
            "DOWNLOAD_SIZE_UNSUPPORTED",
            "The file exceeds browser integer precision.",
        ));
    }
    Ok(())
}
fn unsupported_storage() -> Error {
    error(
        "DOWNLOAD_STORAGE_UNSUPPORTED",
        t(
            "此浏览器不支持保存文件选择器或可写私有存储，无法安全下载。请通过 HTTPS 使用支持这些功能的现代浏览器。",
            "This browser has neither a save-file picker nor writable private storage. Safe downloading requires HTTPS and a modern browser supporting either capability.",
        ),
    )
}
async fn transfer(
    path: &str,
    disk: &JsValue,
    staged: Option<&Rc<spool::Spool>>,
    control: &Control,
    admission: SessionAdmission,
) -> Result<String, Error> {
    control.check(admission)?;
    let options = RequestInit::new();
    options.set_method("GET");
    options.set_mode(RequestMode::SameOrigin);
    options.set_credentials(RequestCredentials::SameOrigin);
    options.set_redirect(RequestRedirect::Error);
    options.set_signal(Some(&control.controller.signal()));
    let request =
        web_sys::Request::new_with_str_and_init(path, &options).map_err(download_error)?;
    let credentials = CREDENTIALS.with(|saved| saved.borrow().clone());
    if let Some(token) = credentials.token.as_ref() {
        request
            .headers()
            .set("Authorization", &format!("Bearer {token}"))
            .map_err(download_error)?;
    }
    let window = web_sys::window().ok_or_else(invalid_response)?;
    let response: Response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(download_error)?
        .dyn_into()
        .map_err(download_error)?;
    control.check(admission)?;
    if !response.ok() {
        if response.status() == 401 {
            CREDENTIALS.with(|saved| saved.borrow_mut().revoke(admission.0));
        }
        let bytes = read_body(
            &response,
            storage_contract::client_protocol::MAX_RESPONSE_BYTES,
        )
        .await?;
        return match response::decode(response.status(), &bytes) {
            Err(error) => Err(error),
            Ok(_) => Err(invalid_response()),
        };
    }
    let length = response
        .headers()
        .get("Content-Length")
        .map_err(download_error)?
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(invalid_response)?;
    let hash = response
        .headers()
        .get("ETag")
        .map_err(download_error)?
        .map(|v| v.trim_matches('"').to_owned())
        .filter(|v| v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(invalid_response)?;
    validate_size(length)?;
    if let Some(staged) = staged {
        staged.admit_size(length)?;
    }
    let body = response.body().ok_or_else(invalid_response)?;
    let reader = body
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|value| download_error(value.into()))?;
    *control.reader.borrow_mut() = Some(reader.clone());
    let mut integrity = Integrity::new(length);
    let result = async {
        loop {
            control.check(admission)?;
            let part = JsFuture::from(reader.read())
                .await
                .map_err(download_error)?;
            control.check(admission)?;
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
            // A browser controls network chunk size; bound every WASM copy and
            // writer queue entry regardless of that size. Never queue writes.
            let mut offset = 0;
            while offset < chunk.length() {
                let end = offset.saturating_add(HASH_CHUNK).min(chunk.length());
                let piece = chunk.subarray(offset, end);
                let bytes = piece.to_vec();
                integrity.update(&bytes)?;
                control.check(admission)?;
                JsFuture::from(call(disk, "write", Some(piece.as_ref()))?)
                    .await
                    .map_err(download_error)?;
                control.check(admission)?;
                control.activity.set(control.now_ms());
                offset = end;
            }
        }
        integrity.finish_for_session(&hash, admission)?;
        control.check(admission)?;
        // The only commit point. Abort/error never closes this stream.
        JsFuture::from(call(disk, "close", None)?)
            .await
            .map_err(download_error)?;
        control.check(admission)?;
        if let Some(staged) = staged {
            staged.verify_stored(length, &hash, control).await?;
            control.check(admission)?;
            spool::retain(staged.clone());
            control.spool.borrow_mut().take();
            Ok(t("临时副本已完整校验。请点击“保存到设备”导出；保存完成前不要清理。", "Temporary copy verified. Click Save to device to export; do not clean up until saving finishes."))
        } else {
            Ok(t("下载已完整校验并保存。", "Download verified and saved."))
        }
    }
    .await;
    if result.is_err() && control.reader_cancel.borrow().is_none() {
        *control.reader_cancel.borrow_mut() = Some(JsFuture::from(reader.cancel()));
    }
    control.reader.borrow_mut().take();
    reader.release_lock();
    result
}
pub(super) fn retained_copy() -> Option<(String, bool)> {
    spool::retained_name().map(|name| (name, spool::export_ready()))
}
pub(super) fn export() -> Result<String, Error> {
    spool::export()
}
pub(super) async fn cleanup() -> Result<(), Error> {
    if ACTIVE.with(|slot| slot.borrow().is_some()) {
        return Err(busy_error());
    }
    let clock = web_sys::window()
        .and_then(|window| window.performance())
        .ok_or_else(invalid_response)?;
    let control = new_control(clock)?;
    control.activity.set(control.now_ms());
    ACTIVE.with(|slot| *slot.borrow_mut() = Some(control.clone()));
    let guard = Guard(control.clone());
    foreground(control.clone(), async move {
        let watch = control.clone();
        let _watch = gloo_timers::callback::Interval::new(250, move || {
            if watch.now_ms() - watch.activity.get() > IDLE_TIMEOUT_MS {
                watch.stop();
            }
        });
        // The original cleanup owns this guard through its actual completion,
        // even when the UI timed out or logout requested cancellation.
        let result = spool::cleanup_retained().await.map(|()| String::new());
        drop(guard);
        result
    })
    .await
    .map(|_| ())
}

#[cfg(test)]
#[path = "../../../../test/web/unit/api/download/tests.rs"]
mod tests;
