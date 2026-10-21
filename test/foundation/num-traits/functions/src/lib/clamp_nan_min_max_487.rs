#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn clamp_nan_min_max() {
    clamp(0., f32::NAN, f32::NAN);
}
