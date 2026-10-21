use super::*;

#[test]
fn browser_preflight_accepts_the_limit_and_rejects_unbounded_buffering() {
    assert!(browser_size(0.0).is_ok());
    assert!(browser_size(64.0 * 1024.0 * 1024.0).is_ok());
    assert!(browser_size(64.0 * 1024.0 * 1024.0 + 1.0).is_err());
    for size in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(browser_size(size).is_err());
    }
}
