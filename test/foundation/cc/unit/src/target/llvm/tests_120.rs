#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use std::process::Command;

    use crate::TargetInfo;

    #[test]
    fn test_old_ios_target() {
        assert_eq!(
            TargetInfo {
                full_arch: "armv7",
                arch: "armv7",
                vendor: "apple",
                os: "ios",
                env: "",
                abi: "",
            }
            .llvm_target("armv7-apple-ios", None),
            "armv7-apple-ios"
        );
    }

    #[test]
    fn basic_llvm_triple_guessing() {
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "unknown",
                os: "linux",
                env: "",
                abi: "",
            }
            .llvm_target("invalid", None),
            "aarch64-unknown-linux"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "unknown",
                os: "linux",
                env: "gnu",
                abi: "",
            }
            .llvm_target("invalid", None),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "unknown",
                os: "linux",
                env: "gnu",
                abi: "eabi",
            }
            .llvm_target("invalid", None),
            "x86_64-unknown-linux-gnueabi"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "apple",
                os: "macos",
                env: "",
                abi: "",
            }
            .llvm_target("invalid", None),
            "x86_64-apple-macosx"
        );
    }

    #[test]
    fn llvm_version() {
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "apple",
                os: "ios",
                env: "sim",
                abi: "",
            }
            .llvm_target("aarch64-apple-ios-sim", Some("14.0")),
            "arm64-apple-ios14.0-simulator"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "apple",
                os: "visionos",
                env: "",
                abi: "",
            }
            .llvm_target("aarch64-apple-visionos", Some("2.0")),
            "arm64-apple-xros2.0"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "apple",
                os: "ios",
                env: "",
                abi: "macabi",
            }
            .llvm_target("aarch64-apple-ios-macabi", Some("13.1")),
            "arm64-apple-ios13.1-macabi"
        );
    }

    #[test]
    fn uefi() {
        assert_eq!(
            TargetInfo {
                full_arch: "i686",
                arch: "x86",
                vendor: "unknown",
                os: "uefi",
                env: "",
                abi: "",
            }
            .llvm_target("i686-unknown-uefi", None),
            "i686-unknown-windows-gnu"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "x86_64",
                arch: "x86_64",
                vendor: "unknown",
                os: "uefi",
                env: "",
                abi: "",
            }
            .llvm_target("x86_64-unknown-uefi", None),
            "x86_64-unknown-windows-gnu"
        );
        assert_eq!(
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "unknown",
                os: "uefi",
                env: "",
                abi: "",
            }
            .llvm_target("aarch64-unknown-uefi", None),
            "aarch64-unknown-windows-gnu"
        );
    }

    #[test]
    #[ignore = "not yet done"]
    fn llvm_for_all_rustc_targets() {
        #[allow(clippy::disallowed_methods)]
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());

        let target_list = Command::new(&rustc)
            .arg("--print=target-list")
            .output()
            .unwrap()
            .stdout;
        let target_list = String::from_utf8(target_list).unwrap();

        let mut has_failure = false;
        for target in target_list.lines() {
            let spec_json = Command::new(&rustc)
                .arg("--target")
                .arg(target)
                .arg("-Zunstable-options")
                .arg("--print=target-spec-json")
                .env("RUSTC_BOOTSTRAP", "1") // Crimes
                .output()
                .unwrap()
                .stdout;
            let spec_json = String::from_utf8(spec_json).unwrap();

            // JSON crimes
            let expected = spec_json
                .split_once("llvm-target\": \"")
                .unwrap()
                .1
                .split_once("\"")
                .unwrap()
                .0;
            let actual = TargetInfo::from_rustc_target(target)
                .map(|target| target.llvm_target("invalid", None));

            if Some(expected) != actual.as_deref().ok() {
                eprintln!("failed comparing {target}:");
                eprintln!("  expected: Ok({expected:?})");
                eprintln!("    actual: {actual:?}");
                eprintln!();
                has_failure = true;
            }
        }

        if has_failure {
            panic!("failed comparing targets");
        }
    }
}
