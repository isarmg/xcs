#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "CharacterData",
        extends = "Node",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "Text",
        typescript_type = "Text"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `Text` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Text)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Text`*"]
    pub type Text;
    #[wasm_bindgen(catch, method, getter, js_class = "Text", js_name = "wholeText")]
    #[doc = "Getter for the `wholeText` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Text/wholeText)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Text`*"]
    pub fn whole_text(this: &Text) -> Result<::alloc::string::String, JsValue>;

    #[wasm_bindgen(catch, constructor, js_class = "Text")]
    #[doc = "The `new Text(..)` constructor, creating a new instance of `Text`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Text/Text)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Text`*"]
    pub fn new() -> Result<Text, JsValue>;
    #[wasm_bindgen(catch, constructor, js_class = "Text")]
    #[doc = "The `new Text(..)` constructor, creating a new instance of `Text`."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Text/Text)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Text`*"]
    pub fn new_with_data(data: &str) -> Result<Text, JsValue>;
    #[wasm_bindgen(catch, method, js_class = "Text", js_name = "splitText")]
    #[doc = "The `splitText()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Text/splitText)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Text`*"]
    pub fn split_text(this: &Text, offset: u32) -> Result<Text, JsValue>;

    #[wasm_bindgen(catch, method, js_class = "Text", js_name = "getBoxQuads")]
    #[doc = "The `getBoxQuads()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/Text/getBoxQuads)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `Text`*"]
    pub fn get_box_quads(this: &Text) -> Result<::js_sys::Array, JsValue>;

}
