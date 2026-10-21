#[cfg(test)]
mod tests {
    use super::IgnoreAsciiCase;

    #[test]
    fn ignore_ascii_case() {
        assert!("x86_64-w64-mingw32-CLANG".contains_ignore_ascii_case("mingw32-clang"));
        assert!("Clang-CL.exe".contains_ignore_ascii_case("clang-cl"));
        assert!("zig".contains_ignore_ascii_case(""));
        assert!(!"zi".contains_ignore_ascii_case("zig"));
        assert!(!"clang".contains_ignore_ascii_case("clang-cl"));

        assert!("LLVM-ML64".starts_with_ignore_ascii_case("llvm-ml"));
        assert!(!"llvm-m".starts_with_ignore_ascii_case("llvm-ml"));
        assert!(!"my-llvm-ml".starts_with_ignore_ascii_case("llvm-ml"));

        assert!("x86_64-w64-mingw32-Clang++".ends_with_ignore_ascii_case("-mingw32-clang++"));
        assert!("CL".ends_with_ignore_ascii_case("cl"));
        assert!(!"l".ends_with_ignore_ascii_case("cl"));
        assert!(!"cl.exe".ends_with_ignore_ascii_case("cl"));

        // Only ASCII letters are folded, and other characters still compare.
        assert!("Ünïcode-CLANG".contains_ignore_ascii_case("Ünïcode-clang"));
        assert!(!"ÜNÏCODE-clang".contains_ignore_ascii_case("ünïcode-clang"));
        assert!(!"\u{212A}ache".starts_with_ignore_ascii_case("kache"));
    }
}
