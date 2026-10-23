//! Implementations of various IR traversals.

use crate::ir::*;

/// Perform an intra-procedural, depth-first, in-order traversal of the IR.
///
/// * *Intra-procedural*: Only traverses IR within a function. Does not cross
///   function boundaries (although it will report edges to other functions via
///   `visit_function_id` calls on the visitor, so you can use this as a
///   building block for making global, inter-procedural analyses).
///
/// * *Depth-first, in-order*: Visits instructions and instruction sequences in
///   the order they are defined and nested. See [Wikipedia][in-order] for
///   details.
///
/// Calls `visitor` methods for every instruction, instruction sequence, and
/// resource that the traversal visits.
///
/// The traversals begins at the `start` instruction sequence and goes from
/// there. To traverse everything in a function, pass `func.entry_block()` as
/// `start`.
///
/// This implementation is iterative &mdash; not recursive &mdash; and so it
/// will not blow the call stack on deeply nested Wasm (although it may still
/// OOM).
///
/// [in-order]: https://en.wikipedia.org/wiki/Tree_traversal#In-order_(LNR)
///
/// # Example
///
/// This example counts the number of instruction sequences in a function.
///
/// ```no_run
/// use walrus::LocalFunction;
/// use walrus::ir::*;
///
/// #[derive(Default)]
/// struct CountInstructionSequences {
///     count: usize,
/// }
///
/// impl<'instr> Visitor<'instr> for CountInstructionSequences {
///     fn start_instr_seq(&mut self, _: &'instr InstrSeq) {
///         self.count += 1;
///     }
/// }
///
/// // Get a function from somewhere.
/// # let get_my_function = || unimplemented!();
/// let my_func: &LocalFunction = get_my_function();
///
/// // Create our visitor.
/// let mut visitor = CountInstructionSequences::default();
///
/// // Traverse everything in the function with our visitor.
/// dfs_in_order(&mut visitor, my_func, my_func.entry_block());
///
/// // Use the aggregate results that `visitor` built up.
/// println!("The number of instruction sequences in `my_func` is {}", visitor.count);
/// ```
pub fn dfs_in_order<'instr>(
    visitor: &mut impl Visitor<'instr>,
    func: &'instr LocalFunction,
    start: InstrSeqId,
) {
    // The stack of instruction sequences we still need to visit, and how far
    // along in the instruction sequence we are.
    let mut stack: Vec<(InstrSeqId, usize)> = vec![(start, 0)];

    'traversing_blocks: while let Some((seq_id, index)) = stack.pop() {
        let seq = func.block(seq_id);

        if index == 0 {
            // If the `index` is zero, then we haven't processed any
            // instructions in this sequence yet, and it is the first time we
            // are entering it, so let the visitor know.
            visitor.start_instr_seq(seq);
            seq.visit(visitor);
        }

        'traversing_instrs: for (index, (instr, loc)) in seq.instrs.iter().enumerate().skip(index) {
            // Visit this instruction.
            log::trace!("dfs_in_order: visit_instr({:?})", instr);
            visitor.visit_instr(instr, loc);

            // Visit every other resource that this instruction references,
            // e.g. `MemoryId`s, `FunctionId`s and all that.
            log::trace!("dfs_in_order: ({:?}).visit(..)", instr);
            instr.visit(visitor);

            match instr {
                // Pause iteration through this sequence's instructions and
                // enqueue `seq` to be traversed next before continuing with
                // this one where we left off.
                Instr::Block(Block { seq }) | Instr::Loop(Loop { seq }) => {
                    stack.push((seq_id, index + 1));
                    stack.push((*seq, 0));
                    continue 'traversing_blocks;
                }

                // Pause iteration through this sequence's instructions.
                // Traverse the consequent and then the alternative.
                Instr::IfElse(IfElse {
                    consequent,
                    alternative,
                }) => {
                    stack.push((seq_id, index + 1));
                    stack.push((*alternative, 0));
                    stack.push((*consequent, 0));
                    continue 'traversing_blocks;
                }

                // Pause iteration through this sequence's instructions.
                // Traverse the try_table body.
                Instr::TryTable(TryTable { seq, catches }) => {
                    stack.push((seq_id, index + 1));
                    // Visit catch clauses to mark tags as used.
                    // Note: The labels in TryTable catches are branch targets, not handler
                    // blocks. They reference blocks already in the control flow and will be
                    // visited naturally during traversal.
                    for catch in catches.iter() {
                        log::trace!("dfs_in_order: ({:?}).visit(..)", catch);
                        catch.visit(visitor);
                    }
                    stack.push((*seq, 0));
                    continue 'traversing_blocks;
                }

                // Pause iteration and traverse the try body and all catch handlers.
                Instr::Try(Try { seq, catches }) => {
                    stack.push((seq_id, index + 1));
                    // Visit catch instructions in order.
                    for catch in catches.iter() {
                        log::trace!("dfs_in_order: ({:?}).visit(..)", catch);
                        catch.visit(visitor);
                    }
                    // Push catch handlers in reverse order so they are visited in order
                    for catch in catches.iter().rev() {
                        match catch {
                            LegacyCatch::Catch { handler, .. }
                            | LegacyCatch::CatchAll { handler } => {
                                stack.push((*handler, 0));
                            }
                            LegacyCatch::Delegate { .. } => {
                                // Delegate doesn't have a handler block
                            }
                        }
                    }
                    // Push the try body last so it's visited first
                    stack.push((*seq, 0));
                    continue 'traversing_blocks;
                }

                // No other instructions define new instruction sequences, so
                // continue to the next instruction.
                _ => continue 'traversing_instrs,
            }
        }

        // If we made it through the whole loop above, then we processed every
        // instruction in the sequence, and its nested sequences, so we are
        // finished with it!
        visitor.end_instr_seq(seq);
    }
}

