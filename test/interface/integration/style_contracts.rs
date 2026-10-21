//! Source-level guards only. The Chromium fixture checks actual CSS behavior.
#[test]
fn shared_styles_do_not_erase_native_input_focus_globally() {
    let css = include_str!("../../../src/web/public/styles/components/base.css");
    assert!(
        !css.contains(":root :is(input,select,textarea,[role=combobox]):focus"),
        "Focus suppression belongs only to controls with an explicit replacement indicator"
    );
}

#[test]
fn completed_contour_fallbacks_are_not_hidden_as_pending_menus() {
    let css = include_str!("../../../src/web/public/styles/components/select.css");
    let hidden_rule = css
        .lines()
        .find(|line| line.contains(":popover-open") && line.contains("visibility: hidden"))
        .expect("The first-frame pending paint guard should remain");
    assert!(
        hidden_rule.contains(":not([data-zxs-contour-state=fallback])"),
        "A completed square fallback must remain usable, regardless of reason"
    );
    assert!(
        css.contains("[data-zxs-contour-state=fallback][data-zxs-contour-reason=not-visible] { visibility: hidden; }"),
        "A previously closed, unmeasurable menu still waits for its first paint"
    );
}
