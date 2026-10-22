use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct Counter { value: u32 }
#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new(value: u32) -> Self { Self { value } }
    pub fn increment(&mut self) -> u32 { self.value += 1; self.value }
}
#[wasm_bindgen]
pub fn double(value: u32) -> u32 { value * 2 }
#[wasm_bindgen]
extern "C" { fn imported(value: u32); }
pub fn module_url() -> String { wasm_bindgen::link_to!(inline_js = "export const value = 1;") }
#[test]
fn native_exports_execute() {
    let mut counter = Counter::new(4);
    assert_eq!(counter.increment(), 5);
    assert_eq!(double(9), 18);
}
