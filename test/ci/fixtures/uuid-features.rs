// First-party consumers of the maintained native/browser UUID integration.
use uuid::{uuid, Uuid, Variant, Version};

#[test]
fn v4_generation_preserves_version_variant_and_roundtrips() {
    const KNOWN: Uuid = uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8");
    assert_eq!(KNOWN.get_version(), Some(Version::Random));
    for _ in 0..64 {
        let id = Uuid::new_v4();
        assert_eq!(id.get_version(), Some(Version::Random));
        assert_eq!(id.get_variant(), Variant::RFC4122);
        assert_eq!(Uuid::parse_str(&id.hyphenated().to_string()).unwrap(), id);
        assert_eq!(Uuid::parse_str(&id.simple().to_string()).unwrap(), id);
        assert_eq!(Uuid::from_slice(id.as_bytes()).unwrap(), id);
    }
}
