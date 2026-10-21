#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn clamp_nan_min() {
    clamp(0., f32::NAN, 1.);
}
