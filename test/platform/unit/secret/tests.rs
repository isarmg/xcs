use super::*;
#[test]
fn formatting_never_discloses_values() {
    let secret = SecretString::new("needle".into());
    assert_eq!(format!("{secret:?}/{secret}"), "[REDACTED]/[REDACTED]");
    assert!(!format!("{:?}", SecretBytes::new(b"needle".to_vec())).contains("needle"));
    assert_eq!(format!("{:?}", SecretKey::new([7_u8; 32])), "[REDACTED]");
}