/// Perform an intra-procedural, depth-first, pre-order, mutable traversal of
/// the IR.
///
/// * *Intra-procedural*: Only traverses IR within a function. Does not cross
///   function boundaries (although it will report edges to other functions via
///   `visit_function_id` calls on the visitor, so you can use this as a
///   building block for making global, inter-procedural analyses).
///
/// * *Depth-first, pre-order*: Visits instructions and instruction sequences in
///   a top-down manner, where all instructions in a parent sequences are
///   visited before child sequences. See [Wikipedia][pre-order] for details.
///
/// Calls `visitor` methods for every instruction, instruction sequence, and
/// resource that the traversal visits.
///
/// The traversals begins at the `start` instruction sequence and goes from
/// there. To traverse everything in a function, pass `func.entry_block()` as
/// `start`.
///
/// This implementation is iterative &mdash; not recursive &mdash; and so it
/// will not blow the call stack on deeply nested Wasm (although it may still
/// OOM).
///
/// [pre-order]: https://en.wikipedia.org/wiki/Tree_traversal#Pre-order_(NLR)
///
/// # Example
///
/// This example walks the IR and adds one to all `i32.const`'s values.
///
/// ```no_run
/// use walrus::LocalFunction;
/// use walrus::ir::*;
///
/// #[derive(Default)]
/// struct AddOneToI32Consts;
///
/// impl VisitorMut for AddOneToI32Consts {
///     fn visit_const_mut(&mut self, c: &mut Const) {
///         match &mut c.value {
///             Value::I32(x) => {
///                 *x += 1;
///             }
///             _ => {},
///         }
///     }
/// }
///
/// // Get a function from somewhere.
/// # let get_my_function = || unimplemented!();
/// let my_func: &mut LocalFunction = get_my_function();
///
/// // Create our visitor.
/// let mut visitor = AddOneToI32Consts::default();
///
/// // Traverse and mutate everything in the function with our visitor.
/// dfs_pre_order_mut(&mut visitor, my_func, my_func.entry_block());
/// ```
pub fn dfs_pre_order_mut(
    visitor: &mut impl VisitorMut,
    func: &mut LocalFunction,
    start: InstrSeqId,
) {
    let mut stack = vec![start];

    while let Some(seq_id) = stack.pop() {
        let seq = func.block_mut(seq_id);
        visitor.start_instr_seq_mut(seq);
        seq.visit_mut(visitor);

        for (instr, loc) in &mut seq.instrs {
            visitor.visit_instr_mut(instr, loc);
            instr.visit_mut(visitor);

            match instr {
                Instr::Block(Block { seq }) | Instr::Loop(Loop { seq }) => {
                    stack.push(*seq);
                }

                Instr::IfElse(IfElse {
                    consequent,
                    alternative,
                }) => {
                    stack.push(*alternative);
                    stack.push(*consequent);
                }

                Instr::TryTable(TryTable { seq, catches }) => {
                    // Visit catch clauses to mark tags as used.
                    // Note: The labels in TryTable catches are branch targets, not handler
                    // blocks. They reference blocks already in the control flow and will be
                    // visited naturally during traversal.
                    for catch in catches.iter_mut() {
                        catch.visit_mut(visitor);
                    }
                    stack.push(*seq);
                }

                Instr::Try(Try { seq, catches }) => {
                    for catch in catches.iter_mut() {
                        catch.visit_mut(visitor);
                    }
                    // Push catch handlers in reverse order so they are visited in order
                    for catch in catches.iter().rev() {
                        match catch {
                            LegacyCatch::Catch { handler, .. }
                            | LegacyCatch::CatchAll { handler } => {
                                stack.push(*handler);
                            }
                            LegacyCatch::Delegate { .. } => {
                                // Delegate doesn't have a handler block
                            }
                        }
                    }
                    // Push the try body last so it's visited first
                    stack.push(*seq);
                }

                _ => {}
            }
        }

        visitor.end_instr_seq_mut(seq);
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/walrus/unit/src/ir/traversals/tests_292.rs"));
