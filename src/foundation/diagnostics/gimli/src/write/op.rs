use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::common::{Encoding, Register};
use crate::constants::{self, DwOp};
use crate::leb128::write::{sleb128_size, uleb128_size};
use crate::write::{
    Address, DebugInfoReference, Error, Reference, Result, UnitEntryId, UnitOffsets, Writer,
};

/// The bytecode for a DWARF expression or location description.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Expression {
    operations: Vec<Operation>,
}

impl Expression {
    /// Create an empty expression.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an expression from raw bytecode.
    ///
    /// This does not support operations that require references, such as `DW_OP_addr`.
    #[inline]
    pub fn raw(bytecode: Vec<u8>) -> Self {
        Expression {
            operations: vec![Operation::Raw(bytecode)],
        }
    }

    /// Add an operation to the expression.
    ///
    /// This should only be used for operations that have no explicit operands.
    pub fn op(&mut self, opcode: DwOp) {
        self.operations.push(Operation::Simple(opcode));
    }

    /// Add a `DW_OP_addr` operation to the expression.
    pub fn op_addr(&mut self, address: Address) {
        self.operations.push(Operation::Address(address));
    }

    /// Add a `DW_OP_constu` operation to the expression.
    ///
    /// This may be emitted as a smaller equivalent operation.
    pub fn op_constu(&mut self, value: u64) {
        self.operations.push(Operation::UnsignedConstant(value));
    }

    /// Add a `DW_OP_consts` operation to the expression.
    ///
    /// This may be emitted as a smaller equivalent operation.
    pub fn op_consts(&mut self, value: i64) {
        self.operations.push(Operation::SignedConstant(value));
    }

    /// Add a `DW_OP_const_type` or `DW_OP_GNU_const_type` operation to the expression.
    pub fn op_const_type(&mut self, base: UnitEntryId, value: Box<[u8]>) {
        self.operations.push(Operation::ConstantType(base, value));
    }

    /// Add a `DW_OP_fbreg` operation to the expression.
    pub fn op_fbreg(&mut self, offset: i64) {
        self.operations.push(Operation::FrameOffset(offset));
    }

    /// Add a `DW_OP_bregx` operation to the expression.
    ///
    /// This may be emitted as a smaller equivalent operation.
    pub fn op_breg(&mut self, register: Register, offset: i64) {
        self.operations
            .push(Operation::RegisterOffset(register, offset));
    }

    /// Add a `DW_OP_regval_type` or `DW_OP_GNU_regval_type` operation to the expression.
    ///
    /// This may be emitted as a smaller equivalent operation.
    pub fn op_regval_type(&mut self, register: Register, base: UnitEntryId) {
        self.operations
            .push(Operation::RegisterType(register, base));
    }

    /// Add a `DW_OP_pick` operation to the expression.
    ///
    /// This may be emitted as a `DW_OP_dup` or `DW_OP_over` operation.
    pub fn op_pick(&mut self, index: u8) {
        self.operations.push(Operation::Pick(index));
    }

    /// Add a `DW_OP_deref` operation to the expression.
    pub fn op_deref(&mut self) {
        self.operations.push(Operation::Deref { space: false });
    }

    /// Add a `DW_OP_xderef` operation to the expression.
    pub fn op_xderef(&mut self) {
        self.operations.push(Operation::Deref { space: true });
    }

    /// Add a `DW_OP_deref_size` operation to the expression.
    pub fn op_deref_size(&mut self, size: u8) {
        self.operations
            .push(Operation::DerefSize { size, space: false });
    }

    /// Add a `DW_OP_xderef_size` operation to the expression.
    pub fn op_xderef_size(&mut self, size: u8) {
        self.operations
            .push(Operation::DerefSize { size, space: true });
    }

    /// Add a `DW_OP_deref_type` or `DW_OP_GNU_deref_type` operation to the expression.
    pub fn op_deref_type(&mut self, size: u8, base: UnitEntryId) {
        self.operations.push(Operation::DerefType {
            size,
            base,
            space: false,
        });
    }

    /// Add a `DW_OP_xderef_type` operation to the expression.
    pub fn op_xderef_type(&mut self, size: u8, base: UnitEntryId) {
        self.operations.push(Operation::DerefType {
            size,
            base,
            space: true,
        });
    }

    /// Add a `DW_OP_plus_uconst` operation to the expression.
    pub fn op_plus_uconst(&mut self, value: u64) {
        self.operations.push(Operation::PlusConstant(value));
    }

    /// Add a `DW_OP_skip` operation to the expression.
    ///
    /// Returns the index of the operation. The caller must call `set_target` with
    /// this index to set the target of the branch.
    pub fn op_skip(&mut self) -> usize {
        let index = self.next_index();
        self.operations.push(Operation::Skip(!0));
        index
    }

    /// Add a `DW_OP_bra` operation to the expression.
    ///
    /// Returns the index of the operation. The caller must call `set_target` with
    /// this index to set the target of the branch.
    pub fn op_bra(&mut self) -> usize {
        let index = self.next_index();
        self.operations.push(Operation::Branch(!0));
        index
    }

