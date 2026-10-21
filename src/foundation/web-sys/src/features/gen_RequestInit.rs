#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "RequestInit")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `RequestInit` dictionary."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    pub type RequestInit;
    #[doc = "Get the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, getter = "body")]
    pub fn get_body(this: &RequestInit) -> ::wasm_bindgen::JsValue;
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body(this: &RequestInit, val: &::wasm_bindgen::JsValue);
    #[cfg(feature = "Blob")]
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Blob`, `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_blob(this: &RequestInit, val: Option<&Blob>);
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_buffer_source(this: &RequestInit, val: Option<&::js_sys::Object>);
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_u8_slice(this: &RequestInit, val: Option<&mut [u8]>);
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_u8_array(this: &RequestInit, val: Option<&::js_sys::Uint8Array>);

    #[cfg(feature = "UrlSearchParams")]
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`, `UrlSearchParams`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_url_search_params(this: &RequestInit, val: Option<&UrlSearchParams>);
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_str(this: &RequestInit, val: Option<&str>);
    #[cfg(feature = "ReadableStream")]
    #[doc = "Change the `body` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `ReadableStream`, `RequestInit`*"]
    #[wasm_bindgen(method, setter = "body")]
    pub fn set_body_opt_readable_stream(this: &RequestInit, val: Option<&ReadableStream>);

    #[cfg(feature = "RequestCredentials")]
    #[doc = "Get the `credentials` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestCredentials`, `RequestInit`*"]
    #[wasm_bindgen(method, getter = "credentials")]
    pub fn get_credentials(this: &RequestInit) -> Option<RequestCredentials>;
    #[cfg(feature = "RequestCredentials")]
    #[doc = "Change the `credentials` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestCredentials`, `RequestInit`*"]
    #[wasm_bindgen(method, setter = "credentials")]
    pub fn set_credentials(this: &RequestInit, val: RequestCredentials);
    #[doc = "Get the `headers` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, getter = "headers")]
    pub fn get_headers(this: &RequestInit) -> ::wasm_bindgen::JsValue;
    #[doc = "Change the `headers` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "headers")]
    pub fn set_headers(this: &RequestInit, val: &::wasm_bindgen::JsValue);
    #[cfg(feature = "Headers")]
    #[doc = "Change the `headers` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Headers`, `RequestInit`*"]
    #[wasm_bindgen(method, setter = "headers")]
    pub fn set_headers_headers(this: &RequestInit, val: &Headers);
    #[doc = "Change the `headers` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "headers")]
    pub fn set_headers_str_sequence_sequence(this: &RequestInit, val: &::wasm_bindgen::JsValue);
    #[doc = "Change the `headers` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "headers")]
    pub fn set_headers_record_from_str_to_str(this: &RequestInit, val: &::js_sys::Object);
    #[doc = "Get the `integrity` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, getter = "integrity")]
    pub fn get_integrity(this: &RequestInit) -> Option<::alloc::string::String>;
    #[doc = "Change the `integrity` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "integrity")]
    pub fn set_integrity(this: &RequestInit, val: &str);
    #[doc = "Get the `method` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, getter = "method")]
    pub fn get_method(this: &RequestInit) -> Option<::alloc::string::String>;
    #[doc = "Change the `method` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "method")]
    pub fn set_method(this: &RequestInit, val: &str);
    #[cfg(feature = "RequestMode")]
    #[doc = "Get the `mode` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`, `RequestMode`*"]
    #[wasm_bindgen(method, getter = "mode")]
    pub fn get_mode(this: &RequestInit) -> Option<RequestMode>;
    #[cfg(feature = "RequestMode")]
    #[doc = "Change the `mode` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`, `RequestMode`*"]
    #[wasm_bindgen(method, setter = "mode")]
    pub fn set_mode(this: &RequestInit, val: RequestMode);

    #[doc = "Change the `observe` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "observe")]
    pub fn set_observe_callback(this: &RequestInit, val: &::js_sys::Function);

    #[cfg(feature = "RequestRedirect")]
    #[doc = "Get the `redirect` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`, `RequestRedirect`*"]
    #[wasm_bindgen(method, getter = "redirect")]
    pub fn get_redirect(this: &RequestInit) -> Option<RequestRedirect>;
    #[cfg(feature = "RequestRedirect")]
    #[doc = "Change the `redirect` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`, `RequestRedirect`*"]
    #[wasm_bindgen(method, setter = "redirect")]
    pub fn set_redirect(this: &RequestInit, val: RequestRedirect);
    #[doc = "Get the `referrer` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, getter = "referrer")]
    pub fn get_referrer(this: &RequestInit) -> Option<::alloc::string::String>;
    #[doc = "Change the `referrer` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    #[wasm_bindgen(method, setter = "referrer")]
    pub fn set_referrer(this: &RequestInit, val: &str);

    #[cfg(feature = "AbortSignal")]
    #[doc = "Get the `signal` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `AbortSignal`, `RequestInit`*"]
    #[wasm_bindgen(method, getter = "signal")]
    pub fn get_signal(this: &RequestInit) -> Option<AbortSignal>;
    #[cfg(feature = "AbortSignal")]
    #[doc = "Change the `signal` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `AbortSignal`, `RequestInit`*"]
    #[wasm_bindgen(method, setter = "signal")]
    pub fn set_signal(this: &RequestInit, val: Option<&AbortSignal>);
}
impl RequestInit {
    #[doc = "Construct a new `RequestInit`."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `RequestInit`*"]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }
    #[deprecated = "Use `set_body()` instead."]
    pub fn body(&mut self, val: Option<&::wasm_bindgen::JsValue>) -> &mut Self {
        self.set_body(val.unwrap_or(&::wasm_bindgen::JsValue::NULL));
        self
    }

    #[cfg(feature = "RequestCredentials")]
    #[deprecated = "Use `set_credentials()` instead."]
    pub fn credentials(&mut self, val: RequestCredentials) -> &mut Self {
        self.set_credentials(val);
        self
    }
    #[deprecated = "Use `set_headers()` instead."]
    pub fn headers(&mut self, val: &::wasm_bindgen::JsValue) -> &mut Self {
        self.set_headers(val);
        self
    }
    #[deprecated = "Use `set_integrity()` instead."]
    pub fn integrity(&mut self, val: &str) -> &mut Self {
        self.set_integrity(val);
        self
    }
    #[deprecated = "Use `set_method()` instead."]
    pub fn method(&mut self, val: &str) -> &mut Self {
        self.set_method(val);
        self
    }
    #[cfg(feature = "RequestMode")]
    #[deprecated = "Use `set_mode()` instead."]
    pub fn mode(&mut self, val: RequestMode) -> &mut Self {
        self.set_mode(val);
        self
    }

    #[cfg(feature = "RequestRedirect")]
    #[deprecated = "Use `set_redirect()` instead."]
    pub fn redirect(&mut self, val: RequestRedirect) -> &mut Self {
        self.set_redirect(val);
        self
    }
    #[deprecated = "Use `set_referrer()` instead."]
    pub fn referrer(&mut self, val: &str) -> &mut Self {
        self.set_referrer(val);
        self
    }

    #[cfg(feature = "AbortSignal")]
    #[deprecated = "Use `set_signal()` instead."]
    pub fn signal(&mut self, val: Option<&AbortSignal>) -> &mut Self {
        self.set_signal(val);
        self
    }
}
impl Default for RequestInit {
    fn default() -> Self {
        Self::new()
    }
}
