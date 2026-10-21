#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "::js_sys::Object",
        js_name = "Navigator",
        typescript_type = "Navigator"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Navigator` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub type Navigator;

    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "doNotTrack")]
    #[doc = "Getter for the `doNotTrack` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/doNotTrack)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn do_not_track(this: &Navigator) -> ::alloc::string::String;
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "maxTouchPoints")]
    #[doc = "Getter for the `maxTouchPoints` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/maxTouchPoints)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn max_touch_points(this: &Navigator) -> i32;

    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "activeVRDisplays")]
    #[doc = "Getter for the `activeVRDisplays` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/activeVRDisplays)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn active_vr_displays(this: &Navigator) -> ::js_sys::Array;

    #[wasm_bindgen(
        method,
        getter,
        js_class = "Navigator",
        js_name = "hardwareConcurrency"
    )]
    #[doc = "Getter for the `hardwareConcurrency` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/hardwareConcurrency)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn hardware_concurrency(this: &Navigator) -> f64;
    #[cfg(web_sys_unstable_apis)]
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "deviceMemory")]
    #[doc = "Getter for the `deviceMemory` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/deviceMemory)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    #[doc = ""]
    #[doc = "*This API is unstable and requires `--cfg=web_sys_unstable_apis` to be activated, as"]
    #[doc = "[described in the `wasm-bindgen` guide](https://wasm-bindgen.github.io/wasm-bindgen/web-sys/unstable-apis.html)*"]
    pub fn device_memory(this: &Navigator) -> f64;

    #[wasm_bindgen(catch, method, getter, js_class = "Navigator", js_name = "appCodeName")]
    #[doc = "Getter for the `appCodeName` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/appCodeName)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn app_code_name(this: &Navigator) -> Result<::alloc::string::String, JsValue>;
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "appName")]
    #[doc = "Getter for the `appName` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/appName)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn app_name(this: &Navigator) -> ::alloc::string::String;
    #[wasm_bindgen(catch, method, getter, js_class = "Navigator", js_name = "appVersion")]
    #[doc = "Getter for the `appVersion` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/appVersion)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn app_version(this: &Navigator) -> Result<::alloc::string::String, JsValue>;
    #[wasm_bindgen(catch, method, getter, js_class = "Navigator", js_name = "platform")]
    #[doc = "Getter for the `platform` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/platform)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn platform(this: &Navigator) -> Result<::alloc::string::String, JsValue>;
    #[wasm_bindgen(catch, method, getter, js_class = "Navigator", js_name = "userAgent")]
    #[doc = "Getter for the `userAgent` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/userAgent)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn user_agent(this: &Navigator) -> Result<::alloc::string::String, JsValue>;
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "product")]
    #[doc = "Getter for the `product` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/product)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn product(this: &Navigator) -> ::alloc::string::String;
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "language")]
    #[doc = "Getter for the `language` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/language)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn language(this: &Navigator) -> Option<::alloc::string::String>;
    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "languages")]
    #[doc = "Getter for the `languages` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/languages)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn languages(this: &Navigator) -> ::js_sys::Array;

    #[wasm_bindgen(method, getter, js_class = "Navigator", js_name = "onLine")]
    #[doc = "Getter for the `onLine` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/onLine)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn on_line(this: &Navigator) -> bool;

    #[wasm_bindgen(method, js_class = "Navigator", js_name = "canShare")]
    #[doc = "The `canShare()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/canShare)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn can_share(this: &Navigator) -> bool;

    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "getGamepads")]
    #[doc = "The `getGamepads()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/getGamepads)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn get_gamepads(this: &Navigator) -> Result<::js_sys::Array, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "getVRDisplays")]
    #[doc = "The `getVRDisplays()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/getVRDisplays)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn get_vr_displays(this: &Navigator) -> Result<::js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "requestMIDIAccess")]
    #[doc = "The `requestMIDIAccess()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/requestMIDIAccess)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn request_midi_access(this: &Navigator) -> Result<::js_sys::Promise, JsValue>;

    #[wasm_bindgen(
        method,
        js_class = "Navigator",
        js_name = "requestMediaKeySystemAccess"
    )]
    #[doc = "The `requestMediaKeySystemAccess()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/requestMediaKeySystemAccess)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn request_media_key_system_access(
        this: &Navigator,
        key_system: &str,
        supported_configurations: &::wasm_bindgen::JsValue,
    ) -> ::js_sys::Promise;

    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn send_beacon(this: &Navigator, url: &str) -> Result<bool, JsValue>;
    #[cfg(feature = "Blob")]
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Blob`, `Navigator`*"]
    pub fn send_beacon_with_opt_blob(
        this: &Navigator,
        url: &str,
        data: Option<&Blob>,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn send_beacon_with_opt_buffer_source(
        this: &Navigator,
        url: &str,
        data: Option<&::js_sys::Object>,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn send_beacon_with_opt_u8_array(
        this: &Navigator,
        url: &str,
        data: Option<&mut [u8]>,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn send_beacon_with_opt_js_u8_array(
        this: &Navigator,
        url: &str,
        data: Option<&::js_sys::Uint8Array>,
    ) -> Result<bool, JsValue>;

    #[cfg(feature = "UrlSearchParams")]
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`, `UrlSearchParams`*"]
    pub fn send_beacon_with_opt_url_search_params(
        this: &Navigator,
        url: &str,
        data: Option<&UrlSearchParams>,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn send_beacon_with_opt_str(
        this: &Navigator,
        url: &str,
        data: Option<&str>,
    ) -> Result<bool, JsValue>;
    #[cfg(feature = "ReadableStream")]
    #[wasm_bindgen(catch, method, js_class = "Navigator", js_name = "sendBeacon")]
    #[doc = "The `sendBeacon()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/sendBeacon)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`, `ReadableStream`*"]
    pub fn send_beacon_with_opt_readable_stream(
        this: &Navigator,
        url: &str,
        data: Option<&ReadableStream>,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(method, js_class = "Navigator")]
    #[doc = "The `share()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/share)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn share(this: &Navigator) -> ::js_sys::Promise;

    #[wasm_bindgen(method, js_class = "Navigator", js_name = "vibrate")]
    #[doc = "The `vibrate()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/vibrate)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn vibrate_with_duration(this: &Navigator, pattern: u32) -> bool;
    #[wasm_bindgen(method, js_class = "Navigator", js_name = "vibrate")]
    #[doc = "The `vibrate()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/vibrate)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn vibrate_with_pattern(this: &Navigator, pattern: &::wasm_bindgen::JsValue) -> bool;
    #[wasm_bindgen(
        catch,
        method,
        js_class = "Navigator",
        js_name = "registerContentHandler"
    )]
    #[doc = "The `registerContentHandler()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/registerContentHandler)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn register_content_handler(
        this: &Navigator,
        mime_type: &str,
        url: &str,
        title: &str,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(
        catch,
        method,
        js_class = "Navigator",
        js_name = "registerProtocolHandler"
    )]
    #[doc = "The `registerProtocolHandler()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/registerProtocolHandler)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn register_protocol_handler(
        this: &Navigator,
        scheme: &str,
        url: &str,
        title: &str,
    ) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "Navigator", js_name = "taintEnabled")]
    #[doc = "The `taintEnabled()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/taintEnabled)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Navigator`*"]
    pub fn taint_enabled(this: &Navigator) -> bool;
}