    /// Return the index that will be assigned to the next operation.
    ///
    /// This can be passed to `set_target`.
    #[inline]
    pub fn next_index(&self) -> usize {
        self.operations.len()
    }

    /// Set the target of a `DW_OP_skip` or `DW_OP_bra` operation .
    pub fn set_target(&mut self, operation: usize, new_target: usize) {
        debug_assert!(new_target <= self.next_index());
        debug_assert_ne!(operation, new_target);
        match self.operations[operation] {
            Operation::Skip(ref mut target) | Operation::Branch(ref mut target) => {
                *target = new_target;
            }
            _ => unimplemented!(),
        }
    }

    /// Add a `DW_OP_call4` operation to the expression.
    pub fn op_call(&mut self, entry: UnitEntryId) {
        self.operations.push(Operation::Call(entry));
    }

    /// Add a `DW_OP_call_ref` operation to the expression.
    pub fn op_call_ref(&mut self, entry: Reference) {
        self.operations.push(Operation::CallRef(entry));
    }

    /// Add a `DW_OP_convert` or `DW_OP_GNU_convert` operation to the expression.
    ///
    /// `base` is the DIE of the base type, or `None` for the generic type.
    pub fn op_convert(&mut self, base: Option<UnitEntryId>) {
        self.operations.push(Operation::Convert(base));
    }

    /// Add a `DW_OP_reinterpret` or `DW_OP_GNU_reinterpret` operation to the expression.
    ///
    /// `base` is the DIE of the base type, or `None` for the generic type.
    pub fn op_reinterpret(&mut self, base: Option<UnitEntryId>) {
        self.operations.push(Operation::Reinterpret(base));
    }

    /// Add a `DW_OP_entry_value` or `DW_OP_GNU_entry_value` operation to the expression.
    pub fn op_entry_value(&mut self, expression: Expression) {
        self.operations.push(Operation::EntryValue(expression));
    }

    /// Add a `DW_OP_regx` operation to the expression.
    ///
    /// This may be emitted as a smaller equivalent operation.
    pub fn op_reg(&mut self, register: Register) {
        self.operations.push(Operation::Register(register));
    }

    /// Add a `DW_OP_implicit_value` operation to the expression.
    pub fn op_implicit_value(&mut self, data: Box<[u8]>) {
        self.operations.push(Operation::ImplicitValue(data));
    }

    /// Add a `DW_OP_implicit_pointer` or `DW_OP_GNU_implicit_pointer` operation to the expression.
    pub fn op_implicit_pointer(&mut self, entry: Reference, byte_offset: i64) {
        self.operations
            .push(Operation::ImplicitPointer { entry, byte_offset });
    }

    /// Add a `DW_OP_piece` operation to the expression.
    pub fn op_piece(&mut self, size_in_bytes: u64) {
        self.operations.push(Operation::Piece { size_in_bytes });
    }

    /// Add a `DW_OP_bit_piece` operation to the expression.
    pub fn op_bit_piece(&mut self, size_in_bits: u64, bit_offset: u64) {
        self.operations.push(Operation::BitPiece {
            size_in_bits,
            bit_offset,
        });
    }

    /// Add a `DW_OP_GNU_parameter_ref` operation to the expression.
    pub fn op_gnu_parameter_ref(&mut self, entry: UnitEntryId) {
        self.operations.push(Operation::ParameterRef(entry));
    }

    /// Add a `DW_OP_WASM_location 0x0` operation to the expression.
    pub fn op_wasm_local(&mut self, index: u32) {
        self.operations.push(Operation::WasmLocal(index));
    }

    /// Add a `DW_OP_WASM_location 0x1` operation to the expression.
    pub fn op_wasm_global(&mut self, index: u32) {
        self.operations.push(Operation::WasmGlobal(index));
    }

    /// Add a `DW_OP_WASM_location 0x2` operation to the expression.
    pub fn op_wasm_stack(&mut self, index: u32) {
        self.operations.push(Operation::WasmStack(index));
    }

    pub(crate) fn size(
        &self,
        encoding: Encoding,
        unit_offsets: Option<&UnitOffsets>,
    ) -> Result<usize> {
        let mut size = 0;
        for operation in &self.operations {
            size += operation.size(encoding, unit_offsets)?;
        }
        Ok(size)
    }

    pub(crate) fn write<W: Writer>(
        &self,
        w: &mut W,
        mut refs: Option<&mut Vec<DebugInfoReference>>,
        encoding: Encoding,
        unit_offsets: Option<&UnitOffsets>,
    ) -> Result<()> {
        // TODO: only calculate offsets if needed?
        let mut offsets = Vec::with_capacity(self.operations.len());
        let mut offset = w.len();
        for operation in &self.operations {
            offsets.push(offset);
            offset += operation.size(encoding, unit_offsets)?;
        }
        offsets.push(offset);
        for (operation, offset) in self.operations.iter().zip(offsets.iter().copied()) {
            debug_assert_eq!(w.len(), offset);
            operation.write(w, refs.as_deref_mut(), encoding, unit_offsets, &offsets)?;
        }
        debug_assert_eq!(w.len(), offset);
        Ok(())
    }
}

