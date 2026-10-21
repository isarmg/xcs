#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_is_subset_of_dynamic() {
        let cpu = features();
        let dynamic = featureflags::get(cpu);
        assert_eq!(dynamic & CAPS_STATIC, CAPS_STATIC);
    }
}
