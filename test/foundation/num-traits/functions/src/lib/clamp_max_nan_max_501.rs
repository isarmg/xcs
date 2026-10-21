#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn clamp_max_nan_max() {
    clamp_max(0., f32::NAN);
}
