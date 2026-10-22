use wasm_bindgen::prelude::*;
// Valid option, unused on a free exported function: a warning by default.
#[wasm_bindgen(catch)]
pub fn unused_option() {}