/// A single DWARF operation.
//
// This type is intentionally not public so that we can change the
// representation of expressions as needed.
//
// Variants are listed in the order they appear in Section 2.5.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Operation {
    /// Raw bytecode.
    ///
    /// Does not support references.
    Raw(Vec<u8>),
    /// An operation that has no explicit operands.
    ///
    /// Represents:
    /// - `DW_OP_drop`, `DW_OP_swap`, `DW_OP_rot`
    /// - `DW_OP_push_object_address`, `DW_OP_form_tls_address`, `DW_OP_call_frame_cfa`
    /// - `DW_OP_abs`, `DW_OP_and`, `DW_OP_div`, `DW_OP_minus`, `DW_OP_mod`, `DW_OP_mul`,
    ///   `DW_OP_neg`, `DW_OP_not`, `DW_OP_or`, `DW_OP_plus`, `DW_OP_shl`, `DW_OP_shr`,
    ///   `DW_OP_shra`, `DW_OP_xor`
    /// - `DW_OP_le`, `DW_OP_ge`, `DW_OP_eq`, `DW_OP_lt`, `DW_OP_gt`, `DW_OP_ne`
    /// - `DW_OP_nop`
    /// - `DW_OP_stack_value`
    Simple(DwOp),
    /// Relocate the address if needed, and push it on the stack.
    ///
    /// Represents `DW_OP_addr`.
    Address(Address),
    /// Push an unsigned constant value on the stack.
    ///
    /// Represents `DW_OP_constu`.
    UnsignedConstant(u64),
    /// Push a signed constant value on the stack.
    ///
    /// Represents `DW_OP_consts`.
    SignedConstant(i64),
    /* TODO: requires .debug_addr write support
    /// Read the address at the given index in `.debug_addr, relocate the address if needed,
    /// and push it on the stack.
    ///
    /// Represents `DW_OP_addrx`.
    AddressIndex(DebugAddrIndex<Offset>),
    /// Read the address at the given index in `.debug_addr, and push it on the stack.
    /// Do not relocate the address.
    ///
    /// Represents `DW_OP_constx`.
    ConstantIndex(DebugAddrIndex<Offset>),
    */
    /// Interpret the value bytes as a constant of a given type, and push it on the stack.
    ///
    /// Represents `DW_OP_const_type`.
    ConstantType(UnitEntryId, Box<[u8]>),
    /// Compute the frame base (using `DW_AT_frame_base`), add the
    /// given offset, and then push the resulting sum on the stack.
    ///
    /// Represents `DW_OP_fbreg`.
    FrameOffset(i64),
    /// Find the contents of the given register, add the offset, and then
    /// push the resulting sum on the stack.
    ///
    /// Represents `DW_OP_bregx`.
    RegisterOffset(Register, i64),
    /// Interpret the contents of the given register as a value of the given type,
    /// and push it on the stack.
    ///
    /// Represents `DW_OP_regval_type`.
    RegisterType(Register, UnitEntryId),
    /// Copy the item at a stack index and push it on top of the stack.
    ///
    /// Represents `DW_OP_pick`, `DW_OP_dup`, and `DW_OP_over`.
    Pick(u8),
    /// Pop the topmost value of the stack, dereference it, and push the
    /// resulting value.
    ///
    /// Represents `DW_OP_deref` and `DW_OP_xderef`.
    Deref {
        /// True if the dereference operation takes an address space
        /// argument from the stack; false otherwise.
        space: bool,
    },
    /// Pop the topmost value of the stack, dereference it to obtain a value
    /// of the given size, and push the resulting value.
    ///
    /// Represents `DW_OP_deref_size` and `DW_OP_xderef_size`.
    DerefSize {
        /// True if the dereference operation takes an address space
        /// argument from the stack; false otherwise.
        space: bool,
        /// The size of the data to dereference.
        size: u8,
    },
    /// Pop the topmost value of the stack, dereference it to obtain a value
    /// of the given type, and push the resulting value.
    ///
    /// Represents `DW_OP_deref_type` and `DW_OP_xderef_type`.
    DerefType {
        /// True if the dereference operation takes an address space
        /// argument from the stack; false otherwise.
        space: bool,
        /// The size of the data to dereference.
        size: u8,
        /// The DIE of the base type, or `None` for the generic type.
        base: UnitEntryId,
    },
    /// Add an unsigned constant to the topmost value on the stack.
    ///
    /// Represents `DW_OP_plus_uconst`.
    PlusConstant(u64),
    /// Unconditional branch to the target location.
    ///
    /// The value is the index within the expression of the operation to branch to.
    /// This will be converted to a relative offset when writing.
    ///
    /// Represents `DW_OP_skip`.
    Skip(usize),
    /// Branch to the target location if the top of stack is nonzero.
    ///
    /// The value is the index within the expression of the operation to branch to.
    /// This will be converted to a relative offset when writing.
    ///
    /// Represents `DW_OP_bra`.
    Branch(usize),
    /// Evaluate a DWARF expression as a subroutine.
    ///
    /// The expression comes from the `DW_AT_location` attribute of the indicated DIE.
    ///
    /// Represents `DW_OP_call4`.
    Call(UnitEntryId),
    /// Evaluate an external DWARF expression as a subroutine.
    ///
    /// The expression comes from the `DW_AT_location` attribute of the indicated DIE,
    /// which may be in another compilation unit or shared object.
    ///
    /// Represents `DW_OP_call_ref`.
    CallRef(Reference),
    /// Pop the top stack entry, convert it to a different type, and push it on the stack.
    ///
    /// Represents `DW_OP_convert`.
    Convert(Option<UnitEntryId>),
    /// Pop the top stack entry, reinterpret the bits in its value as a different type,
    /// and push it on the stack.
    ///
    /// Represents `DW_OP_reinterpret`.
    Reinterpret(Option<UnitEntryId>),
    /// Evaluate an expression at the entry to the current subprogram, and push it on the stack.
    ///
    /// Represents `DW_OP_entry_value`.
    EntryValue(Expression),
    // FIXME: EntryRegister
    /// Indicate that this piece's location is in the given register.
    ///
    /// Completes the piece or expression.
    ///
    /// Represents `DW_OP_regx`.
    Register(Register),
    /// The object has no location, but has a known constant value.
    ///
    /// Completes the piece or expression.
    ///
    /// Represents `DW_OP_implicit_value`.
    ImplicitValue(Box<[u8]>),
    /// The object is a pointer to a value which has no actual location, such as
    /// an implicit value or a stack value.
    ///
    /// Completes the piece or expression.
    ///
    /// Represents `DW_OP_implicit_pointer`.
    ImplicitPointer {
        /// The DIE of the value that this is an implicit pointer into.
        entry: Reference,
        /// The byte offset into the value that the implicit pointer points to.
        byte_offset: i64,
    },
    /// Terminate a piece.
    ///
    /// Represents `DW_OP_piece`.
    Piece {
        /// The size of this piece in bytes.
        size_in_bytes: u64,
    },
    /// Terminate a piece with a size in bits.
    ///
    /// Represents `DW_OP_bit_piece`.
    BitPiece {
        /// The size of this piece in bits.
        size_in_bits: u64,
        /// The bit offset of this piece.
        bit_offset: u64,
    },
    /// This represents a parameter that was optimized out.
    ///
    /// The entry is the definition of the parameter, and is matched to
    /// the `DW_TAG_GNU_call_site_parameter` in the caller that also
    /// points to the same definition of the parameter.
    ///
    /// Represents `DW_OP_GNU_parameter_ref`.
    ParameterRef(UnitEntryId),
    /// The index of a local in the currently executing function.
    ///
    /// Represents `DW_OP_WASM_location 0x00`.
    WasmLocal(u32),
    /// The index of a global.
    ///
    /// Represents `DW_OP_WASM_location 0x01`.
    WasmGlobal(u32),
    /// The index of an item on the operand stack.
    ///
    /// Represents `DW_OP_WASM_location 0x02`.
    WasmStack(u32),
}

