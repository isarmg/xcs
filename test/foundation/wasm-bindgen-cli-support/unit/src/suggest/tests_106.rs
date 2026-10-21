#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_basic() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("a", ""), 1);
        assert_eq!(levenshtein("", "a"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("Point", "Pont"), 1);
        assert_eq!(levenshtein("Point", "Pont2"), 2);
    }

    #[test]
    fn threshold_short_inputs_strict() {
        // 2-char input: threshold 0 so nothing fuzzy-matches; protects
        // against `a` suggesting `b` etc.
        let s = suggest("ab", ["ax", "ay", "az"]);
        assert!(s.is_empty(), "got: {s:?}");
    }

    #[test]
    fn threshold_longer_inputs_tolerant() {
        let s = suggest("Pont", ["Point", "Status", "Sphere"]);
        assert!(s.contains("Point"), "got: {s:?}");
        assert!(!s.contains("Status"), "got: {s:?}");
    }

    #[test]
    fn caps_at_max_suggestions() {
        let s = suggest("Foo", ["Foa", "Fob", "Foc", "Fod", "Foe"]);
        let count = s.matches("  - ").count();
        assert!(count <= MAX_SUGGESTIONS, "got: {s:?}");
    }

    #[test]
    fn excludes_exact_match() {
        // Exact match isn't a "suggestion" — caller should be using the
        // exact match directly. This helper is only invoked when lookup
        // already failed.
        let s = suggest("Foo", ["Foo", "Foa"]);
        assert!(!s.contains("`Foo`"), "got: {s:?}");
        assert!(s.contains("Foa"), "got: {s:?}");
    }

    #[test]
    fn empty_when_no_match() {
        let s = suggest("Banana", ["Carburetor", "Pneumatic", "Xylophone"]);
        assert!(s.is_empty(), "got: {s:?}");
    }

    #[test]
    fn single_vs_plural_phrasing() {
        let single = suggest("Pont", ["Point", "Sphere"]);
        assert!(single.contains("did you mean `Point`?"), "got: {single:?}");
        let multi = suggest("Pont", ["Point", "Pent"]);
        assert!(multi.contains("one of these"), "got: {multi:?}");
    }
}
