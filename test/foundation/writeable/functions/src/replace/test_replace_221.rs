#[test]
fn test_replace() {
    use crate::assert_writeable_eq;
    use crate::concat::Concat;

    // Basic replacement
    let replace1 = Replace {
        source: Concat("Hello", " 10 22 1101 33"),
        needle: "10",
        replacement: Concat("4", "4"),
    };
    assert_writeable_eq!(replace1, "Hello 44 22 1441 33");

    // Empty needle (should just write source)
    let replace2 = Replace {
        source: "Hello World",
        needle: "",
        replacement: "X",
    };
    assert_writeable_eq!(replace2, "Hello World");

    // Empty replacement
    let replace3 = Replace {
        source: "Hello 10 World 10",
        needle: "10",
        replacement: "",
    };
    assert_writeable_eq!(replace3, "Hello  World ");

    // Needle not found
    let replace4 = Replace {
        source: "Hello World",
        needle: "10",
        replacement: "X",
    };
    assert_writeable_eq!(replace4, "Hello World");

    // Needle at the beginning
    let replace5 = Replace {
        source: "10 Hello World",
        needle: "10",
        replacement: "X",
    };
    assert_writeable_eq!(replace5, "X Hello World");

    // Needle at the end
    let replace6 = Replace {
        source: "Hello World 10",
        needle: "10",
        replacement: "X",
    };
    assert_writeable_eq!(replace6, "Hello World X");

    // Overlapping needles (should consume and not match again)
    let replace7 = Replace {
        source: "ababa",
        needle: "aba",
        replacement: "X",
    };
    assert_writeable_eq!(replace7, "Xba");

    // Self-overlap but no match
    let replace8 = Replace {
        source: "aab",
        needle: "aac",
        replacement: "X",
    };
    assert_writeable_eq!(replace8, "aab");

    // Multi-byte UTF-8
    let replace9 = Replace {
        source: "🚀 🛸 🚀🚀 🚁",
        needle: "🚀",
        replacement: "星",
    };
    assert_writeable_eq!(replace9, "星 🛸 星星 🚁");

    // Multi-byte UTF-8 with partial match
    let replace10 = Replace {
        source: "🚀🚁",
        needle: "🚀🛸",
        replacement: "星",
    };
    assert_writeable_eq!(replace10, "🚀🚁");

    // Multi-byte UTF-8 with backtracking (no match)
    let replace11 = Replace {
        source: "🚀🚀🚁",
        needle: "🚀🚀🛸",
        replacement: "星",
    };
    assert_writeable_eq!(replace11, "🚀🚀🚁");

    // Multi-byte UTF-8 with backtracking (match)
    let replace12 = Replace {
        source: "🚀🚀🚀🛸",
        needle: "🚀🚀🛸",
        replacement: "星",
    };
    assert_writeable_eq!(replace12, "🚀星");
}
