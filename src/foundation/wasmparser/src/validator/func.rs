use super::operators::{Frame, OperatorValidator, OperatorValidatorAllocations};
use crate::{BinaryReader, Result, ValType, VisitOperator};
use crate::{FrameStack, FunctionBody, ModuleArity, Operator, WasmFeatures, WasmModuleResources};

/// Resources necessary to perform validation of a function.
///
/// This structure is created by
/// [`Validator::code_section_entry`](crate::Validator::code_section_entry) and
/// is created per-function in a WebAssembly module. This structure is suitable
/// for sending to other threads while the original
/// [`Validator`](crate::Validator) continues processing other functions.
#[derive(Debug)]
pub struct FuncToValidate<T> {
    /// Reusable, heap allocated resources to drive the Wasm validation.
    pub resources: T,
    /// The core Wasm function index being validated.
    pub index: u32,
    /// The core Wasm type index of the function being validated,
    /// defining the results and parameters to the function.
    pub ty: u32,
    /// The Wasm features enabled to validate the function.
    pub features: WasmFeatures,
}

impl<T: WasmModuleResources> FuncToValidate<T> {
    /// Converts this [`FuncToValidate`] into a [`FuncValidator`] using the
    /// `allocs` provided.
    ///
    /// This method, in conjunction with [`FuncValidator::into_allocations`],
    /// provides a means to reuse allocations across validation of each
    /// individual function. Note that it is also sufficient to call this
    /// method with `Default::default()` if no prior allocations are
    /// available.
    ///
    /// # Panics
    ///
    /// If a `FuncToValidate` was created with an invalid `ty` index then this
    /// function will panic.
    pub fn into_validator(self, allocs: FuncValidatorAllocations) -> FuncValidator<T> {
        let FuncToValidate {
            resources,
            index,
            ty,
            features,
        } = self;
        let validator =
            OperatorValidator::new_func(ty, 0, &features, &resources, allocs.0).unwrap();
        FuncValidator {
            validator,
            resources,
            index,
        }
    }
}

/// Validation context for a WebAssembly function.
///
/// This is a finalized validator which is ready to process a [`FunctionBody`].
/// This is created from the [`FuncToValidate::into_validator`] method.
#[derive(Clone)]
pub struct FuncValidator<T> {
    validator: OperatorValidator,
    resources: T,
    index: u32,
}

impl<T: WasmModuleResources> ModuleArity for FuncValidator<T> {
    fn sub_type_at(&self, type_idx: u32) -> Option<&crate::SubType> {
        self.resources.sub_type_at(type_idx)
    }

    fn tag_type_arity(&self, at: u32) -> Option<(u32, u32)> {
        let ty = self.resources.tag_at(at)?;
        Some((
            u32::try_from(ty.params().len()).unwrap(),
            u32::try_from(ty.results().len()).unwrap(),
        ))
    }

    fn type_index_of_function(&self, func_idx: u32) -> Option<u32> {
        self.resources.type_index_of_function(func_idx)
    }

    fn func_type_of_cont_type(&self, cont_ty: &crate::ContType) -> Option<&crate::FuncType> {
        let id = cont_ty.0.as_core_type_id()?;
        Some(self.resources.sub_type_at_id(id).unwrap_func())
    }

    fn sub_type_of_ref_type(&self, rt: &crate::RefType) -> Option<&crate::SubType> {
        let id = rt.type_index()?.as_core_type_id()?;
        Some(self.resources.sub_type_at_id(id))
    }

    fn control_stack_height(&self) -> u32 {
        u32::try_from(self.validator.control_stack_height()).unwrap()
    }

    fn label_block(&self, depth: u32) -> Option<(crate::BlockType, crate::FrameKind)> {
        self.validator.jump(depth)
    }
}

/// External handle to the internal allocations used during function validation.
///
/// This is created with either the `Default` implementation or with
/// [`FuncValidator::into_allocations`]. It is then passed as an argument to
/// [`FuncToValidate::into_validator`] to provide a means of reusing allocations
/// between each function.
#[derive(Default)]
pub struct FuncValidatorAllocations(OperatorValidatorAllocations);

