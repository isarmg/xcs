#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = "::js_sys::Object", js_name = "BlobPropertyBag")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `BlobPropertyBag` dictionary."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `BlobPropertyBag`*"]
    pub type BlobPropertyBag;

    #[doc = "Get the `type` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `BlobPropertyBag`*"]
    #[wasm_bindgen(method, getter = "type")]
    pub fn get_type(this: &BlobPropertyBag) -> Option<::alloc::string::String>;
    #[doc = "Change the `type` field of this object."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `BlobPropertyBag`*"]
    #[wasm_bindgen(method, setter = "type")]
    pub fn set_type(this: &BlobPropertyBag, val: &str);
}
impl BlobPropertyBag {
    #[doc = "Construct a new `BlobPropertyBag`."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `BlobPropertyBag`*"]
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut ret: Self = ::wasm_bindgen::JsCast::unchecked_into(::js_sys::Object::new());
        ret
    }

    #[deprecated = "Use `set_type()` instead."]
    pub fn type_(&mut self, val: &str) -> &mut Self {
        self.set_type(val);
        self
    }
}
impl Default for BlobPropertyBag {
    fn default() -> Self {
        Self::new()
    }
}
