#[cfg(test)]
mod tests {
    // This should always pass on any x86 system except very, very, old ones.
    #[cfg(target_arch = "x86")]
    #[test]
    fn x86_has_sse2() {
        use super::*;
        use crate::cpu::{self, GetFeature as _};
        assert!(matches!(cpu::features().get_feature(), Some(Sse2 { .. })))
    }
}
