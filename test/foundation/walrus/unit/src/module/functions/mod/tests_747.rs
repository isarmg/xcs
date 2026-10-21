#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Export, FunctionBuilder, Module};

    #[test]
    fn get_memory_id() {
        let mut module = Module::default();
        let expected_id = module.memories.add_local(false, false, 0, None, None);
        assert!(module.get_memory_id().is_ok_and(|id| id == expected_id));
    }

    /// Running `replace_exported_func` with a closure that builds
    /// a function should replace the existing function with the new one
    #[test]
    fn replace_exported_func() {
        let mut module = Module::default();

        // Create original function
        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let original_fn_id: FunctionId = builder.finish(vec![], &mut module.funcs);
        let original_export_id = module.exports.add("dummy", original_fn_id);

        // Replace the existing function with a new one with a reversed const value
        let new_fn_id = module
            .replace_exported_func(original_fn_id, |(body, _)| {
                body.i32_const(4321).drop();
            })
            .expect("function replacement worked");

        assert!(
            module.exports.get_exported_func(original_fn_id).is_none(),
            "replaced function cannot be gotten by ID"
        );

        // Ensure the function was replaced
        match module
            .exports
            .get_exported_func(new_fn_id)
            .expect("failed to unwrap exported func")
        {
            exp @ Export {
                item: ExportItem::Function(fid),
                ..
            } => {
                assert_eq!(*fid, new_fn_id, "retrieved function ID matches");
                assert_eq!(exp.id(), original_export_id, "export ID is unchanged");
            }
            _ => panic!("expected an Export with a Function inside"),
        }
    }

    /// Running `replace_exported_func` with a closure that returns None
    /// should replace the function with a generated no-op function
    #[test]
    fn replace_exported_func_generated_no_op() {
        let mut module = Module::default();

        // Create original function
        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let original_fn_id: FunctionId = builder.finish(vec![], &mut module.funcs);
        let original_export_id = module.exports.add("dummy", original_fn_id);

        // Replace the existing function with a new one with a reversed const value
        let new_fn_id = module
            .replace_exported_func(original_fn_id, |(body, _arg_locals)| {
                body.unreachable();
            })
            .expect("export function replacement worked");

        assert!(
            module.exports.get_exported_func(original_fn_id).is_none(),
            "replaced export function cannot be gotten by ID"
        );

        // Ensure the function was replaced
        match module
            .exports
            .get_exported_func(new_fn_id)
            .expect("failed to unwrap exported func")
        {
            exp @ Export {
                item: ExportItem::Function(fid),
                name,
                ..
            } => {
                assert_eq!(name, "dummy", "function name on export is unchanged");
                assert_eq!(*fid, new_fn_id, "retrieved function ID matches");
                assert_eq!(exp.id(), original_export_id, "export ID is unchanged");
            }
            _ => panic!("expected an Export with a Function inside"),
        }
    }

    /// Running `replace_imported_func` with a closure that builds
    /// a function should replace the existing function with the new one
    #[test]
    fn replace_imported_func() {
        let mut module = Module::default();

        // Create original import function
        let types = module.types.add(&[], &[]);
        let (original_fn_id, original_import_id) = module.add_import_func("mod", "dummy", types);

        // Replace the existing function with a new one with a reversed const value
        let new_fn_id = module
            .replace_imported_func(original_fn_id, |(body, _)| {
                body.i32_const(4321).drop();
            })
            .expect("import fn replacement worked");

        assert!(
            !module.imports.iter().any(|i| i.id() == original_import_id),
            "original import is missing",
        );

        assert!(
            module.imports.get_imported_func(original_fn_id).is_none(),
            "replaced import function cannot be gotten by ID"
        );

        assert!(
            module.imports.get_imported_func(new_fn_id).is_none(),
            "new import function cannot be gotten by ID (it is now local)"
        );

        assert!(
            matches!(module.funcs.get(new_fn_id).kind, FunctionKind::Local(_)),
            "new local function has the right kind"
        );
    }

    /// Running `replace_imported_func` with a closure that returns None
    /// should replace the function with a generated no-op function
    #[test]
    fn replace_imported_func_generated_no_op() {
        let mut module = Module::default();

        // Create original import function
        let types = module.types.add(&[], &[]);
        let (original_fn_id, original_import_id) = module.add_import_func("mod", "dummy", types);

        // Replace the existing function with a new one with a reversed const value
        let new_fn_id = module
            .replace_imported_func(original_fn_id, |(body, _arg_locals)| {
                body.unreachable();
            })
            .expect("import fn replacement worked");

        assert!(
            !module.imports.iter().any(|i| i.id() == original_import_id),
            "original import is missing",
        );

        assert!(
            module.imports.get_imported_func(original_fn_id).is_none(),
            "replaced import function cannot be gotten by ID"
        );

        assert!(
            module.imports.get_imported_func(new_fn_id).is_none(),
            "new import function cannot be gotten by ID (it is now local)"
        );

        assert!(
            matches!(module.funcs.get(new_fn_id).kind, FunctionKind::Local(_)),
            "new local function has the right kind"
        );
    }
}
