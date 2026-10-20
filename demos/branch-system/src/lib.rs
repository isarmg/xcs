mod graph;
mod model;
mod scenarios;
#[cfg(test)]
mod tests;
mod view;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    zxs_web::mount("root", view::app)
}
