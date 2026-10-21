#[test]
#[should_panic]
#[cfg(debug_assertions)]
fn clamp_nan_max() {
    clamp(0., -1., f32::NAN);
}
