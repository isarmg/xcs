#[test]
fn clamp_test() {
    // Int test
    assert_eq!(1, clamp(1, -1, 2));
    assert_eq!(-1, clamp(-2, -1, 2));
    assert_eq!(2, clamp(3, -1, 2));
    assert_eq!(1, clamp_min(1, -1));
    assert_eq!(-1, clamp_min(-2, -1));
    assert_eq!(-1, clamp_max(1, -1));
    assert_eq!(-2, clamp_max(-2, -1));

    // Float test
    assert_eq!(1.0, clamp(1.0, -1.0, 2.0));
    assert_eq!(-1.0, clamp(-2.0, -1.0, 2.0));
    assert_eq!(2.0, clamp(3.0, -1.0, 2.0));
    assert_eq!(1.0, clamp_min(1.0, -1.0));
    assert_eq!(-1.0, clamp_min(-2.0, -1.0));
    assert_eq!(-1.0, clamp_max(1.0, -1.0));
    assert_eq!(-2.0, clamp_max(-2.0, -1.0));
    assert!(clamp(f32::NAN, -1.0, 1.0).is_nan());
    assert!(clamp_min(f32::NAN, 1.0).is_nan());
    assert!(clamp_max(f32::NAN, 1.0).is_nan());
}
