#[cfg(test)]
mod tests {
    use crate::{GlobalType, MemoryType, RefType, TableType, ValType, Validator, WasmFeatures};
    use anyhow::Result;

    #[test]
    fn test_module_type_information() -> Result<()> {
        let bytes = wat::parse_str(
            r#"
            (module
                (type (func (param i32 i64) (result i32)))
                (memory 1 5)
                (table 10 funcref)
                (global (mut i32) (i32.const 0))
                (func (type 0) (i32.const 0))
                (tag (param i64 i32))
                (elem funcref (ref.func 0))
            )
        "#,
        )?;

        let mut validator =
            Validator::new_with_features(WasmFeatures::default() | WasmFeatures::EXCEPTIONS);

        let types = validator.validate_all(&bytes)?;
        let types = types.as_ref();

        assert_eq!(types.core_type_count_in_module(), 2);
        assert_eq!(types.memory_count(), 1);
        assert_eq!(types.table_count(), 1);
        assert_eq!(types.global_count(), 1);
        assert_eq!(types.function_count(), 1);
        assert_eq!(types.tag_count(), 1);
        assert_eq!(types.element_count(), 1);
        assert_eq!(types.module_count(), 0);
        assert_eq!(types.component_count(), 0);
        assert_eq!(types.core_instance_count(), 0);
        assert_eq!(types.value_count(), 0);

        let id = types.core_type_at_in_module(0);
        let ty = types[id].unwrap_func();
        assert_eq!(ty.params(), [ValType::I32, ValType::I64]);
        assert_eq!(ty.results(), [ValType::I32]);

        let id = types.core_type_at_in_module(1);
        let ty = types[id].unwrap_func();
        assert_eq!(ty.params(), [ValType::I64, ValType::I32]);
        assert_eq!(ty.results(), []);

        assert_eq!(
            types.memory_at(0),
            MemoryType {
                memory64: false,
                shared: false,
                initial: 1,
                maximum: Some(5),
                page_size_log2: None,
            }
        );

        assert_eq!(
            types.table_at(0),
            TableType {
                initial: 10,
                maximum: None,
                element_type: RefType::FUNCREF,
                table64: false,
                shared: false,
            }
        );

        assert_eq!(
            types.global_at(0),
            GlobalType {
                content_type: ValType::I32,
                mutable: true,
                shared: false
            }
        );

        let id = types.core_function_at(0);
        let ty = types[id].unwrap_func();
        assert_eq!(ty.params(), [ValType::I32, ValType::I64]);
        assert_eq!(ty.results(), [ValType::I32]);

        let ty = types.tag_at(0);
        let ty = types[ty].unwrap_func();
        assert_eq!(ty.params(), [ValType::I64, ValType::I32]);
        assert_eq!(ty.results(), []);

        assert_eq!(types.element_at(0), RefType::FUNCREF);

        Ok(())
    }

    #[test]
    fn test_type_id_aliasing() -> Result<()> {
        let bytes = wat::parse_str(
            r#"
            (component
              (type $T (list string))
              (alias outer 0 $T (type $A1))
              (alias outer 0 $T (type $A2))
            )
        "#,
        )?;

        let mut validator =
            Validator::new_with_features(WasmFeatures::default() | WasmFeatures::COMPONENT_MODEL);

        let types = validator.validate_all(&bytes)?;
        let types = types.as_ref();

        let t_id = types.component_defined_type_at(0);
        let a1_id = types.component_defined_type_at(1);
        let a2_id = types.component_defined_type_at(2);

        // The ids should all be the same
        assert!(t_id == a1_id);
        assert!(t_id == a2_id);
        assert!(a1_id == a2_id);

        // However, they should all point to the same type
        assert!(std::ptr::eq(&types[t_id], &types[a1_id],));
        assert!(std::ptr::eq(&types[t_id], &types[a2_id],));

        Ok(())
    }

    #[test]
    fn test_type_id_exports() -> Result<()> {
        let bytes = wat::parse_str(
            r#"
            (component
              (type $T (list string))
              (export $A1 "A1" (type $T))
              (export $A2 "A2" (type $T))
            )
        "#,
        )?;

        let mut validator =
            Validator::new_with_features(WasmFeatures::default() | WasmFeatures::COMPONENT_MODEL);

        let types = validator.validate_all(&bytes)?;
        let types = types.as_ref();

        let t_id = types.component_defined_type_at(0);
        let a1_id = types.component_defined_type_at(1);
        let a2_id = types.component_defined_type_at(2);

        // The ids should all be the same
        assert!(t_id != a1_id);
        assert!(t_id != a2_id);
        assert!(a1_id != a2_id);

        // However, they should all point to the same type
        assert!(std::ptr::eq(&types[t_id], &types[a1_id],));
        assert!(std::ptr::eq(&types[t_id], &types[a2_id],));

        Ok(())
    }

    #[test]
    fn reset_fresh_validator() {
        Validator::new().reset();
    }
}
