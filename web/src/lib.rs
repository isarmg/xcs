mod api;
mod files;
mod format;
mod history;
mod icons;
mod monitor;
mod ui;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    zxs_web::mount("root", ui::app)
}
