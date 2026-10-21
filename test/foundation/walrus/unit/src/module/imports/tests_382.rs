#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionBuilder, Module};

    #[test]
    fn get_imported_func() {
        let mut module = Module::default();

        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let new_fn_id: FunctionId = builder.finish(vec![], &mut module.funcs);
        module.imports.add("mod", "dummy", new_fn_id);

        assert!(module.imports.get_imported_func(new_fn_id).is_some());
    }

    #[test]
    fn get_func_by_name() {
        let mut module = Module::default();

        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let new_fn_id: FunctionId = builder.finish(vec![], &mut module.funcs);
        module.imports.add("mod", "dummy", new_fn_id);

        assert!(module
            .imports
            .get_func("mod", "dummy")
            .is_ok_and(|fid| fid == new_fn_id));
    }
}
