#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "std")]
    fn test_aarch64_registers() {
        use super::*;
        use std::collections::HashSet;

        let mut names = HashSet::new();
        for n in (0..=39).chain(46..=127) {
            let name = AArch64::register_name(Register(n))
                .unwrap_or_else(|| panic!("Register {} should have a name.", n));
            assert!(names.insert(name));
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_power64_registers() {
        use super::*;
        use std::collections::HashSet;

        let mut names = HashSet::new();
        for n in (0..=63).chain(68..=75).chain(77..=108) {
            let name = PowerPc64::register_name(Register(n))
                .unwrap_or_else(|| panic!("Register {} should have a name.", n));
            assert!(names.insert(name));
        }
    }
}
