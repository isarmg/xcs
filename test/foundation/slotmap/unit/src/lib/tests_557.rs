#[cfg(test)]
mod tests {
    // Intentionally no `use super::*;` because we want to test macro expansion
    // in the *users* scope, which might not have that.
    #[test]
    fn macro_expansion() {
        #![allow(dead_code)]

        // Clobber namespace with clashing names - should still work.
        trait Serialize {}
        trait Deserialize {}
        trait Serializer {}
        trait Deserializer {}
        trait Key {}
        trait From {}
        struct Result;
        struct KeyData;

        new_key_type! {
            struct A;
            pub(crate) struct B;
            pub struct C;
        }
    }

    #[test]
    fn check_is_older_version() {
        use super::util::is_older_version;

        let is_older = |a, b| is_older_version(a, b);
        assert!(!is_older(42, 42));
        assert!(is_older(0, 1));
        assert!(is_older(0, 1 << 31));
        assert!(!is_older(0, (1 << 31) + 1));
        assert!(is_older(u32::MAX, 0));
    }

    #[test]
    fn iters_cloneable() {
        use super::*;

        struct NoClone;

        let mut sm = SlotMap::new();
        #[allow(deprecated)]
        let mut hsm = HopSlotMap::new();
        let mut dsm = DenseSlotMap::new();
        let mut scm = SecondaryMap::new();
        let mut sscm = SparseSecondaryMap::new();
        scm.insert(sm.insert(NoClone), NoClone);
        sscm.insert(hsm.insert(NoClone), NoClone);
        dsm.insert(NoClone);

        let _ = sm.keys().clone();
        let _ = sm.values().clone();
        let _ = sm.iter().clone();
        let _ = hsm.keys().clone();
        let _ = hsm.values().clone();
        let _ = hsm.iter().clone();
        let _ = dsm.keys().clone();
        let _ = dsm.values().clone();
        let _ = dsm.iter().clone();
        let _ = scm.keys().clone();
        let _ = scm.values().clone();
        let _ = scm.iter().clone();
        let _ = sscm.keys().clone();
        let _ = sscm.values().clone();
        let _ = sscm.iter().clone();
    }

    #[cfg(feature = "serde")]
    #[test]
    fn key_serde() {
        use super::*;

        // Check round-trip through serde.
        let mut sm = SlotMap::new();
        let k = sm.insert(42);
        let ser = serde_json::to_string(&k).unwrap();
        let de: DefaultKey = serde_json::from_str(&ser).unwrap();
        assert_eq!(k, de);

        // Even if a malicious entity sends up even (unoccupied) versions in the
        // key, we make the version point to the occupied version.
        let malicious: KeyData = serde_json::from_str(&r#"{"idx":0,"version":4}"#).unwrap();
        assert_eq!(malicious.version.get(), 5);
    }
}
