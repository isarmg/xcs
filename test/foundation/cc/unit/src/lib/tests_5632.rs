#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_clang_compiler_uses_target_arg_internally() {
        for version in 16..21 {
            assert!(android_clang_compiler_uses_target_arg_internally(
                &PathBuf::from(format!("armv7a-linux-androideabi{}-clang", version))
            ));
            assert!(android_clang_compiler_uses_target_arg_internally(
                &PathBuf::from(format!("armv7a-linux-androideabi{}-clang++", version))
            ));
        }
        assert!(!android_clang_compiler_uses_target_arg_internally(
            &PathBuf::from("clang-i686-linux-android")
        ));
        assert!(!android_clang_compiler_uses_target_arg_internally(
            &PathBuf::from("clang")
        ));
        assert!(!android_clang_compiler_uses_target_arg_internally(
            &PathBuf::from("clang++")
        ));
    }
}
