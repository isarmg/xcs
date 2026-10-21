#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn test_uuid_ext() {
        let s1 = "67e5504410b1426f9247bb680e5fe0c8";
        let s2 = "67e55044-10b1-426f-9247-bb680e5fe0c8";

        let u = Uuid::parse(s1).unwrap();

        let a1 = u.format_simple().to_string();
        let a2 = format!("{:X}", u.format_hyphenated());

        assert_eq!(a1, s1);
        assert_eq!(a2, s2.to_ascii_uppercase());
    }
}
