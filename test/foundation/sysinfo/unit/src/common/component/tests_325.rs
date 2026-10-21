#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_components_mac_m1() {
        let mut components = Components::new();
        components.refresh(false);
        components.refresh(false);
    }
}
