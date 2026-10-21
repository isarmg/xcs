#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestVisitor {
        visits: Vec<String>,
    }

    impl TestVisitor {
        fn push(&mut self, s: impl ToString) {
            self.visits.push(s.to_string());
        }
    }

    impl<'a> Visitor<'a> for TestVisitor {
        fn start_instr_seq(&mut self, _: &'a InstrSeq) {
            self.push("start");
        }

        fn end_instr_seq(&mut self, _: &'a InstrSeq) {
            self.push("end");
        }

        fn visit_const(&mut self, c: &Const) {
            match c.value {
                Value::I32(x) => self.push(x),
                _ => unreachable!(),
            }
        }

        fn visit_drop(&mut self, _: &Drop) {
            self.push("drop");
        }

        fn visit_block(&mut self, _: &Block) {
            self.push("block");
        }

        fn visit_if_else(&mut self, _: &IfElse) {
            self.push("if-else");
        }

        fn visit_try_table(&mut self, _: &TryTable) {
            self.push("try-table");
        }

        fn visit_try(&mut self, _: &Try) {
            self.push("try");
        }

        fn visit_tag_id(&mut self, _: &crate::TagId) {
            self.push("tag");
        }
    }

    impl VisitorMut for TestVisitor {
        fn start_instr_seq_mut(&mut self, _: &mut InstrSeq) {
            self.push("start");
        }

        fn end_instr_seq_mut(&mut self, _: &mut InstrSeq) {
            self.push("end");
        }

        fn visit_const_mut(&mut self, c: &mut Const) {
            match &mut c.value {
                Value::I32(x) => {
                    self.push(*x);
                    *x += 1;
                }
                _ => unreachable!(),
            }
        }

        fn visit_drop_mut(&mut self, _: &mut Drop) {
            self.push("drop");
        }

        fn visit_block_mut(&mut self, _: &mut Block) {
            self.push("block");
        }

        fn visit_if_else_mut(&mut self, _: &mut IfElse) {
            self.push("if-else");
        }

        fn visit_try_table_mut(&mut self, _: &mut TryTable) {
            self.push("try-table");
        }

        fn visit_try_mut(&mut self, _: &mut Try) {
            self.push("try");
        }

        fn visit_tag_id_mut(&mut self, _: &mut crate::TagId) {
            self.push("tag");
        }
    }

    fn make_test_func(module: &mut crate::Module) -> &mut LocalFunction {
        let block_ty = module.types.add(&[], &[]);
        let tag_type = module.types.add(&[], &[]);
        let tag_id = module.tags.add(tag_type);
        let mut builder = crate::FunctionBuilder::new(&mut module.types, &[], &[]);

        let try_block = Instr::Try(Try {
            seq: builder
                .dangling_instr_seq(block_ty)
                .i32_const(7)
                .drop()
                .id(),
            catches: vec![
                LegacyCatch::Catch {
                    tag: tag_id,
                    handler: builder
                        .dangling_instr_seq(block_ty)
                        .i32_const(8)
                        .drop()
                        .id(),
                },
                LegacyCatch::CatchAll {
                    handler: builder
                        .dangling_instr_seq(block_ty)
                        .i32_const(9)
                        .drop()
                        .id(),
                },
            ],
        });

        builder
            .func_body()
            .i32_const(1)
            .drop()
            .block(block_ty, |block| {
                block
                    .i32_const(2)
                    .drop()
                    .if_else(
                        block_ty,
                        |then| {
                            then.i32_const(3).drop();
                        },
                        |else_| {
                            else_.i32_const(4).drop();
                        },
                    )
                    .i32_const(5)
                    .drop();
            })
            .i32_const(6)
            .drop()
            .instr(try_block)
            .i32_const(10)
            .drop();

        let func_id = builder.finish(vec![], &mut module.funcs);
        module.funcs.get_mut(func_id).kind.unwrap_local_mut()
    }

    #[test]
    fn dfs_in_order() {
        let mut module = crate::Module::default();
        let func = make_test_func(&mut module);

        let mut visitor = TestVisitor::default();
        crate::ir::dfs_in_order(&mut visitor, func, func.entry_block());

        let mut expected = vec![];
        // Entry block start, then first instructions
        expected.extend(vec!["start", "1", "drop", "block"]);
        // Inside the block
        expected.extend(vec!["start", "2", "drop", "if-else"]);
        // Consequent
        expected.extend(vec!["start", "3", "drop", "end"]);
        // Alternative
        expected.extend(vec!["start", "4", "drop", "end"]);
        // Rest of block
        expected.extend(vec!["5", "drop", "end"]);
        // After block, before try
        expected.extend(vec!["6", "drop", "try", "tag"]);
        // Try body
        expected.extend(vec!["start", "7", "drop", "end"]);
        // Catch handler
        expected.extend(vec!["start", "8", "drop", "end"]);
        // CatchAll handler
        expected.extend(vec!["start", "9", "drop", "end"]);
        // After try
        expected.extend(vec!["10", "drop", "end"]);

        assert_eq!(
            visitor.visits,
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dfs_pre_order_mut() {
        let mut module = crate::Module::default();
        let func = make_test_func(&mut module);

        let mut visitor = TestVisitor::default();
        crate::ir::dfs_pre_order_mut(&mut visitor, func, func.entry_block());

        let mut expected = vec![];
        // function entry
        expected.extend(vec![
            "start", "1", "drop", "block", "6", "drop", "try", "tag", "10", "drop", "end",
        ]);
        // try body (pushed last, visited first due to LIFO)
        expected.extend(vec!["start", "7", "drop", "end"]);
        // catch handler
        expected.extend(vec!["start", "8", "drop", "end"]);
        // catch_all handler
        expected.extend(vec!["start", "9", "drop", "end"]);
        // block (pushed first, visited last)
        expected.extend(vec!["start", "2", "drop", "if-else", "5", "drop", "end"]);
        // consequent
        expected.extend(vec!["start", "3", "drop", "end"]);
        // alternative
        expected.extend(vec!["start", "4", "drop", "end"]);

        assert_eq!(
            visitor.visits,
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );

        // And then check that the increments of the constant values did indeed
        // take effect.

        visitor.visits.clear();
        crate::ir::dfs_in_order(&mut visitor, func, func.entry_block());

        let mut expected = vec![];
        // Entry block start, then first instructions (all values +1)
        expected.extend(vec!["start", "2", "drop", "block"]);
        // Inside the block
        expected.extend(vec!["start", "3", "drop", "if-else"]);
        // Consequent
        expected.extend(vec!["start", "4", "drop", "end"]);
        // Alternative
        expected.extend(vec!["start", "5", "drop", "end"]);
        // Rest of block
        expected.extend(vec!["6", "drop", "end"]);
        // After block, before try
        expected.extend(vec!["7", "drop", "try", "tag"]);
        // Try body
        expected.extend(vec!["start", "8", "drop", "end"]);
        // Catch handler
        expected.extend(vec!["start", "9", "drop", "end"]);
        // CatchAll handler
        expected.extend(vec!["start", "10", "drop", "end"]);
        // After try
        expected.extend(vec!["11", "drop", "end"]);

        assert_eq!(
            visitor.visits,
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn dfs_in_order_try_table() {
        let mut module = crate::Module::default();

        let tag_type = module.types.add(&[crate::ValType::I32], &[]);
        let tag_id = module.tags.add(tag_type);

        let block_ty = module.types.add(&[], &[]);
        let mut builder = crate::FunctionBuilder::new(&mut module.types, &[], &[]);

        let mut func_body = builder.func_body();

        // Build the try body
        let try_body = func_body.dangling_instr_seq(block_ty).id();
        func_body.instr_seq(try_body).i32_const(5).drop();

        // Create dummy label targets (these are branch targets, not handler blocks)
        let catch_label = func_body.dangling_instr_seq(block_ty).id();
        let catch_all_label = func_body.dangling_instr_seq(block_ty).id();

        func_body
            .i32_const(1)
            .drop()
            .instr(TryTable {
                seq: try_body,
                catches: vec![
                    TryTableCatch::Catch {
                        tag: tag_id,
                        label: catch_label,
                    },
                    TryTableCatch::CatchAll {
                        label: catch_all_label,
                    },
                ],
            })
            .i32_const(99)
            .drop();

        let func_id = builder.finish(vec![], &mut module.funcs);
        let func = module.funcs.get_mut(func_id).kind.unwrap_local_mut();

        let mut visitor = TestVisitor::default();
        crate::ir::dfs_in_order(&mut visitor, func, func.entry_block());

        // Expected order:
        // - start (entry block)
        // - 1, drop (before try-table)
        // - try-table instruction
        // - tag (from Catch clause visiting tag ID)
        // - start, 5, drop, end (try body)
        // - 99, drop (after try-table)
        // - end (entry block)
        // Note: catch labels are branch targets, not separate blocks to traverse
        let expected = [
            "start",
            "1",
            "drop",
            "try-table",
            "tag",
            "start",
            "5",
            "drop",
            "end", // try body
            "99",
            "drop",
            "end",
        ];

        assert_eq!(
            visitor.visits,
            expected.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
    }
}
