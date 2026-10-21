#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::parse_wat;
    use crate::wit::AdapterId;

    /// Build a `wit` section claiming the named exports as adapter exports.
    fn wit_claiming(module: &walrus::Module, names: &[&str]) -> NonstandardWitSection {
        let mut wit = NonstandardWitSection::default();
        for (i, name) in names.iter().enumerate() {
            let export = module
                .exports
                .iter()
                .find(|e| e.name == *name)
                .unwrap_or_else(|| panic!("no export named `{name}`"));
            wit.exports.push((export.id(), AdapterId(i)));
        }
        wit
    }

    fn func_of(module: &walrus::Module, name: &str) -> FunctionId {
        match module.exports.iter().find(|e| e.name == name).unwrap().item {
            ExportItem::Function(f) => f,
            _ => panic!("`{name}` is not a function export"),
        }
    }

    /// `default()` already covers exceptions and memory64; legacy EH does not.
    fn validate(module: &mut walrus::Module, extra: wasmparser::WasmFeatures) {
        let features = wasmparser::WasmFeatures::default() | extra;
        wasmparser::Validator::new_with_features(features)
            .validate_all(&module.emit_wasm())
            .expect("transformed module should validate");
    }

    /// Pins the two halves of the fix in both dialects: the wrapper saves
    /// `__stack_pointer` before it calls anything, and every rethrow writes
    /// that same saved value back first. A wrapper doing neither still
    /// validates and still repoints the export, so nothing else here notices.
    fn assert_saves_and_restores(module: &walrus::Module, func: FunctionId, sp: GlobalId) {
        let FunctionKind::Local(local) = &module.funcs.get(func).kind else {
            panic!("wrapper should be a local function");
        };

        let saved_sp = match local.block(local.entry_block()).instrs.as_slice() {
            [(Instr::GlobalGet(g), _), (Instr::LocalSet(l), _), ..] if g.global == sp => l.local,
            _ => panic!("wrapper should open by saving `__stack_pointer` into a local"),
        };

        struct Scan {
            sp: GlobalId,
            saved_sp: LocalId,
            rethrows: usize,
        }

        impl<'instr> Visitor<'instr> for Scan {
            fn start_instr_seq(&mut self, seq: &'instr InstrSeq) {
                for (i, (instr, _)) in seq.instrs.iter().enumerate() {
                    if !matches!(instr, Instr::ThrowRef(_) | Instr::Rethrow(_)) {
                        continue;
                    }
                    self.rethrows += 1;
                    assert!(
                        i >= 2
                            && matches!(
                                seq.instrs[i - 2].0,
                                Instr::LocalGet(LocalGet { local }) if local == self.saved_sp
                            )
                            && matches!(
                                seq.instrs[i - 1].0,
                                Instr::GlobalSet(GlobalSet { global }) if global == self.sp
                            ),
                        "rethrow is not preceded by `local.get $saved_sp` + \
                         `global.set $__stack_pointer`"
                    );
                }
            }
        }

        let mut scan = Scan {
            sp,
            saved_sp,
            rethrows: 0,
        };
        dfs_in_order(&mut scan, local, local.entry_block());
        assert_eq!(scan.rethrows, 1, "wrapper should rethrow exactly once");
    }

    fn stack_pointer(module: &walrus::Module) -> GlobalId {
        crate::wasm_conventions::get_stack_pointer(module).unwrap()
    }

    /// A module with a shadow stack, an adapter-backed export and an intrinsic
    /// export that must be left alone.
    const MODULE: &str = r#"
        (module
            (global $__stack_pointer (mut i32) (i32.const 1048576))
            (func $shim (param i32) (result i32) local.get 0)
            (func $__wbindgen_malloc (param i32) (result i32) local.get 0)
            (export "shim" (func $shim))
            (export "__wbindgen_malloc" (func $__wbindgen_malloc))
        )
    "#;

    #[test]
    fn wraps_adapter_exports_and_leaves_intrinsics_alone() {
        let mut module = parse_wat(MODULE);
        let shim = func_of(&module, "shim");
        let malloc = func_of(&module, "__wbindgen_malloc");
        let wit = wit_claiming(&module, &["shim"]);

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        let wrapped = func_of(&module, "shim");
        assert_ne!(wrapped, shim);
        assert_eq!(
            module.funcs.get(wrapped).name.as_deref(),
            Some("shim sp wrapper")
        );
        assert_eq!(module.funcs.get(wrapped).ty(), module.funcs.get(shim).ty());
        assert_saves_and_restores(&module, wrapped, stack_pointer(&module));

        // Still reachable by its original `FunctionId`, and still exported once.
        assert_eq!(func_of(&module, "__wbindgen_malloc"), malloc);
        assert_eq!(
            module
                .exports
                .iter()
                .filter(|e| e.name == "__wbindgen_malloc")
                .count(),
            1
        );

        validate(&mut module, wasmparser::WasmFeatures::empty());
    }

    /// `__wbg_*_free` and `__wbg_upcast_*` are `extern "C-unwind"` class
    /// exports resolved by name, with no adapter in `wit.exports`. `free`
    /// panics recoverably when the value is still borrowed, so an unwrapped
    /// export leaks a frame per caught panic.
    #[test]
    fn wraps_name_resolved_c_unwind_exports() {
        let wat = r#"
            (module
                (global $__stack_pointer (mut i32) (i32.const 1048576))
                (func $free (param i32 i32))
                (func $upcast (param i32) (result i32) local.get 0)
                (func $__wbindgen_malloc (param i32) (result i32) local.get 0)
                (export "__wbg_widget_free" (func $free))
                (export "__wbg_upcast_child_to_parent" (func $upcast))
                (export "__wbindgen_malloc" (func $__wbindgen_malloc))
            )
        "#;
        let mut module = parse_wat(wat);
        let free = func_of(&module, "__wbg_widget_free");
        let upcast = func_of(&module, "__wbg_upcast_child_to_parent");
        let malloc = func_of(&module, "__wbindgen_malloc");
        let wit = NonstandardWitSection::default();

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        let sp = stack_pointer(&module);
        for (name, original) in [
            ("__wbg_widget_free", free),
            ("__wbg_upcast_child_to_parent", upcast),
        ] {
            let wrapped = func_of(&module, name);
            assert_ne!(wrapped, original, "{name}");
            assert_saves_and_restores(&module, wrapped, sp);
        }

        // Plain `extern "C"` intrinsics abort on an escaping panic and stay
        // resolvable by `FunctionId`.
        assert_eq!(func_of(&module, "__wbindgen_malloc"), malloc);

        validate(&mut module, wasmparser::WasmFeatures::empty());
    }

    #[test]
    fn legacy_eh_wrapper_restores_and_validates() {
        let mut module = parse_wat(MODULE);
        let wit = wit_claiming(&module, &["shim"]);

        run(&mut module, &wit, ExceptionHandlingVersion::Legacy);

        let wrapped = func_of(&module, "shim");
        assert_saves_and_restores(&module, wrapped, stack_pointer(&module));

        validate(&mut module, wasmparser::WasmFeatures::LEGACY_EXCEPTIONS);
    }

    /// The `__stack_pointer` global is `i64` on memory64, so the scratch local
    /// has to follow it or the wrapper fails validation.
    #[test]
    fn memory64_stack_pointer_uses_an_i64_local() {
        let wat = r#"
            (module
                (memory i64 1)
                (global $__stack_pointer (mut i64) (i64.const 1048576))
                (func $shim (param i32) (result i32) local.get 0)
                (export "shim" (func $shim))
            )
        "#;
        let mut module = parse_wat(wat);
        let wit = wit_claiming(&module, &["shim"]);
        assert_eq!(
            module
                .locals
                .iter()
                .filter(|l| l.ty() == ValType::I64)
                .count(),
            0
        );

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        assert_eq!(
            module
                .locals
                .iter()
                .filter(|l| l.ty() == ValType::I64)
                .count(),
            1,
            "expected exactly one i64 scratch local for the stack pointer"
        );

        validate(&mut module, wasmparser::WasmFeatures::empty());
    }

    /// One function exported under two names gets one shared wrapper.
    #[test]
    fn shared_function_gets_one_wrapper() {
        let wat = r#"
            (module
                (global $__stack_pointer (mut i32) (i32.const 1048576))
                (func $shim (param i32) (result i32) local.get 0)
                (export "a" (func $shim))
                (export "b" (func $shim))
            )
        "#;
        let mut module = parse_wat(wat);
        let before = module.funcs.iter().count();
        let wit = wit_claiming(&module, &["a", "b"]);

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        assert_eq!(func_of(&module, "a"), func_of(&module, "b"));
        assert_eq!(module.funcs.iter().count(), before + 1);

        validate(&mut module, wasmparser::WasmFeatures::empty());
    }

    #[test]
    fn no_wrappers_without_unwinding() {
        for eh_version in [
            ExceptionHandlingVersion::None,
            ExceptionHandlingVersion::ModernButWithPanicAbort,
        ] {
            let mut module = parse_wat(MODULE);
            let shim = func_of(&module, "shim");
            let wit = wit_claiming(&module, &["shim"]);

            run(&mut module, &wit, eh_version);

            assert_eq!(func_of(&module, "shim"), shim, "{eh_version:?}");
        }
    }

    /// `global.set` on an immutable global would not validate, so the wrapper
    /// has to be skipped rather than emitted.
    #[test]
    fn no_wrappers_for_an_immutable_stack_pointer() {
        let wat = r#"
            (module
                (global $__stack_pointer i32 (i32.const 1048576))
                (func $shim (param i32) (result i32) local.get 0)
                (export "shim" (func $shim))
            )
        "#;
        let mut module = parse_wat(wat);
        let shim = func_of(&module, "shim");
        let wit = wit_claiming(&module, &["shim"]);

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        assert_eq!(func_of(&module, "shim"), shim);
    }

    #[test]
    fn no_wrappers_without_a_shadow_stack() {
        let wat = r#"
            (module
                (func $shim (param i32) (result i32) local.get 0)
                (export "shim" (func $shim))
            )
        "#;
        let mut module = parse_wat(wat);
        let shim = func_of(&module, "shim");
        let wit = wit_claiming(&module, &["shim"]);

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        assert_eq!(func_of(&module, "shim"), shim);
    }

    /// A re-exported import has no frame of its own, so it is left alone.
    #[test]
    fn no_wrapper_for_an_imported_function() {
        let wat = r#"
            (module
                (import "env" "imported" (func $imported (param i32) (result i32)))
                (global $__stack_pointer (mut i32) (i32.const 1048576))
                (export "imported" (func $imported))
            )
        "#;
        let mut module = parse_wat(wat);
        let imported = func_of(&module, "imported");
        let before = module.funcs.iter().count();
        let wit = wit_claiming(&module, &["imported"]);

        run(&mut module, &wit, ExceptionHandlingVersion::Modern);

        assert_eq!(func_of(&module, "imported"), imported);
        assert_eq!(module.funcs.iter().count(), before);
    }
}
