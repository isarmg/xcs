#[cfg(test)]
mod tests {
    use super::*;
    use crate::Module;
    use wasmparser::WasmFeatures;

    #[test]
    fn func_types_dont_require_wasm_gc() {
        let mut types = TypeSection::new();
        types.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Func(FuncType::new([], [])),
                shared: false,
                descriptor: None,
                describes: None,
            },
        });

        let mut module = Module::new();
        module.section(&types);
        let wasm_bytes = module.finish();

        let mut validator =
            wasmparser::Validator::new_with_features(WasmFeatures::default() & !WasmFeatures::GC);

        validator.validate_all(&wasm_bytes).expect(
            "Encoding pre Wasm GC type should not accidentally use Wasm GC specific encoding",
        );
    }
}