impl<T: WasmModuleResources> FuncValidator<T> {
    /// Convenience function to validate an entire function's body.
    ///
    /// You may not end up using this in final implementations because you'll
    /// often want to interleave validation with parsing.
    pub fn validate(&mut self, body: &FunctionBody<'_>) -> Result<()> {
        let mut reader = body.get_binary_reader();
        self.read_locals(&mut reader)?;
        #[cfg(feature = "features")]
        {
            reader.set_features(self.validator.features);
        }
        while !reader.eof() {
            // In a `debug_check_try_op` build, verify that `rollback` successfully returns the
            // validator to its previous state after each (valid or invalid) operator.
            #[cfg(all(debug_check_try_op, feature = "try-op"))]
            {
                let snapshot = self.validator.clone();
                let op = reader.peek_operator(&self.visitor(reader.original_position()))?;
                self.validator.begin_try_op();
                let _ = self.op(reader.original_position(), &op);
                self.validator.rollback();
                self.validator.pop_push_log.clear();
                assert!(self.validator == snapshot);
            }

            // In a debug build, verify that the validator's pops and pushes to and from
            // the operand stack match the operator's arity.
            #[cfg(debug_assertions)]
            let (ops_before, arity) = {
                let op = reader.peek_operator(&self.visitor(reader.original_position()))?;
                let arity = op.operator_arity(&self.visitor(reader.original_position()));
                (reader.clone(), arity)
            };

            reader.visit_operator(&mut self.visitor(reader.original_position()))??;

            #[cfg(debug_assertions)]
            {
                let (params, results) = arity.ok_or(format_err!(
                    reader.original_position(),
                    "could not calculate operator arity"
                ))?;

                // Analyze the log to determine the actual, externally visible
                // pop/push count. This allows us to hide the fact that we might
                // push and then pop a temporary while validating an
                // instruction, which shouldn't be visible from the outside.
                let mut pop_count = 0;
                let mut push_count = 0;
                for op in self.validator.pop_push_log.drain(..) {
                    match op {
                        true => push_count += 1,
                        false if push_count > 0 => push_count -= 1,
                        false => pop_count += 1,
                    }
                }

                if pop_count != params || push_count != results {
                    panic!(
                        "\
arity mismatch in validation
    operator: {:?}
    expected: {params} -> {results}
    got       {pop_count} -> {push_count}",
                        ops_before.peek_operator(&self.visitor(ops_before.original_position()))?,
                    );
                }
            }
        }
        reader.finish_expression(&self.visitor(reader.original_position()))
    }

    /// Reads the local definitions from the given `BinaryReader`, often sourced
    /// from a `FunctionBody`.
    ///
    /// This function will automatically advance the `BinaryReader` forward,
    /// leaving reading operators up to the caller afterwards.
    pub fn read_locals(&mut self, reader: &mut BinaryReader<'_>) -> Result<()> {
        for _ in 0..reader.read_var_u32()? {
            let offset = reader.original_position();
            let cnt = reader.read()?;
            let ty = reader.read()?;
            self.define_locals(offset, cnt, ty)?;
        }
        Ok(())
    }

    /// Defines locals into this validator.
    ///
    /// This should be used if the application is already reading local
    /// definitions and there's no need to re-parse the function again.
    pub fn define_locals(&mut self, offset: usize, count: u32, ty: ValType) -> Result<()> {
        self.validator
            .define_locals(offset, count, ty, &self.resources)
    }

    /// Validates the next operator in a function.
    ///
    /// This function is expected to be called once-per-operator in a
    /// WebAssembly function. Each operator's offset in the original binary and
    /// the operator itself are passed to this function to provide more useful
    /// error messages. On error, the validator may be left in an undefined
    /// state and should not be reused.
    pub fn op(&mut self, offset: usize, operator: &Operator<'_>) -> Result<()> {
        self.visitor(offset).visit_operator(operator)
    }

    /// Validates the next operator in a function, rolling back the validator
    /// to its previous state if this is unsuccesful. The validator may be reused
    /// even after an error.
    #[cfg(feature = "try-op")]
    pub fn try_op(&mut self, offset: usize, operator: &Operator<'_>) -> Result<()> {
        self.validator.begin_try_op();
        let res = self.op(offset, operator);
        if res.is_ok() {
            self.validator.commit();
        } else {
            self.validator.rollback();
        }
        res
    }

