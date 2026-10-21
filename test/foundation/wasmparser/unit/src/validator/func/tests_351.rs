#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CoreTypeId;
    use crate::{HeapType, Parser, RefType, Validator};
    use alloc::vec::Vec;

    struct EmptyResources(crate::SubType);

    impl Default for EmptyResources {
        fn default() -> Self {
            EmptyResources(crate::SubType {
                supertype_idx: None,
                is_final: true,
                composite_type: crate::CompositeType {
                    inner: crate::CompositeInnerType::Func(crate::FuncType::new([], [])),
                    shared: false,
                    descriptor_idx: None,
                    describes_idx: None,
                },
            })
        }
    }

    impl WasmModuleResources for EmptyResources {
        fn table_at(&self, _at: u32) -> Option<crate::TableType> {
            todo!()
        }
        fn memory_at(&self, _at: u32) -> Option<crate::MemoryType> {
            todo!()
        }
        fn tag_at(&self, _at: u32) -> Option<&crate::FuncType> {
            todo!()
        }
        fn global_at(&self, _at: u32) -> Option<crate::GlobalType> {
            todo!()
        }
        fn sub_type_at(&self, _type_idx: u32) -> Option<&crate::SubType> {
            Some(&self.0)
        }
        fn sub_type_at_id(&self, _id: CoreTypeId) -> &crate::SubType {
            todo!()
        }
        fn type_id_of_function(&self, _at: u32) -> Option<CoreTypeId> {
            todo!()
        }
        fn type_index_of_function(&self, _at: u32) -> Option<u32> {
            todo!()
        }
        fn check_heap_type(&self, _t: &mut HeapType, _offset: usize) -> Result<()> {
            Ok(())
        }
        fn top_type(&self, _heap_type: &HeapType) -> HeapType {
            todo!()
        }
        fn element_type_at(&self, _at: u32) -> Option<crate::RefType> {
            todo!()
        }
        fn is_subtype(&self, _t1: ValType, _t2: ValType) -> bool {
            todo!()
        }
        fn is_shared(&self, _ty: RefType) -> bool {
            todo!()
        }
        fn element_count(&self) -> u32 {
            todo!()
        }
        fn data_count(&self) -> Option<u32> {
            todo!()
        }
        fn is_function_referenced(&self, _idx: u32) -> bool {
            todo!()
        }
        fn has_function_exact_type(&self, _idx: u32) -> bool {
            todo!()
        }
    }

    #[test]
    fn operand_stack_height() {
        let mut v = FuncToValidate {
            index: 0,
            ty: 0,
            resources: EmptyResources::default(),
            features: Default::default(),
        }
        .into_validator(Default::default());

        // Initially zero values on the stack.
        assert_eq!(v.operand_stack_height(), 0);

        // Pushing a constant value makes use have one value on the stack.
        assert!(v.op(0, &Operator::I32Const { value: 0 }).is_ok());
        assert_eq!(v.operand_stack_height(), 1);

        // Entering a new control block does not affect the stack height.
        assert!(
            v.op(
                1,
                &Operator::Block {
                    blockty: crate::BlockType::Empty
                }
            )
            .is_ok()
        );
        assert_eq!(v.operand_stack_height(), 1);

        // Pushing another constant value makes use have two values on the stack.
        assert!(v.op(2, &Operator::I32Const { value: 99 }).is_ok());
        assert_eq!(v.operand_stack_height(), 2);
    }

    fn assert_arity(wat: &str, expected: Vec<Vec<(u32, u32)>>) {
        let wasm = wat::parse_str(wat).unwrap();
        assert!(Validator::new().validate_all(&wasm).is_ok());

        let parser = Parser::new(0);
        let mut validator = Validator::new();

        let mut actual = vec![];

        for payload in parser.parse_all(&wasm) {
            let payload = payload.unwrap();
            match payload {
                crate::Payload::CodeSectionEntry(body) => {
                    let mut arity = vec![];
                    let mut func_validator = validator
                        .code_section_entry(&body)
                        .unwrap()
                        .into_validator(FuncValidatorAllocations::default());
                    let ops = body.get_operators_reader().unwrap();
                    for op in ops.into_iter() {
                        let op = op.unwrap();
                        arity.push(
                            op.operator_arity(&func_validator)
                                .expect("valid operators should have arity"),
                        );
                        func_validator.op(usize::MAX, &op).expect("should be valid");
                    }
                    actual.push(arity);
                }
                p => {
                    validator.payload(&p).unwrap();
                }
            }
        }

        assert_eq!(actual, expected);
    }

    #[test]
    fn arity_smoke_test() {
        let wasm = r#"
            (module
                (type $pair (struct (field i32) (field i32)))

                (func $add (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )

                (func $f (param i32 i32) (result (ref null $pair))
                    local.get 0
                    local.get 1
                    call $add
                    if (result (ref null $pair))
                    local.get 0
                    local.get 1
                      struct.new $pair
                    else
                      unreachable
                      i32.add
                      unreachable
                    end
                )
            )
        "#;

        assert_arity(
            wasm,
            vec![
                // $add
                vec![
                    // local.get 0
                    (0, 1),
                    // local.get 1
                    (0, 1),
                    // i32.add
                    (2, 1),
                    // end
                    (1, 1),
                ],
                // $f
                vec![
                    // local.get 0
                    (0, 1),
                    // local.get 1
                    (0, 1),
                    // call $add
                    (2, 1),
                    // if
                    (1, 0),
                    // local.get 0
                    (0, 1),
                    // local.get 1
                    (0, 1),
                    // struct.new $pair
                    (2, 1),
                    // else
                    (1, 0),
                    // unreachable,
                    (0, 0),
                    // i32.add
                    (2, 1),
                    // unreachable
                    (0, 0),
                    // end
                    (1, 1),
                    // implicit end
                    (1, 1),
                ],
            ],
        );
    }

    #[test]
    fn arity_if_no_else_same_params_and_results() {
        let wasm = r#"
            (module
                (func (export "f") (param i64 i32) (result i64)
                    (local.get 0)
                    (local.get 1)
                    ;; If with no else. Same number of params and results.
                    if (param i64) (result i64)
                        drop
                        i64.const -1
                    end
                )
            )
        "#;

        assert_arity(
            wasm,
            vec![vec![
                // local.get 0
                (0, 1),
                // local.get 1
                (0, 1),
                // if
                (2, 1),
                // drop
                (1, 0),
                // i64.const -1
                (0, 1),
                // end
                (1, 1),
                // implicit end
                (1, 1),
            ]],
        );
    }

    #[test]
    fn arity_br_table() {
        let wasm = r#"
            (module
                (func (export "f") (result i32 i32)
                    i32.const 0
                    i32.const 1
                    i32.const 2
                    br_table 0 0
                )
            )
        "#;

        assert_arity(
            wasm,
            vec![vec![
                // i32.const 0
                (0, 1),
                // i32.const 1
                (0, 1),
                // i32.const 2
                (0, 1),
                // br_table
                (3, 0),
                // implicit end
                (2, 2),
            ]],
        );
    }
}
