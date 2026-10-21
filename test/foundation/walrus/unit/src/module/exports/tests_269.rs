#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionBuilder, Module};
    use id_arena::Arena;

    /// this function always returns the same ID
    fn always_the_same_id<T>() -> Id<T> {
        let arena: Arena<T> = Arena::new();
        arena.next_id()
    }

    #[test]
    fn get_exported_func() {
        let mut module = Module::default();
        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let id: FunctionId = builder.finish(vec![], &mut module.funcs);
        module.exports.add("dummy", id);

        let actual: Option<&Export> = module.exports.get_exported_func(id);

        let export: &Export = actual.expect("Expected Some(Export) got None");
        assert_eq!(export.name, "dummy");
        match export.item {
            ExportItem::Function(f) => assert_eq!(f, id),
            _ => panic!("Expected a Function variant"),
        }
    }

    #[test]
    fn get_exported_func_should_return_none_for_unknown_function_id() {
        let module = Module::default();
        let id: FunctionId = always_the_same_id();
        let actual: Option<&Export> = module.exports.get_exported_func(id);
        assert!(actual.is_none());
    }

    #[test]
    fn get_exported_table() {
        let mut module = Module::default();
        let id: TableId = always_the_same_id();
        module.exports.add("dummy", id);

        let actual: Option<&Export> = module.exports.get_exported_table(id);

        let export: &Export = actual.expect("Expected Some(Export) got None");
        assert_eq!(export.name, "dummy");
        match export.item {
            ExportItem::Table(f) => assert_eq!(f, id),
            _ => panic!("Expected a Table variant"),
        }
    }

    #[test]
    fn get_exported_table_should_return_none_for_unknown_table_id() {
        let module = Module::default();
        let id: TableId = always_the_same_id();
        let actual: Option<&Export> = module.exports.get_exported_table(id);
        assert!(actual.is_none());
    }

    #[test]
    fn get_exported_memory() {
        let mut module = Module::default();
        let id: MemoryId = always_the_same_id();
        module.exports.add("dummy", id);

        let actual: Option<&Export> = module.exports.get_exported_memory(id);

        let export: &Export = actual.expect("Expected Some(Export) got None");
        assert_eq!(export.name, "dummy");
        match export.item {
            ExportItem::Memory(f) => assert_eq!(f, id),
            _ => panic!("Expected a Memory variant"),
        }
    }

    #[test]
    fn get_exported_memory_should_return_none_for_unknown_memory_id() {
        let module = Module::default();
        let id: MemoryId = always_the_same_id();
        let actual: Option<&Export> = module.exports.get_exported_memory(id);
        assert!(actual.is_none());
    }

    #[test]
    fn get_exported_global() {
        let mut module = Module::default();
        let id: GlobalId = always_the_same_id();
        module.exports.add("dummy", id);

        let actual: Option<&Export> = module.exports.get_exported_global(id);

        let export: &Export = actual.expect("Expected Some(Export) got None");
        assert_eq!(export.name, "dummy");
        match export.item {
            ExportItem::Global(f) => assert_eq!(f, id),
            _ => panic!("Expected a Global variant"),
        }
    }

    #[test]
    fn get_exported_global_should_return_none_for_unknown_global_id() {
        let module = Module::default();
        let id: GlobalId = always_the_same_id();
        let actual: Option<&Export> = module.exports.get_exported_global(id);
        assert!(actual.is_none());
    }

    #[test]
    fn delete() {
        let mut module = Module::default();
        let fn_id: FunctionId = always_the_same_id();
        let export_id: ExportId = module.exports.add("dummy", fn_id);
        assert!(module.exports.get_exported_func(fn_id).is_some());
        module.exports.delete(export_id);

        assert!(module.exports.get_exported_func(fn_id).is_none());
    }

    #[test]
    fn get_func_by_name() {
        let mut module = Module::default();
        let fn_id: FunctionId = always_the_same_id();
        let export_id: ExportId = module.exports.add("dummy", fn_id);
        assert!(module.exports.get_func("dummy").is_ok());
        module.exports.delete(export_id);
        assert!(module.exports.get_func("dummy").is_err());
    }

    #[test]
    fn iter_mut_can_update_export_item() {
        let mut module = Module::default();

        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let fn_id0: FunctionId = builder.finish(vec![], &mut module.funcs);

        let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
        builder.func_body().i32_const(1234).drop();
        let fn_id1: FunctionId = builder.finish(vec![], &mut module.funcs);

        assert_ne!(fn_id0, fn_id1);

        let export_id: ExportId = module.exports.add("dummy", fn_id0);
        assert!(module.exports.get_exported_func(fn_id0).is_some());

        module
            .exports
            .iter_mut()
            .for_each(|x: &mut Export| x.item = ExportItem::Function(fn_id1));

        assert!(module.exports.get_exported_func(fn_id0).is_none());
        let actual: &Export = module
            .exports
            .get_exported_func(fn_id1)
            .expect("Expected Some(Export) got None");
        assert_eq!(actual.id, export_id);
        assert_eq!(actual.name, "dummy");
        match actual.item {
            ExportItem::Function(f) => assert_eq!(f, fn_id1),
            _ => panic!("Expected a Function variant"),
        }
    }
}
