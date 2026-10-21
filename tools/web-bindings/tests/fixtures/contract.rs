use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct Widget {
    value: Option<String>,
}
#[wasm_bindgen]
impl Widget {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { value: None }
    }
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> Option<String> {
        self.value.clone()
    }
    #[wasm_bindgen(setter)]
    pub fn set_value(&mut self, value: Option<String>) {
        self.value = value;
    }
    pub fn rename(&mut self, value: String) -> String {
        self.value = Some(value);
        self.value.clone().unwrap()
    }
}
#[wasm_bindgen]
pub enum Color {
    Red,
    Green,
    Blue,
}
#[wasm_bindgen]
pub fn unicode_roundtrip(value: String) -> String {
    value
}
// These declarations must be consumed without emitting any declaration file.
#[wasm_bindgen(typescript_custom_section)]
const DECLARATIONS: &str = "export interface DeclarationMustNotLeak { value: string }";
#[wasm_bindgen]
pub enum Mode {
    Ready = "ready",
    Unicode = "文件 Ω",
}
#[wasm_bindgen]
pub fn mode_roundtrip(mode: Mode) -> Mode {
    mode
}
#[wasm_bindgen]
pub enum Message {
    Loading = "loading",
    Text(String),
}
#[wasm_bindgen]
pub fn message_roundtrip(message: Message) -> Message {
    message
}