impl Operation {
    fn size(&self, encoding: Encoding, unit_offsets: Option<&UnitOffsets>) -> Result<usize> {
        let base_size = |entry| match unit_offsets {
            Some(offsets) => offsets
                .unit_offset(entry)
                .map(uleb128_size)
                .ok_or(Error::UnsupportedExpressionForwardReference),
            None => Err(Error::UnsupportedCfiExpressionReference),
        };
        Ok(1 + match *self {
            Operation::Raw(ref bytecode) => return Ok(bytecode.len()),
            Operation::Simple(_) => 0,
            Operation::Address(_) => encoding.address_size as usize,
            Operation::UnsignedConstant(value) => {
                if value < 32 {
                    0
                } else {
                    uleb128_size(value)
                }
            }
            Operation::SignedConstant(value) => sleb128_size(value),
            Operation::ConstantType(base, ref value) => base_size(base)? + 1 + value.len(),
            Operation::FrameOffset(offset) => sleb128_size(offset),
            Operation::RegisterOffset(register, offset) => {
                if register.0 < 32 {
                    sleb128_size(offset)
                } else {
                    uleb128_size(register.0.into()) + sleb128_size(offset)
                }
            }
            Operation::RegisterType(register, base) => {
                uleb128_size(register.0.into()) + base_size(base)?
            }
            Operation::Pick(index) => {
                if index > 1 {
                    1
                } else {
                    0
                }
            }
            Operation::Deref { .. } => 0,
            Operation::DerefSize { .. } => 1,
            Operation::DerefType { base, .. } => 1 + base_size(base)?,
            Operation::PlusConstant(value) => uleb128_size(value),
            Operation::Skip(_) => 2,
            Operation::Branch(_) => 2,
            Operation::Call(_) => 4,
            Operation::CallRef(_) => encoding.format.word_size() as usize,
            Operation::Convert(base) => match base {
                Some(base) => base_size(base)?,
                None => 1,
            },
            Operation::Reinterpret(base) => match base {
                Some(base) => base_size(base)?,
                None => 1,
            },
            Operation::EntryValue(ref expression) => {
                let length = expression.size(encoding, unit_offsets)?;
                uleb128_size(length as u64) + length
            }
            Operation::Register(register) => {
                if register.0 < 32 {
                    0
                } else {
                    uleb128_size(register.0.into())
                }
            }
            Operation::ImplicitValue(ref data) => uleb128_size(data.len() as u64) + data.len(),
            Operation::ImplicitPointer { byte_offset, .. } => {
                let size = if encoding.version == 2 {
                    encoding.address_size
                } else {
                    encoding.format.word_size()
                };
                size as usize + sleb128_size(byte_offset)
            }
            Operation::Piece { size_in_bytes } => uleb128_size(size_in_bytes),
            Operation::BitPiece {
                size_in_bits,
                bit_offset,
            } => uleb128_size(size_in_bits) + uleb128_size(bit_offset),
            Operation::ParameterRef(_) => 4,
            Operation::WasmLocal(index)
            | Operation::WasmGlobal(index)
            | Operation::WasmStack(index) => 1 + uleb128_size(index.into()),
        })
    }

