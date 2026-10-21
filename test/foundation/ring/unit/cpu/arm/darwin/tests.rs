#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu;

    #[test]
    fn sha512_detection() {
        // We intentionally disable static feature detection for SHA-512.
        const _SHA512_NOT_STATICALLY_DETECTED: () = assert!((CAPS_STATIC & Sha512::mask()) == 0);

        if cfg!(target_os = "macos") {
            use crate::cpu::{arm::Sha512, GetFeature as _};

            // All aarch64-apple-darwin targets have SHA3 enabled statically...
            assert!(cfg!(target_feature = "sha3"));

            // ...so we should detect it.
            let cpu = cpu::features();
            assert!(matches!(cpu.get_feature(), Some(Sha512 { .. })));
        }
    }
}
