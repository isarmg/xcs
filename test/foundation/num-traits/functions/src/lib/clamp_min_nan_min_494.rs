#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn clamp_min_nan_min() {
    clamp_min(0., f32::NAN);
}