    pub(crate) fn write<W: Writer>(
        &self,
        w: &mut W,
        refs: Option<&mut Vec<DebugInfoReference>>,
        encoding: Encoding,
        unit_offsets: Option<&UnitOffsets>,
        offsets: &[usize],
    ) -> Result<()> {
        let entry_offset = |entry| match unit_offsets {
            Some(offsets) => offsets
                .unit_offset(entry)
                .ok_or(Error::UnsupportedExpressionForwardReference),
            None => Err(Error::UnsupportedCfiExpressionReference),
        };
        match *self {
            Operation::Raw(ref bytecode) => w.write(bytecode)?,
            Operation::Simple(opcode) => w.write_u8(opcode.0)?,
            Operation::Address(address) => {
                w.write_u8(constants::DW_OP_addr.0)?;
                w.write_address(address, encoding.address_size)?;
            }
            Operation::UnsignedConstant(value) => {
                if value < 32 {
                    w.write_u8(constants::DW_OP_lit0.0 + value as u8)?;
                } else {
                    w.write_u8(constants::DW_OP_constu.0)?;
                    w.write_uleb128(value)?;
                }
            }
            Operation::SignedConstant(value) => {
                w.write_u8(constants::DW_OP_consts.0)?;
                w.write_sleb128(value)?;
            }
            Operation::ConstantType(base, ref value) => {
                if encoding.version >= 5 {
                    w.write_u8(constants::DW_OP_const_type.0)?;
                } else {
                    w.write_u8(constants::DW_OP_GNU_const_type.0)?;
                }
                w.write_uleb128(entry_offset(base)?)?;
                w.write_udata(value.len() as u64, 1)?;
                w.write(value)?;
            }
            Operation::FrameOffset(offset) => {
                w.write_u8(constants::DW_OP_fbreg.0)?;
                w.write_sleb128(offset)?;
            }
            Operation::RegisterOffset(register, offset) => {
                if register.0 < 32 {
                    w.write_u8(constants::DW_OP_breg0.0 + register.0 as u8)?;
                } else {
                    w.write_u8(constants::DW_OP_bregx.0)?;
                    w.write_uleb128(register.0.into())?;
                }
                w.write_sleb128(offset)?;
            }
            Operation::RegisterType(register, base) => {
                if encoding.version >= 5 {
                    w.write_u8(constants::DW_OP_regval_type.0)?;
                } else {
                    w.write_u8(constants::DW_OP_GNU_regval_type.0)?;
                }
                w.write_uleb128(register.0.into())?;
                w.write_uleb128(entry_offset(base)?)?;
            }
            Operation::Pick(index) => match index {
                0 => w.write_u8(constants::DW_OP_dup.0)?,
                1 => w.write_u8(constants::DW_OP_over.0)?,
                _ => {
                    w.write_u8(constants::DW_OP_pick.0)?;
                    w.write_u8(index)?;
                }
            },
            Operation::Deref { space } => {
                if space {
                    w.write_u8(constants::DW_OP_xderef.0)?;
                } else {
                    w.write_u8(constants::DW_OP_deref.0)?;
                }
            }
            Operation::DerefSize { space, size } => {
                if space {
                    w.write_u8(constants::DW_OP_xderef_size.0)?;
                } else {
                    w.write_u8(constants::DW_OP_deref_size.0)?;
                }
                w.write_u8(size)?;
            }
            Operation::DerefType { space, size, base } => {
                if space {
                    w.write_u8(constants::DW_OP_xderef_type.0)?;
                } else {
                    if encoding.version >= 5 {
                        w.write_u8(constants::DW_OP_deref_type.0)?;
                    } else {
                        w.write_u8(constants::DW_OP_GNU_deref_type.0)?;
                    }
                }
                w.write_u8(size)?;
                w.write_uleb128(entry_offset(base)?)?;
            }
            Operation::PlusConstant(value) => {
                w.write_u8(constants::DW_OP_plus_uconst.0)?;
                w.write_uleb128(value)?;
            }
            Operation::Skip(target) => {
                w.write_u8(constants::DW_OP_skip.0)?;
                let offset = offsets[target] as i64 - (w.len() as i64 + 2);
                w.write_sdata(offset, 2)?;
            }
            Operation::Branch(target) => {
                w.write_u8(constants::DW_OP_bra.0)?;
                let offset = offsets[target] as i64 - (w.len() as i64 + 2);
                w.write_sdata(offset, 2)?;
            }
            Operation::Call(entry) => {
                w.write_u8(constants::DW_OP_call4.0)?;
                // TODO: this probably won't work in practice, because we may
                // only know the offsets of base type DIEs at this point.
                w.write_udata(entry_offset(entry)?, 4)?;
            }
            Operation::CallRef(entry) => {
                w.write_u8(constants::DW_OP_call_ref.0)?;
                let size = encoding.format.word_size();
                match entry {
                    Reference::Symbol(symbol) => w.write_reference(symbol, size)?,
                    Reference::Entry(unit, entry) => {
                        let refs = refs.ok_or(Error::InvalidReference)?;
                        refs.push(DebugInfoReference {
                            offset: w.len(),
                            unit,
                            entry,
                            size,
                        });
                        w.write_udata(0, size)?;
                    }
                }
            }
            Operation::Convert(base) => {
                if encoding.version >= 5 {
                    w.write_u8(constants::DW_OP_convert.0)?;
                } else {
                    w.write_u8(constants::DW_OP_GNU_convert.0)?;
                }
                match base {
                    Some(base) => w.write_uleb128(entry_offset(base)?)?,
                    None => w.write_u8(0)?,
                }
            }
            Operation::Reinterpret(base) => {
                if encoding.version >= 5 {
                    w.write_u8(constants::DW_OP_reinterpret.0)?;
                } else {
                    w.write_u8(constants::DW_OP_GNU_reinterpret.0)?;
                }
                match base {
                    Some(base) => w.write_uleb128(entry_offset(base)?)?,
                    None => w.write_u8(0)?,
                }
            }
            Operation::EntryValue(ref expression) => {
                if encoding.version >= 5 {
                    w.write_u8(constants::DW_OP_entry_value.0)?;
                } else {
                    w.write_u8(constants::DW_OP_GNU_entry_value.0)?;
                }
                let length = expression.size(encoding, unit_offsets)?;
                w.write_uleb128(length as u64)?;
                expression.write(w, refs, encoding, unit_offsets)?;
            }
            Operation::Register(register) => {
                if register.0 < 32 {
                    w.write_u8(constants::DW_OP_reg0.0 + register.0 as u8)?;
                } else {
                    w.write_u8(constants::DW_OP_regx.0)?;
                    w.write_uleb128(register.0.into())?;
                }
            }
            Operation::ImplicitValue(ref data) => {
                w.write_u8(constants::DW_OP_implicit_value.0)?;
                w.write_uleb128(data.len() as u64)?;
                w.write(data)?;
            }
            Operation::ImplicitPointer { entry, byte_offset } => {
                if encoding.version >= 5 {
                    w.write_u8(constants::DW_OP_implicit_pointer.0)?;
                } else {
                    w.write_u8(constants::DW_OP_GNU_implicit_pointer.0)?;
                }
                let size = if encoding.version == 2 {
                    encoding.address_size
                } else {
                    encoding.format.word_size()
                };
                match entry {
                    Reference::Symbol(symbol) => {
                        w.write_reference(symbol, size)?;
                    }
                    Reference::Entry(unit, entry) => {
                        let refs = refs.ok_or(Error::InvalidReference)?;
                        refs.push(DebugInfoReference {
                            offset: w.len(),
                            unit,
                            entry,
                            size,
                        });
                        w.write_udata(0, size)?;
                    }
                }
                w.write_sleb128(byte_offset)?;
            }
            Operation::Piece { size_in_bytes } => {
                w.write_u8(constants::DW_OP_piece.0)?;
                w.write_uleb128(size_in_bytes)?;
            }
            Operation::BitPiece {
                size_in_bits,
                bit_offset,
            } => {
                w.write_u8(constants::DW_OP_bit_piece.0)?;
                w.write_uleb128(size_in_bits)?;
                w.write_uleb128(bit_offset)?;
            }
            Operation::ParameterRef(entry) => {
                w.write_u8(constants::DW_OP_GNU_parameter_ref.0)?;
                w.write_udata(entry_offset(entry)?, 4)?;
            }
            Operation::WasmLocal(index) => {
                w.write(&[constants::DW_OP_WASM_location.0, 0])?;
                w.write_uleb128(index.into())?;
            }
            Operation::WasmGlobal(index) => {
                w.write(&[constants::DW_OP_WASM_location.0, 1])?;
                w.write_uleb128(index.into())?;
            }
            Operation::WasmStack(index) => {
                w.write(&[constants::DW_OP_WASM_location.0, 2])?;
                w.write_uleb128(index.into())?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "read")]
pub(crate) mod convert {
    use super::*;
    use crate::common::UnitSectionOffset;
    use crate::read::{self, Reader};
    use crate::write::{ConvertError, ConvertResult, UnitId};
    use std::collections::HashMap;

    impl Expression {
        /// Create an expression from the input expression.
        pub fn from<R: Reader<Offset = usize>>(
            from_expression: read::Expression<R>,
            encoding: Encoding,
            dwarf: Option<&read::Dwarf<R>>,
            unit: Option<&read::Unit<R>>,
            entry_ids: Option<&HashMap<UnitSectionOffset, (UnitId, UnitEntryId)>>,
            convert_address: &dyn Fn(u64) -> Option<Address>,
        ) -> ConvertResult<Expression> {
            let convert_unit_offset = |offset: read::UnitOffset| -> ConvertResult<_> {
                let entry_ids = entry_ids.ok_or(ConvertError::UnsupportedOperation)?;
                let unit = unit.ok_or(ConvertError::UnsupportedOperation)?;
                let id = entry_ids
                    .get(&offset.to_unit_section_offset(unit))
                    .ok_or(ConvertError::InvalidUnitRef)?;
                Ok(id.1)
            };
            let convert_debug_info_offset = |offset| -> ConvertResult<_> {
                // TODO: support relocations
                let entry_ids = entry_ids.ok_or(ConvertError::UnsupportedOperation)?;
                let id = entry_ids
                    .get(&UnitSectionOffset::DebugInfoOffset(offset))
                    .ok_or(ConvertError::InvalidDebugInfoRef)?;
                Ok(Reference::Entry(id.0, id.1))
            };

            // Calculate offsets for use in branch/skip operations.
            let mut offsets = Vec::new();
            let mut offset = 0;
            let mut from_operations = from_expression.clone().operations(encoding);
            while from_operations.next()?.is_some() {
                offsets.push(offset);
                offset = from_operations.offset_from(&from_expression);
            }
            offsets.push(from_expression.0.len());

            let mut from_operations = from_expression.clone().operations(encoding);
            let mut operations = Vec::new();
            while let Some(from_operation) = from_operations.next()? {
                let operation = match from_operation {
                    read::Operation::Deref {
                        base_type,
                        size,
                        space,
                    } => {
                        if base_type.0 != 0 {
                            let base = convert_unit_offset(base_type)?;
                            Operation::DerefType { space, size, base }
                        } else if size != encoding.address_size {
                            Operation::DerefSize { space, size }
                        } else {
                            Operation::Deref { space }
                        }
                    }
                    read::Operation::Drop => Operation::Simple(constants::DW_OP_drop),
                    read::Operation::Pick { index } => Operation::Pick(index),
                    read::Operation::Swap => Operation::Simple(constants::DW_OP_swap),
                    read::Operation::Rot => Operation::Simple(constants::DW_OP_rot),
                    read::Operation::Abs => Operation::Simple(constants::DW_OP_abs),
                    read::Operation::And => Operation::Simple(constants::DW_OP_and),
                    read::Operation::Div => Operation::Simple(constants::DW_OP_div),
                    read::Operation::Minus => Operation::Simple(constants::DW_OP_minus),
                    read::Operation::Mod => Operation::Simple(constants::DW_OP_mod),
                    read::Operation::Mul => Operation::Simple(constants::DW_OP_mul),
                    read::Operation::Neg => Operation::Simple(constants::DW_OP_neg),
                    read::Operation::Not => Operation::Simple(constants::DW_OP_not),
                    read::Operation::Or => Operation::Simple(constants::DW_OP_or),
                    read::Operation::Plus => Operation::Simple(constants::DW_OP_plus),
                    read::Operation::PlusConstant { value } => Operation::PlusConstant(value),
                    read::Operation::Shl => Operation::Simple(constants::DW_OP_shl),
                    read::Operation::Shr => Operation::Simple(constants::DW_OP_shr),
                    read::Operation::Shra => Operation::Simple(constants::DW_OP_shra),
                    read::Operation::Xor => Operation::Simple(constants::DW_OP_xor),
                    read::Operation::Eq => Operation::Simple(constants::DW_OP_eq),
                    read::Operation::Ge => Operation::Simple(constants::DW_OP_ge),
                    read::Operation::Gt => Operation::Simple(constants::DW_OP_gt),
                    read::Operation::Le => Operation::Simple(constants::DW_OP_le),
                    read::Operation::Lt => Operation::Simple(constants::DW_OP_lt),
                    read::Operation::Ne => Operation::Simple(constants::DW_OP_ne),
                    read::Operation::Bra { target } => {
                        let offset = from_operations
                            .offset_from(&from_expression)
                            .wrapping_add(i64::from(target) as usize);
                        let index = offsets
                            .binary_search(&offset)
                            .map_err(|_| ConvertError::InvalidBranchTarget)?;
                        Operation::Branch(index)
                    }
                    read::Operation::Skip { target } => {
                        let offset = from_operations
                            .offset_from(&from_expression)
                            .wrapping_add(i64::from(target) as usize);
                        let index = offsets
                            .binary_search(&offset)
                            .map_err(|_| ConvertError::InvalidBranchTarget)?;
                        Operation::Skip(index)
                    }
                    read::Operation::UnsignedConstant { value } => {
                        Operation::UnsignedConstant(value)
                    }
                    read::Operation::SignedConstant { value } => Operation::SignedConstant(value),
                    read::Operation::Register { register } => Operation::Register(register),
                    read::Operation::RegisterOffset {
                        register,
                        offset,
                        base_type,
                    } => {
                        if base_type.0 != 0 {
                            Operation::RegisterType(register, convert_unit_offset(base_type)?)
                        } else {
                            Operation::RegisterOffset(register, offset)
                        }
                    }
                    read::Operation::FrameOffset { offset } => Operation::FrameOffset(offset),
                    read::Operation::Nop => Operation::Simple(constants::DW_OP_nop),
                    read::Operation::PushObjectAddress => {
                        Operation::Simple(constants::DW_OP_push_object_address)
                    }
                    read::Operation::Call { offset } => match offset {
                        read::DieReference::UnitRef(offset) => {
                            Operation::Call(convert_unit_offset(offset)?)
                        }
                        read::DieReference::DebugInfoRef(offset) => {
                            Operation::CallRef(convert_debug_info_offset(offset)?)
                        }
                    },
                    read::Operation::TLS => Operation::Simple(constants::DW_OP_form_tls_address),
                    read::Operation::CallFrameCFA => {
                        Operation::Simple(constants::DW_OP_call_frame_cfa)
                    }
                    read::Operation::Piece {
                        size_in_bits,
                        bit_offset: None,
                    } => Operation::Piece {
                        size_in_bytes: size_in_bits / 8,
                    },
                    read::Operation::Piece {
                        size_in_bits,
                        bit_offset: Some(bit_offset),
                    } => Operation::BitPiece {
                        size_in_bits,
                        bit_offset,
                    },
                    read::Operation::ImplicitValue { data } => {
                        Operation::ImplicitValue(data.to_slice()?.into_owned().into())
                    }
                    read::Operation::StackValue => Operation::Simple(constants::DW_OP_stack_value),
                    read::Operation::ImplicitPointer { value, byte_offset } => {
                        let entry = convert_debug_info_offset(value)?;
                        Operation::ImplicitPointer { entry, byte_offset }
                    }
                    read::Operation::EntryValue { expression } => {
                        let expression = Expression::from(
                            read::Expression(expression),
                            encoding,
                            dwarf,
                            unit,
                            entry_ids,
                            convert_address,
                        )?;
                        Operation::EntryValue(expression)
                    }
                    read::Operation::ParameterRef { offset } => {
                        let entry = convert_unit_offset(offset)?;
                        Operation::ParameterRef(entry)
                    }
                    read::Operation::Address { address } => {
                        let address =
                            convert_address(address).ok_or(ConvertError::InvalidAddress)?;
                        Operation::Address(address)
                    }
                    read::Operation::AddressIndex { index } => {
                        let dwarf = dwarf.ok_or(ConvertError::UnsupportedOperation)?;
                        let unit = unit.ok_or(ConvertError::UnsupportedOperation)?;
                        let val = dwarf.address(unit, index)?;
                        let address = convert_address(val).ok_or(ConvertError::InvalidAddress)?;
                        Operation::Address(address)
                    }
                    read::Operation::ConstantIndex { index } => {
                        let dwarf = dwarf.ok_or(ConvertError::UnsupportedOperation)?;
                        let unit = unit.ok_or(ConvertError::UnsupportedOperation)?;
                        let val = dwarf.address(unit, index)?;
                        Operation::UnsignedConstant(val)
                    }
                    read::Operation::TypedLiteral { base_type, value } => {
                        let entry = convert_unit_offset(base_type)?;
                        Operation::ConstantType(entry, value.to_slice()?.into_owned().into())
                    }
                    read::Operation::Convert { base_type } => {
                        if base_type.0 == 0 {
                            Operation::Convert(None)
                        } else {
                            let entry = convert_unit_offset(base_type)?;
                            Operation::Convert(Some(entry))
                        }
                    }
                    read::Operation::Reinterpret { base_type } => {
                        if base_type.0 == 0 {
                            Operation::Reinterpret(None)
                        } else {
                            let entry = convert_unit_offset(base_type)?;
                            Operation::Reinterpret(Some(entry))
                        }
                    }
                    read::Operation::WasmLocal { index } => Operation::WasmLocal(index),
                    read::Operation::WasmGlobal { index } => Operation::WasmGlobal(index),
                    read::Operation::WasmStack { index } => Operation::WasmStack(index),
                };
                operations.push(operation);
            }
            Ok(Expression { operations })
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/gimli/unit/src/write/op/tests_1061.rs"));
