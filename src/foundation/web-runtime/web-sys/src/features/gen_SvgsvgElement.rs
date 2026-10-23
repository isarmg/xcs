#![allow(unused_imports)]
#![allow(clippy::all)]
use super::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        extends = "SvgGraphicsElement",
        extends = "SvgElement",
        extends = "Element",
        extends = "Node",
        extends = "EventTarget",
        extends = "::js_sys::Object",
        js_name = "SVGSVGElement",
        typescript_type = "SVGSVGElement"
    )]
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[doc = "The `SvgsvgElement` class."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub type SvgsvgElement;

    #[wasm_bindgen(method, getter, js_class = "SVGSVGElement", js_name = "useCurrentView")]
    #[doc = "Getter for the `useCurrentView` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/useCurrentView)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn use_current_view(this: &SvgsvgElement) -> bool;
    #[wasm_bindgen(method, getter, js_class = "SVGSVGElement", js_name = "currentScale")]
    #[doc = "Getter for the `currentScale` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/currentScale)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn current_scale(this: &SvgsvgElement) -> f32;
    #[wasm_bindgen(method, setter, js_class = "SVGSVGElement", js_name = "currentScale")]
    #[doc = "Setter for the `currentScale` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/currentScale)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn set_current_scale(this: &SvgsvgElement, value: f32);

    #[cfg(feature = "SvgAnimatedRect")]
    #[wasm_bindgen(method, getter, js_class = "SVGSVGElement", js_name = "viewBox")]
    #[doc = "Getter for the `viewBox` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/viewBox)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgAnimatedRect`, `SvgsvgElement`*"]
    pub fn view_box(this: &SvgsvgElement) -> SvgAnimatedRect;

    #[wasm_bindgen(method, getter, js_class = "SVGSVGElement", js_name = "zoomAndPan")]
    #[doc = "Getter for the `zoomAndPan` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/zoomAndPan)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn zoom_and_pan(this: &SvgsvgElement) -> u16;
    #[wasm_bindgen(method, setter, js_class = "SVGSVGElement", js_name = "zoomAndPan")]
    #[doc = "Setter for the `zoomAndPan` field of this object."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/zoomAndPan)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn set_zoom_and_pan(this: &SvgsvgElement, value: u16);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "animationsPaused")]
    #[doc = "The `animationsPaused()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/animationsPaused)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn animations_paused(this: &SvgsvgElement) -> bool;

    #[cfg(feature = "SvgMatrix")]
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "createSVGMatrix")]
    #[doc = "The `createSVGMatrix()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/createSVGMatrix)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgMatrix`, `SvgsvgElement`*"]
    pub fn create_svg_matrix(this: &SvgsvgElement) -> SvgMatrix;

    #[cfg(feature = "SvgRect")]
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "createSVGRect")]
    #[doc = "The `createSVGRect()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/createSVGRect)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgRect`, `SvgsvgElement`*"]
    pub fn create_svg_rect(this: &SvgsvgElement) -> SvgRect;

    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "deselectAll")]
    #[doc = "The `deselectAll()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/deselectAll)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn deselect_all(this: &SvgsvgElement);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "forceRedraw")]
    #[doc = "The `forceRedraw()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/forceRedraw)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn force_redraw(this: &SvgsvgElement);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "getCurrentTime")]
    #[doc = "The `getCurrentTime()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/getCurrentTime)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn get_current_time(this: &SvgsvgElement) -> f32;
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "getElementById")]
    #[doc = "The `getElementById()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/getElementById)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn get_element_by_id(this: &SvgsvgElement, element_id: &str) -> Option<Element>;
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "pauseAnimations")]
    #[doc = "The `pauseAnimations()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/pauseAnimations)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn pause_animations(this: &SvgsvgElement);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "setCurrentTime")]
    #[doc = "The `setCurrentTime()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/setCurrentTime)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn set_current_time(this: &SvgsvgElement, seconds: f32);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "suspendRedraw")]
    #[doc = "The `suspendRedraw()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/suspendRedraw)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn suspend_redraw(this: &SvgsvgElement, max_wait_milliseconds: u32) -> u32;
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "unpauseAnimations")]
    #[doc = "The `unpauseAnimations()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/unpauseAnimations)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn unpause_animations(this: &SvgsvgElement);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "unsuspendRedraw")]
    #[doc = "The `unsuspendRedraw()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/unsuspendRedraw)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn unsuspend_redraw(this: &SvgsvgElement, suspend_handle_id: u32);
    #[wasm_bindgen(method, js_class = "SVGSVGElement", js_name = "unsuspendRedrawAll")]
    #[doc = "The `unsuspendRedrawAll()` method."]
    #[doc = ""]
    #[doc = "[MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/SVGSVGElement/unsuspendRedrawAll)"]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub fn unsuspend_redraw_all(this: &SvgsvgElement);
}
impl SvgsvgElement {
    #[doc = "The `SVGSVGElement.SVG_ZOOMANDPAN_UNKNOWN` const."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub const SVG_ZOOMANDPAN_UNKNOWN: u16 = 0i64 as u16;
    #[doc = "The `SVGSVGElement.SVG_ZOOMANDPAN_DISABLE` const."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub const SVG_ZOOMANDPAN_DISABLE: u16 = 1u64 as u16;
    #[doc = "The `SVGSVGElement.SVG_ZOOMANDPAN_MAGNIFY` const."]
    #[doc = ""]
    #[doc = "*This API requires the following crate features to be activated: `SvgsvgElement`*"]
    pub const SVG_ZOOMANDPAN_MAGNIFY: u16 = 2u64 as u16;
}
