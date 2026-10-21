#[cfg(all(test, target_arch = "wasm64"))]
mod tests {
    use super::WasmWord;

    #[test]
    fn wasm_word_roundtrips_large_pointer_values() {
        let value = 1usize << 60;
        assert_eq!(WasmWord::from_usize(value).into_usize(), value);

        let signed = -(1isize << 40);
        assert_eq!(WasmWord::from_isize(signed).into_isize(), signed);
    }
}
