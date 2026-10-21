#[cfg(test)]
#[allow(unexpected_cfgs)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use std::process::Command;

    use super::TargetInfo;
    use crate::ErrorKind;

    // Test tier 1 targets.
    #[test]
    fn tier1() {
        let targets = [
            "aarch64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "i686-pc-windows-gnu",
            "i686-pc-windows-msvc",
            "i686-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "x86_64-pc-windows-gnu",
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
        ];

        for target in targets {
            // Check that they parse.
            let _ = TargetInfo::from_rustc_target(target).unwrap();
        }
    }

    // Various custom target names not (or no longer) known by `rustc`.
    #[test]
    fn parse_extra() {
        let targets = [
            "aarch64-unknown-none-gnu",
            "aarch64-uwp-windows-gnu",
            "arm-frc-linux-gnueabi",
            "arm-unknown-netbsd-eabi",
            "armv7neon-unknown-linux-gnueabihf",
            "armv7neon-unknown-linux-musleabihf",
            "thumbv7-unknown-linux-gnueabihf",
            "thumbv7-unknown-linux-musleabihf",
            "armv7-apple-ios",
            "wasm32-wasi",
            "x86_64-rumprun-netbsd",
            "x86_64-unknown-linux",
            "x86_64-alpine-linux-musl",
            "x86_64-chimera-linux-musl",
            "x86_64-foxkit-linux-musl",
            "arm-poky-linux-gnueabi",
            "x86_64-unknown-moturus",
            "x86_64-unknown-managarm-mlibc",
        ];

        for target in targets {
            // Check that they parse.
            let _ = TargetInfo::from_rustc_target(target).unwrap();
        }
    }

    fn target_from_rustc_cfgs<'a>(target: &'a str, cfgs: &'a str) -> TargetInfo<'a> {
        // Cannot determine full architecture from cfgs.
        let (full_arch, _rest) = target.split_once('-').expect("target to have arch");

        let mut target = TargetInfo {
            full_arch,
            arch: "invalid-none-set",
            vendor: "invalid-none-set",
            os: "invalid-none-set",
            env: "invalid-none-set",
            // Not set in older Rust versions
            abi: "",
        };

        for cfg in cfgs.lines() {
            if let Some((name, value)) = cfg.split_once('=') {
                // Remove whitespace, if `rustc` decided to insert any.
                let name = name.trim();
                let value = value.trim();

                // Remove quotes around value.
                let value = value.strip_prefix('"').unwrap_or(value);
                let value = value.strip_suffix('"').unwrap_or(value);

                match name {
                    "target_arch" => target.arch = value,
                    "target_vendor" => target.vendor = value,
                    "target_os" => target.os = value,
                    "target_env" => target.env = value,
                    "target_abi" => target.abi = value,
                    _ => {}
                }
            } else {
                // Skip cfgs like `debug_assertions` and `unix`.
            }
        }

        if matches!(target.abi, "macabi" | "sim") {
            assert_eq!(target.env, target.abi);
            target.abi = "";
        }

        target
    }

    #[test]
    fn unknown_env_determined_as_unknown() {
        let err = TargetInfo::from_rustc_target("aarch64-unknown-linux-bogus").unwrap_err();
        assert!(matches!(err.kind, ErrorKind::UnknownTarget));
    }

    // Used in .github/workflows/test-rustc-targets.yml
    #[test]
    #[cfg_attr(
        not(rustc_target_test),
        ignore = "must enable explicitly with --cfg=rustc_target_test"
    )]
    fn parse_rustc_targets() {
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
            let cfgs = Command::new(&rustc)
                .arg("--target")
                .arg(target)
                .arg("--print=cfg")
                .output()
                .unwrap()
                .stdout;
            let cfgs = String::from_utf8(cfgs).unwrap();

            let expected = target_from_rustc_cfgs(target, &cfgs);
            let actual = TargetInfo::from_rustc_target(target);

            if Some(&expected) != actual.as_ref().ok() {
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

    #[test]
    fn parses_apple_envs_correctly() {
        assert_eq!(
            TargetInfo::from_rustc_target("aarch64-apple-ios-macabi").unwrap(),
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "apple",
                os: "ios",
                env: "macabi",
                abi: "",
            }
        );
        assert_eq!(
            TargetInfo::from_rustc_target("aarch64-apple-ios-sim").unwrap(),
            TargetInfo {
                full_arch: "aarch64",
                arch: "aarch64",
                vendor: "apple",
                os: "ios",
                env: "sim",
                abi: "",
            }
        );
    }
}