    /// Get the operator visitor for the next operator in the function.
    ///
    /// The returned visitor is intended to visit just one instruction at the `offset`.
    ///
    /// # Example
    ///
    /// ```
    /// # use wasmparser::{WasmModuleResources, FuncValidator, FunctionBody, Result};
    /// pub fn validate<R>(validator: &mut FuncValidator<R>, body: &FunctionBody<'_>) -> Result<()>
    /// where R: WasmModuleResources
    /// {
    ///     let mut operator_reader = body.get_binary_reader_for_operators()?;
    ///     while !operator_reader.eof() {
    ///         let mut visitor = validator.visitor(operator_reader.original_position());
    ///         operator_reader.visit_operator(&mut visitor)??;
    ///     }
    ///     operator_reader.finish_expression(&validator.visitor(operator_reader.original_position()))
    /// }
    /// ```
    pub fn visitor<'this, 'a: 'this>(
        &'this mut self,
        offset: usize,
    ) -> impl VisitOperator<'a, Output = Result<()>> + ModuleArity + FrameStack + 'this {
        self.validator.with_resources(&self.resources, offset)
    }

    /// Same as [`FuncValidator::visitor`] except that the returned type
    /// implements the [`VisitSimdOperator`](crate::VisitSimdOperator) trait as
    /// well.
    #[cfg(feature = "simd")]
    pub fn simd_visitor<'this, 'a: 'this>(
        &'this mut self,
        offset: usize,
    ) -> impl crate::VisitSimdOperator<'a, Output = Result<()>> + ModuleArity + 'this {
        self.validator.with_resources_simd(&self.resources, offset)
    }

    /// Returns the Wasm features enabled for this validator.
    pub fn features(&self) -> &WasmFeatures {
        &self.validator.features
    }

    /// Returns the underlying module resources that this validator is using.
    pub fn resources(&self) -> &T {
        &self.resources
    }

    /// The index of the function within the module's function index space that
    /// is being validated.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Returns the number of defined local variables in the function.
    pub fn len_locals(&self) -> u32 {
        self.validator.locals.len_locals()
    }

    /// Returns the type of the local variable at the given `index` if any.
    pub fn get_local_type(&self, index: u32) -> Option<ValType> {
        self.validator.locals.get(index)
    }

    /// Get the current height of the operand stack.
    ///
    /// This returns the height of the whole operand stack for this function,
    /// not just for the current control frame.
    pub fn operand_stack_height(&self) -> u32 {
        self.validator.operand_stack_height() as u32
    }

    /// Returns the optional value type of the value operand at the given
    /// `depth` from the top of the operand stack.
    ///
    /// - Returns `None` if the `depth` is out of bounds.
    /// - Returns `Some(None)` if there is a value with unknown type
    /// at the given `depth`.
    ///
    /// # Note
    ///
    /// A `depth` of 0 will refer to the last operand on the stack.
    pub fn get_operand_type(&self, depth: usize) -> Option<Option<ValType>> {
        self.validator.peek_operand_at(depth)
    }

    /// Returns the number of frames on the control flow stack.
    ///
    /// This returns the height of the whole control stack for this function,
    /// not just for the current control frame.
    pub fn control_stack_height(&self) -> u32 {
        self.validator.control_stack_height() as u32
    }

    /// Returns a shared reference to the control flow [`Frame`] of the
    /// control flow stack at the given `depth` if any.
    ///
    /// Returns `None` if the `depth` is out of bounds.
    ///
    /// # Note
    ///
    /// A `depth` of 0 will refer to the last frame on the stack.
    pub fn get_control_frame(&self, depth: usize) -> Option<&Frame> {
        self.validator.get_frame(depth)
    }

    /// Consumes this validator and returns the underlying allocations that
    /// were used during the validation process.
    ///
    /// The returned value here can be paired with
    /// [`FuncToValidate::into_validator`] to reuse the allocations already
    /// created by this validator.
    pub fn into_allocations(self) -> FuncValidatorAllocations {
        FuncValidatorAllocations(self.validator.into_allocations())
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/wasmparser/unit/src/validator/func/tests_351.rs"));
