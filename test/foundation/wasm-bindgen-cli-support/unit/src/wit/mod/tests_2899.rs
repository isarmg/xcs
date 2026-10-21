#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_packer() {
        let mut unpacker = StructUnpacker::new();
        let i32___ = &AdapterType::I32;
        let double = &AdapterType::F64;
        let mut read_ty = |ty| unpacker.read_ty(ty).unwrap();
        assert_eq!(read_ty(i32___), 0); // u32
        assert_eq!(read_ty(i32___), 1); // u32
        assert_eq!(read_ty(double), 2); // f64, already aligned
        assert_eq!(read_ty(i32___), 4); // u32, already aligned
        assert_eq!(read_ty(double), 6); // f64, NOT already aligned, skips up to offset 6
    }

    #[test]
    fn normalize_memory64_descriptor_only_rewrites_explicit_number_abi_options() {
        let mut number_option = Descriptor::Option(Box::new(Descriptor::I64));
        Context::normalize_memory64_descriptor(&mut number_option, walrus::ValType::F64);
        assert_eq!(
            number_option,
            Descriptor::Option(Box::new(Descriptor::I64AsF64))
        );

        let mut exact_i64 = Descriptor::I64;
        Context::normalize_memory64_descriptor(&mut exact_i64, walrus::ValType::I64);
        assert_eq!(exact_i64, Descriptor::I64);

        let mut exact_option = Descriptor::Option(Box::new(Descriptor::I64));
        Context::normalize_memory64_descriptor(&mut exact_option, walrus::ValType::I64);
        assert_eq!(exact_option, Descriptor::Option(Box::new(Descriptor::I64)));
    }

    /// Build a minimal module containing a 32-bit or 64-bit memory and an exported
    /// C `main` shim with the given signature `(i32 argc, argv) -> i32`.
    fn make_main_module(memory64: bool, argv_ty: walrus::ValType) -> (Module, MemoryId) {
        let mut config = walrus::ModuleConfig::new();
        config.generate_producers_section(false);
        let mut module = walrus::Module::with_config(config);

        let mem = module.memories.add_local(false, memory64, 1, None, None);

        // Build `main(i32 argc, argv) -> i32`.
        let argc = module.locals.add(walrus::ValType::I32);
        let argv = module.locals.add(argv_ty);
        let mut builder = walrus::FunctionBuilder::new(
            &mut module.types,
            &[walrus::ValType::I32, argv_ty],
            &[walrus::ValType::I32],
        );
        builder.func_body().i32_const(0);
        let main_id = builder.finish(vec![argc, argv], &mut module.funcs);
        module.exports.add("main", main_id);

        (module, mem)
    }

    fn run_discover_main(module: &mut Module, memory: MemoryId) -> bool {
        let mut cx = Context {
            start_found: false,
            module,
            adapters: Default::default(),
            aux: Default::default(),
            function_exports: Default::default(),
            function_imports: Default::default(),
            memory: Some(memory),
            vendor_prefixes: Default::default(),
            unique_crate_identifier: "",
            descriptors: Default::default(),
            externref_enabled: false,
            thread_count: None,
            support_start: true,
            linked_modules: false,
            export_adapter_sigs: Default::default(),
            generic_import_bindings: Default::default(),
            pending_generic_imports: Default::default(),
            generic_import_display_names: Default::default(),
            item_identities: Default::default(),
            ambiguous_items: Default::default(),
            demangled_exports: Default::default(),
        };
        cx.discover_main().unwrap();
        cx.start_found
    }

    /// Collect argv and argc constants.
    fn start_fn_consts(module: &Module) -> Vec<walrus::ir::Value> {
        let start = module
            .start
            .expect("discover_main should register a start function");
        let func = module.funcs.get(start);
        let walrus::FunctionKind::Local(local) = &func.kind else {
            panic!("start wrapper should be a local function");
        };
        local
            .block(local.entry_block())
            .instrs
            .iter()
            .filter_map(|(instr, _)| match instr {
                walrus::ir::Instr::Const(c) => Some(c.value),
                _ => None,
            })
            .collect()
    }

    /// On wasm64 the C `main` shim is `main(i32 argc, i64 argv) -> i32`
    #[test]
    fn discover_main_handles_wasm64_argv_pointer() {
        let (mut module, mem) = make_main_module(true, walrus::ValType::I64);
        assert!(
            run_discover_main(&mut module, mem),
            "wasm64 `main(i32, i64) -> i32` should be discovered as the entry point"
        );

        assert!(
            matches!(
                start_fn_consts(&module).as_slice(),
                [walrus::ir::Value::I32(0), walrus::ir::Value::I64(0)]
            ),
            "wasm64 start wrapper should pass (i32 argc, i64 argv) = (0, 0)"
        );
    }

    /// On wasm32 the C `main` shim is `main(i32 argc, i32 argv) -> i32`
    #[test]
    fn discover_main_handles_wasm32_argv_pointer() {
        let (mut module, mem) = make_main_module(false, walrus::ValType::I32);
        assert!(
            run_discover_main(&mut module, mem),
            "wasm32 `main(i32, i32) -> i32` should be discovered as the entry point"
        );
        assert!(
            matches!(
                start_fn_consts(&module).as_slice(),
                [walrus::ir::Value::I32(0), walrus::ir::Value::I32(0)]
            ),
            "wasm32 start wrapper should pass (i32 argc, i32 argv) = (0, 0)"
        );
    }

    /// Don't mistake a `main(i32, i32) -> i32` as the entrypoint on wasm64
    #[test]
    fn discover_main_ignores_wrong_argv_width_on_wasm64() {
        let (mut module, mem) = make_main_module(true, walrus::ValType::I32);
        assert!(
            !run_discover_main(&mut module, mem),
            "on wasm64 a 32-bit-argv `main` must not be treated as the entry point"
        );
    }
}
