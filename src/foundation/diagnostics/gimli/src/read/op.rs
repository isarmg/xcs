//! Functions for parsing and evaluating DWARF expressions.

#[cfg(feature = "read")]
use alloc::vec::Vec;
use core::mem;

use super::util::{ArrayLike, ArrayVec};
use crate::common::{DebugAddrIndex, DebugInfoOffset, Encoding, Register};
use crate::constants;
use crate::read::{Error, Reader, ReaderOffset, Result, StoreOnHeap, UnitOffset, Value, ValueType};

/// A reference to a DIE, either relative to the current CU or
/// relative to the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DieReference<T = usize> {
    /// A CU-relative reference.
    UnitRef(UnitOffset<T>),
    /// A section-relative reference.
    DebugInfoRef(DebugInfoOffset<T>),
}

/// A single decoded DWARF expression operation.
///
/// DWARF expression evaluation is done in two parts: first the raw
/// bytes of the next part of the expression are decoded; and then the
/// decoded operation is evaluated.  This approach lets other
/// consumers inspect the DWARF expression without reimplementing the
/// decoding operation.
///
/// Multiple DWARF opcodes may decode into a single `Operation`.  For
/// example, both `DW_OP_deref` and `DW_OP_xderef` are represented
/// using `Operation::Deref`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation<R, Offset = <R as Reader>::Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// Dereference the topmost value of the stack.
    Deref {
        /// The DIE of the base type or 0 to indicate the generic type
        base_type: UnitOffset<Offset>,
        /// The size of the data to dereference.
        size: u8,
        /// True if the dereference operation takes an address space
        /// argument from the stack; false otherwise.
        space: bool,
    },
    /// Drop an item from the stack.
    Drop,
    /// Pick an item from the stack and push it on top of the stack.
    /// This operation handles `DW_OP_pick`, `DW_OP_dup`, and
    /// `DW_OP_over`.
    Pick {
        /// The index, from the top of the stack, of the item to copy.
        index: u8,
    },
    /// Swap the top two stack items.
    Swap,
    /// Rotate the top three stack items.
    Rot,
    /// Take the absolute value of the top of the stack.
    Abs,
    /// Bitwise `and` of the top two values on the stack.
    And,
    /// Divide the top two values on the stack.
    Div,
    /// Subtract the top two values on the stack.
    Minus,
    /// Modulus of the top two values on the stack.
    Mod,
    /// Multiply the top two values on the stack.
    Mul,
    /// Negate the top of the stack.
    Neg,
    /// Bitwise `not` of the top of the stack.
    Not,
    /// Bitwise `or` of the top two values on the stack.
    Or,
    /// Add the top two values on the stack.
    Plus,
    /// Add a constant to the topmost value on the stack.
    PlusConstant {
        /// The value to add.
        value: u64,
    },
    /// Logical left shift of the 2nd value on the stack by the number
    /// of bits given by the topmost value on the stack.
    Shl,
    /// Right shift of the 2nd value on the stack by the number of
    /// bits given by the topmost value on the stack.
    Shr,
    /// Arithmetic left shift of the 2nd value on the stack by the
    /// number of bits given by the topmost value on the stack.
    Shra,
    /// Bitwise `xor` of the top two values on the stack.
    Xor,
    /// Branch to the target location if the top of stack is nonzero.
    Bra {
        /// The relative offset to the target bytecode.
        target: i16,
    },
    /// Compare the top two stack values for equality.
    Eq,
    /// Compare the top two stack values using `>=`.
    Ge,
    /// Compare the top two stack values using `>`.
    Gt,
    /// Compare the top two stack values using `<=`.
    Le,
    /// Compare the top two stack values using `<`.
    Lt,
    /// Compare the top two stack values using `!=`.
    Ne,
    /// Unconditional branch to the target location.
    Skip {
        /// The relative offset to the target bytecode.
        target: i16,
    },
    /// Push an unsigned constant value on the stack.  This handles multiple
    /// DWARF opcodes.
    UnsignedConstant {
        /// The value to push.
        value: u64,
    },
    /// Push a signed constant value on the stack.  This handles multiple
    /// DWARF opcodes.
    SignedConstant {
        /// The value to push.
        value: i64,
    },
    /// Indicate that this piece's location is in the given register.
    ///
    /// Completes the piece or expression.
    Register {
        /// The register number.
        register: Register,
    },
    /// Find the value of the given register, add the offset, and then
    /// push the resulting sum on the stack.
    RegisterOffset {
        /// The register number.
        register: Register,
        /// The offset to add.
        offset: i64,
        /// The DIE of the base type or 0 to indicate the generic type
        base_type: UnitOffset<Offset>,
    },
    /// Compute the frame base (using `DW_AT_frame_base`), add the
    /// given offset, and then push the resulting sum on the stack.
    FrameOffset {
        /// The offset to add.
        offset: i64,
    },
    /// No operation.
    Nop,
    /// Push the object address on the stack.
    PushObjectAddress,
    /// Evaluate a DWARF expression as a subroutine.  The expression
    /// comes from the `DW_AT_location` attribute of the indicated
    /// DIE.
    Call {
        /// The DIE to use.
        offset: DieReference<Offset>,
    },
    /// Compute the address of a thread-local variable and push it on
    /// the stack.
    TLS,
    /// Compute the call frame CFA and push it on the stack.
    CallFrameCFA,
    /// Terminate a piece.
    Piece {
        /// The size of this piece in bits.
        size_in_bits: u64,
        /// The bit offset of this piece.  If `None`, then this piece
        /// was specified using `DW_OP_piece` and should start at the
        /// next byte boundary.
        bit_offset: Option<u64>,
    },
    /// The object has no location, but has a known constant value.
    ///
    /// Represents `DW_OP_implicit_value`.
    /// Completes the piece or expression.
    ImplicitValue {
        /// The implicit value to use.
        data: R,
    },
    /// The object has no location, but its value is at the top of the stack.
    ///
    /// Represents `DW_OP_stack_value`.
    /// Completes the piece or expression.
    StackValue,
    /// The object is a pointer to a value which has no actual location,
    /// such as an implicit value or a stack value.
    ///
    /// Represents `DW_OP_implicit_pointer`.
    /// Completes the piece or expression.
    ImplicitPointer {
        /// The `.debug_info` offset of the value that this is an implicit pointer into.
        value: DebugInfoOffset<Offset>,
        /// The byte offset into the value that the implicit pointer points to.
        byte_offset: i64,
    },
    /// Evaluate an expression at the entry to the current subprogram, and push it on the stack.
    ///
    /// Represents `DW_OP_entry_value`.
    EntryValue {
        /// The expression to be evaluated.
        expression: R,
    },
    /// This represents a parameter that was optimized out.
    ///
    /// The offset points to the definition of the parameter, and is
    /// matched to the `DW_TAG_GNU_call_site_parameter` in the caller that also
    /// points to the same definition of the parameter.
    ///
    /// Represents `DW_OP_GNU_parameter_ref`.
    ParameterRef {
        /// The DIE to use.
        offset: UnitOffset<Offset>,
    },
    /// Relocate the address if needed, and push it on the stack.
    ///
    /// Represents `DW_OP_addr`.
    Address {
        /// The offset to add.
        address: u64,
    },
    /// Read the address at the given index in `.debug_addr, relocate the address if needed,
    /// and push it on the stack.
    ///
    /// Represents `DW_OP_addrx`.
    AddressIndex {
        /// The index of the address in `.debug_addr`.
        index: DebugAddrIndex<Offset>,
    },
    /// Read the address at the given index in `.debug_addr, and push it on the stack.
    /// Do not relocate the address.
    ///
    /// Represents `DW_OP_constx`.
    ConstantIndex {
        /// The index of the address in `.debug_addr`.
        index: DebugAddrIndex<Offset>,
    },
    /// Interpret the value bytes as a constant of a given type, and push it on the stack.
    ///
    /// Represents `DW_OP_const_type`.
    TypedLiteral {
        /// The DIE of the base type.
        base_type: UnitOffset<Offset>,
        /// The value bytes.
        value: R,
    },
    /// Pop the top stack entry, convert it to a different type, and push it on the stack.
    ///
    /// Represents `DW_OP_convert`.
    Convert {
        /// The DIE of the base type.
        base_type: UnitOffset<Offset>,
    },
    /// Pop the top stack entry, reinterpret the bits in its value as a different type,
    /// and push it on the stack.
    ///
    /// Represents `DW_OP_reinterpret`.
    Reinterpret {
        /// The DIE of the base type.
        base_type: UnitOffset<Offset>,
    },
    /// The index of a local in the currently executing function.
    ///
    /// Represents `DW_OP_WASM_location 0x00`.
    /// Completes the piece or expression.
    WasmLocal {
        /// The index of the local.
        index: u32,
    },
    /// The index of a global.
    ///
    /// Represents `DW_OP_WASM_location 0x01` or `DW_OP_WASM_location 0x03`.
    /// Completes the piece or expression.
    WasmGlobal {
        /// The index of the global.
        index: u32,
    },
    /// The index of an item on the operand stack.
    ///
    /// Represents `DW_OP_WASM_location 0x02`.
    /// Completes the piece or expression.
    WasmStack {
        /// The index of the stack item. 0 is the bottom of the operand stack.
        index: u32,
    },
}

#[derive(Debug)]
enum OperationEvaluationResult<R: Reader> {
    Piece,
    Incomplete,
    Complete { location: Location<R> },
    Waiting(EvaluationWaiting<R>, EvaluationResult<R>),
}

/// A single location of a piece of the result of a DWARF expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Location<R, Offset = <R as Reader>::Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// The piece is empty.  Ordinarily this means the piece has been
    /// optimized away.
    Empty,
    /// The piece is found in a register.
    Register {
        /// The register number.
        register: Register,
    },
    /// The piece is found in memory.
    Address {
        /// The address.
        address: u64,
    },
    /// The piece has no location but its value is known.
    Value {
        /// The value.
        value: Value,
    },
    /// The piece is represented by some constant bytes.
    Bytes {
        /// The value.
        value: R,
    },
    /// The piece is a pointer to a value which has no actual location.
    ImplicitPointer {
        /// The `.debug_info` offset of the value that this is an implicit pointer into.
        value: DebugInfoOffset<Offset>,
        /// The byte offset into the value that the implicit pointer points to.
        byte_offset: i64,
    },
}

impl<R, Offset> Location<R, Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// Return true if the piece is empty.
    pub fn is_empty(&self) -> bool {
        matches!(*self, Location::Empty)
    }
}

/// The description of a single piece of the result of a DWARF
/// expression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece<R, Offset = <R as Reader>::Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// If given, the size of the piece in bits.  If `None`, there
    /// must be only one piece whose size is all of the object.
    pub size_in_bits: Option<u64>,
    /// If given, the bit offset of the piece within the location.
    /// If the location is a `Location::Register` or `Location::Value`,
    /// then this offset is from the least significant bit end of
    /// the register or value.
    /// If the location is a `Location::Address` then the offset uses
    /// the bit numbering and direction conventions of the language
    /// and target system.
    ///
    /// If `None`, the piece starts at the location. If the
    /// location is a register whose size is larger than the piece,
    /// then placement within the register is defined by the ABI.
    pub bit_offset: Option<u64>,
    /// Where this piece is to be found.
    pub location: Location<R, Offset>,
}

// A helper function to handle branch offsets.
fn compute_pc<R: Reader>(pc: &R, bytecode: &R, offset: i16) -> Result<R> {
    let pc_offset = pc.offset_from(bytecode);
    let new_pc_offset = pc_offset.wrapping_add(R::Offset::from_i16(offset));
    if new_pc_offset > bytecode.len() {
        Err(Error::BadBranchTarget(new_pc_offset.into_u64()))
    } else {
        let mut new_pc = bytecode.clone();
        new_pc.skip(new_pc_offset)?;
        Ok(new_pc)
    }
}

fn generic_type<O: ReaderOffset>() -> UnitOffset<O> {
    UnitOffset(O::from_u64(0).unwrap())
}

impl<R, Offset> Operation<R, Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// Parse a single DWARF expression operation.
    ///
    /// This is useful when examining a DWARF expression for reasons other
    /// than direct evaluation.
    ///
    /// `bytes` points to a the operation to decode.  It should point into
    /// the same array as `bytecode`, which should be the entire
    /// expression.
    pub fn parse(bytes: &mut R, encoding: Encoding) -> Result<Operation<R, Offset>> {
        let opcode = bytes.read_u8()?;
        let name = constants::DwOp(opcode);
        match name {
            constants::DW_OP_addr => {
                let address = bytes.read_address(encoding.address_size)?;
                Ok(Operation::Address { address })
            }
            constants::DW_OP_deref => Ok(Operation::Deref {
                base_type: generic_type(),
                size: encoding.address_size,
                space: false,
            }),
            constants::DW_OP_const1u => {
                let value = bytes.read_u8()?;
                Ok(Operation::UnsignedConstant {
                    value: u64::from(value),
                })
            }
            constants::DW_OP_const1s => {
                let value = bytes.read_i8()?;
                Ok(Operation::SignedConstant {
                    value: i64::from(value),
                })
            }
            constants::DW_OP_const2u => {
                let value = bytes.read_u16()?;
                Ok(Operation::UnsignedConstant {
                    value: u64::from(value),
                })
            }
            constants::DW_OP_const2s => {
                let value = bytes.read_i16()?;
                Ok(Operation::SignedConstant {
                    value: i64::from(value),
                })
            }
            constants::DW_OP_const4u => {
                let value = bytes.read_u32()?;
                Ok(Operation::UnsignedConstant {
                    value: u64::from(value),
                })
            }
            constants::DW_OP_const4s => {
                let value = bytes.read_i32()?;
                Ok(Operation::SignedConstant {
                    value: i64::from(value),
                })
            }
            constants::DW_OP_const8u => {
                let value = bytes.read_u64()?;
                Ok(Operation::UnsignedConstant { value })
            }
            constants::DW_OP_const8s => {
                let value = bytes.read_i64()?;
                Ok(Operation::SignedConstant { value })
            }
            constants::DW_OP_constu => {
                let value = bytes.read_uleb128()?;
                Ok(Operation::UnsignedConstant { value })
            }
            constants::DW_OP_consts => {
                let value = bytes.read_sleb128()?;
                Ok(Operation::SignedConstant { value })
            }
            constants::DW_OP_dup => Ok(Operation::Pick { index: 0 }),
            constants::DW_OP_drop => Ok(Operation::Drop),
            constants::DW_OP_over => Ok(Operation::Pick { index: 1 }),
            constants::DW_OP_pick => {
                let value = bytes.read_u8()?;
                Ok(Operation::Pick { index: value })
            }
            constants::DW_OP_swap => Ok(Operation::Swap),
            constants::DW_OP_rot => Ok(Operation::Rot),
            constants::DW_OP_xderef => Ok(Operation::Deref {
                base_type: generic_type(),
                size: encoding.address_size,
                space: true,
            }),
            constants::DW_OP_abs => Ok(Operation::Abs),
            constants::DW_OP_and => Ok(Operation::And),
            constants::DW_OP_div => Ok(Operation::Div),
            constants::DW_OP_minus => Ok(Operation::Minus),
            constants::DW_OP_mod => Ok(Operation::Mod),
            constants::DW_OP_mul => Ok(Operation::Mul),
            constants::DW_OP_neg => Ok(Operation::Neg),
            constants::DW_OP_not => Ok(Operation::Not),
            constants::DW_OP_or => Ok(Operation::Or),
            constants::DW_OP_plus => Ok(Operation::Plus),
            constants::DW_OP_plus_uconst => {
                let value = bytes.read_uleb128()?;
                Ok(Operation::PlusConstant { value })
            }
            constants::DW_OP_shl => Ok(Operation::Shl),
            constants::DW_OP_shr => Ok(Operation::Shr),
            constants::DW_OP_shra => Ok(Operation::Shra),
            constants::DW_OP_xor => Ok(Operation::Xor),
            constants::DW_OP_bra => {
                let target = bytes.read_i16()?;
                Ok(Operation::Bra { target })
            }
            constants::DW_OP_eq => Ok(Operation::Eq),
            constants::DW_OP_ge => Ok(Operation::Ge),
            constants::DW_OP_gt => Ok(Operation::Gt),
            constants::DW_OP_le => Ok(Operation::Le),
            constants::DW_OP_lt => Ok(Operation::Lt),
            constants::DW_OP_ne => Ok(Operation::Ne),
            constants::DW_OP_skip => {
                let target = bytes.read_i16()?;
                Ok(Operation::Skip { target })
            }
            constants::DW_OP_lit0
            | constants::DW_OP_lit1
            | constants::DW_OP_lit2
            | constants::DW_OP_lit3
            | constants::DW_OP_lit4
            | constants::DW_OP_lit5
            | constants::DW_OP_lit6
            | constants::DW_OP_lit7
            | constants::DW_OP_lit8
            | constants::DW_OP_lit9
            | constants::DW_OP_lit10
            | constants::DW_OP_lit11
            | constants::DW_OP_lit12
            | constants::DW_OP_lit13
            | constants::DW_OP_lit14
            | constants::DW_OP_lit15
            | constants::DW_OP_lit16
            | constants::DW_OP_lit17
            | constants::DW_OP_lit18
            | constants::DW_OP_lit19
            | constants::DW_OP_lit20
            | constants::DW_OP_lit21
            | constants::DW_OP_lit22
            | constants::DW_OP_lit23
            | constants::DW_OP_lit24
            | constants::DW_OP_lit25
            | constants::DW_OP_lit26
            | constants::DW_OP_lit27
            | constants::DW_OP_lit28
            | constants::DW_OP_lit29
            | constants::DW_OP_lit30
            | constants::DW_OP_lit31 => Ok(Operation::UnsignedConstant {
                value: (opcode - constants::DW_OP_lit0.0).into(),
            }),
            constants::DW_OP_reg0
            | constants::DW_OP_reg1
            | constants::DW_OP_reg2
            | constants::DW_OP_reg3
            | constants::DW_OP_reg4
            | constants::DW_OP_reg5
            | constants::DW_OP_reg6
            | constants::DW_OP_reg7
            | constants::DW_OP_reg8
            | constants::DW_OP_reg9
            | constants::DW_OP_reg10
            | constants::DW_OP_reg11
            | constants::DW_OP_reg12
            | constants::DW_OP_reg13
            | constants::DW_OP_reg14
            | constants::DW_OP_reg15
            | constants::DW_OP_reg16
            | constants::DW_OP_reg17
            | constants::DW_OP_reg18
            | constants::DW_OP_reg19
            | constants::DW_OP_reg20
            | constants::DW_OP_reg21
            | constants::DW_OP_reg22
            | constants::DW_OP_reg23
            | constants::DW_OP_reg24
            | constants::DW_OP_reg25
            | constants::DW_OP_reg26
            | constants::DW_OP_reg27
            | constants::DW_OP_reg28
            | constants::DW_OP_reg29
            | constants::DW_OP_reg30
            | constants::DW_OP_reg31 => Ok(Operation::Register {
                register: Register((opcode - constants::DW_OP_reg0.0).into()),
            }),
            constants::DW_OP_breg0
            | constants::DW_OP_breg1
            | constants::DW_OP_breg2
            | constants::DW_OP_breg3
            | constants::DW_OP_breg4
            | constants::DW_OP_breg5
            | constants::DW_OP_breg6
            | constants::DW_OP_breg7
            | constants::DW_OP_breg8
            | constants::DW_OP_breg9
            | constants::DW_OP_breg10
            | constants::DW_OP_breg11
            | constants::DW_OP_breg12
            | constants::DW_OP_breg13
            | constants::DW_OP_breg14
            | constants::DW_OP_breg15
            | constants::DW_OP_breg16
            | constants::DW_OP_breg17
            | constants::DW_OP_breg18
            | constants::DW_OP_breg19
            | constants::DW_OP_breg20
            | constants::DW_OP_breg21
            | constants::DW_OP_breg22
            | constants::DW_OP_breg23
            | constants::DW_OP_breg24
            | constants::DW_OP_breg25
            | constants::DW_OP_breg26
            | constants::DW_OP_breg27
            | constants::DW_OP_breg28
            | constants::DW_OP_breg29
            | constants::DW_OP_breg30
            | constants::DW_OP_breg31 => {
                let value = bytes.read_sleb128()?;
                Ok(Operation::RegisterOffset {
                    register: Register((opcode - constants::DW_OP_breg0.0).into()),
                    offset: value,
                    base_type: generic_type(),
                })
            }
            constants::DW_OP_regx => {
                let register = bytes.read_uleb128().and_then(Register::from_u64)?;
                Ok(Operation::Register { register })
            }
            constants::DW_OP_fbreg => {
                let value = bytes.read_sleb128()?;
                Ok(Operation::FrameOffset { offset: value })
            }
            constants::DW_OP_bregx => {
                let register = bytes.read_uleb128().and_then(Register::from_u64)?;
                let offset = bytes.read_sleb128()?;
                Ok(Operation::RegisterOffset {
                    register,
                    offset,
                    base_type: generic_type(),
                })
            }
            constants::DW_OP_piece => {
                let size = bytes.read_uleb128()?;
                Ok(Operation::Piece {
                    size_in_bits: 8 * size,
                    bit_offset: None,
                })
            }
            constants::DW_OP_deref_size => {
                let size = bytes.read_u8()?;
                Ok(Operation::Deref {
                    base_type: generic_type(),
                    size,
                    space: false,
                })
            }
            constants::DW_OP_xderef_size => {
                let size = bytes.read_u8()?;
                Ok(Operation::Deref {
                    base_type: generic_type(),
                    size,
                    space: true,
                })
            }
            constants::DW_OP_nop => Ok(Operation::Nop),
            constants::DW_OP_push_object_address => Ok(Operation::PushObjectAddress),
            constants::DW_OP_call2 => {
                let value = bytes.read_u16().map(R::Offset::from_u16)?;
                Ok(Operation::Call {
                    offset: DieReference::UnitRef(UnitOffset(value)),
                })
            }
            constants::DW_OP_call4 => {
                let value = bytes.read_u32().map(R::Offset::from_u32)?;
                Ok(Operation::Call {
                    offset: DieReference::UnitRef(UnitOffset(value)),
                })
            }
            constants::DW_OP_call_ref => {
                let value = bytes.read_offset(encoding.format)?;
                Ok(Operation::Call {
                    offset: DieReference::DebugInfoRef(DebugInfoOffset(value)),
                })
            }
            constants::DW_OP_form_tls_address | constants::DW_OP_GNU_push_tls_address => {
                Ok(Operation::TLS)
            }
            constants::DW_OP_call_frame_cfa => Ok(Operation::CallFrameCFA),
            constants::DW_OP_bit_piece => {
                let size = bytes.read_uleb128()?;
                let offset = bytes.read_uleb128()?;
                Ok(Operation::Piece {
                    size_in_bits: size,
                    bit_offset: Some(offset),
                })
            }
            constants::DW_OP_implicit_value => {
                let len = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                let data = bytes.split(len)?;
                Ok(Operation::ImplicitValue { data })
            }
            constants::DW_OP_stack_value => Ok(Operation::StackValue),
            constants::DW_OP_implicit_pointer | constants::DW_OP_GNU_implicit_pointer => {
                let value = if encoding.version == 2 {
                    bytes
                        .read_address(encoding.address_size)
                        .and_then(Offset::from_u64)?
                } else {
                    bytes.read_offset(encoding.format)?
                };
                let byte_offset = bytes.read_sleb128()?;
                Ok(Operation::ImplicitPointer {
                    value: DebugInfoOffset(value),
                    byte_offset,
                })
            }
            constants::DW_OP_addrx | constants::DW_OP_GNU_addr_index => {
                let index = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::AddressIndex {
                    index: DebugAddrIndex(index),
                })
            }
            constants::DW_OP_constx | constants::DW_OP_GNU_const_index => {
                let index = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::ConstantIndex {
                    index: DebugAddrIndex(index),
                })
            }
            constants::DW_OP_entry_value | constants::DW_OP_GNU_entry_value => {
                let len = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                let expression = bytes.split(len)?;
                Ok(Operation::EntryValue { expression })
            }
            constants::DW_OP_GNU_parameter_ref => {
                let value = bytes.read_u32().map(R::Offset::from_u32)?;
                Ok(Operation::ParameterRef {
                    offset: UnitOffset(value),
                })
            }
            constants::DW_OP_const_type | constants::DW_OP_GNU_const_type => {
                let base_type = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                let len = bytes.read_u8()?;
                let value = bytes.split(R::Offset::from_u8(len))?;
                Ok(Operation::TypedLiteral {
                    base_type: UnitOffset(base_type),
                    value,
                })
            }
            constants::DW_OP_regval_type | constants::DW_OP_GNU_regval_type => {
                let register = bytes.read_uleb128().and_then(Register::from_u64)?;
                let base_type = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::RegisterOffset {
                    register,
                    offset: 0,
                    base_type: UnitOffset(base_type),
                })
            }
            constants::DW_OP_deref_type | constants::DW_OP_GNU_deref_type => {
                let size = bytes.read_u8()?;
                let base_type = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::Deref {
                    base_type: UnitOffset(base_type),
                    size,
                    space: false,
                })
            }
            constants::DW_OP_xderef_type => {
                let size = bytes.read_u8()?;
                let base_type = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::Deref {
                    base_type: UnitOffset(base_type),
                    size,
                    space: true,
                })
            }
            constants::DW_OP_convert | constants::DW_OP_GNU_convert => {
                let base_type = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::Convert {
                    base_type: UnitOffset(base_type),
                })
            }
            constants::DW_OP_reinterpret | constants::DW_OP_GNU_reinterpret => {
                let base_type = bytes.read_uleb128().and_then(R::Offset::from_u64)?;
                Ok(Operation::Reinterpret {
                    base_type: UnitOffset(base_type),
                })
            }
            constants::DW_OP_WASM_location => match bytes.read_u8()? {
                0x0 => {
                    let index = bytes.read_uleb128_u32()?;
                    Ok(Operation::WasmLocal { index })
                }
                0x1 => {
                    let index = bytes.read_uleb128_u32()?;
                    Ok(Operation::WasmGlobal { index })
                }
                0x2 => {
                    let index = bytes.read_uleb128_u32()?;
                    Ok(Operation::WasmStack { index })
                }
                0x3 => {
                    let index = bytes.read_u32()?;
                    Ok(Operation::WasmGlobal { index })
                }
                _ => Err(Error::InvalidExpression(name)),
            },
            _ => Err(Error::InvalidExpression(name)),
        }
    }
}

#[derive(Debug)]
enum EvaluationState<R: Reader> {
    Start(Option<u64>),
    Ready,
    Error(Error),
    Complete,
    Waiting(EvaluationWaiting<R>),
}

#[derive(Debug)]
enum EvaluationWaiting<R: Reader> {
    Memory,
    Register { offset: i64 },
    FrameBase { offset: i64 },
    Tls,
    Cfa,
    AtLocation,
    EntryValue,
    ParameterRef,
    RelocatedAddress,
    IndexedAddress,
    TypedLiteral { value: R },
    Convert,
    Reinterpret,
}

/// The state of an `Evaluation` after evaluating a DWARF expression.
/// The evaluation is either `Complete`, or it requires more data
/// to continue, as described by the variant.
#[derive(Debug, PartialEq)]
pub enum EvaluationResult<R: Reader> {
    /// The `Evaluation` is complete, and `Evaluation::result()` can be called.
    Complete,
    /// The `Evaluation` needs a value from memory to proceed further.  Once the
    /// caller determines what value to provide it should resume the `Evaluation`
    /// by calling `Evaluation::resume_with_memory`.
    RequiresMemory {
        /// The address of the value required.
        address: u64,
        /// The size of the value required. This is guaranteed to be at most the
        /// word size of the target architecture.
        size: u8,
        /// If not `None`, a target-specific address space value.
        space: Option<u64>,
        /// The DIE of the base type or 0 to indicate the generic type
        base_type: UnitOffset<R::Offset>,
    },
    /// The `Evaluation` needs a value from a register to proceed further.  Once
    /// the caller determines what value to provide it should resume the
    /// `Evaluation` by calling `Evaluation::resume_with_register`.
    RequiresRegister {
        /// The register number.
        register: Register,
        /// The DIE of the base type or 0 to indicate the generic type
        base_type: UnitOffset<R::Offset>,
    },
    /// The `Evaluation` needs the frame base address to proceed further.  Once
    /// the caller determines what value to provide it should resume the
    /// `Evaluation` by calling `Evaluation::resume_with_frame_base`.  The frame
    /// base address is the address produced by the location description in the
    /// `DW_AT_frame_base` attribute of the current function.
    RequiresFrameBase,
    /// The `Evaluation` needs a value from TLS to proceed further.  Once the
    /// caller determines what value to provide it should resume the
    /// `Evaluation` by calling `Evaluation::resume_with_tls`.
    RequiresTls(u64),
    /// The `Evaluation` needs the CFA to proceed further.  Once the caller
    /// determines what value to provide it should resume the `Evaluation` by
    /// calling `Evaluation::resume_with_call_frame_cfa`.
    RequiresCallFrameCfa,
    /// The `Evaluation` needs the DWARF expression at the given location to
    /// proceed further.  Once the caller determines what value to provide it
    /// should resume the `Evaluation` by calling
    /// `Evaluation::resume_with_at_location`.
    RequiresAtLocation(DieReference<R::Offset>),
    /// The `Evaluation` needs the value produced by evaluating a DWARF
    /// expression at the entry point of the current subprogram.  Once the
    /// caller determines what value to provide it should resume the
    /// `Evaluation` by calling `Evaluation::resume_with_entry_value`.
    RequiresEntryValue(Expression<R>),
    /// The `Evaluation` needs the value of the parameter at the given location
    /// in the current function's caller.  Once the caller determines what value
    /// to provide it should resume the `Evaluation` by calling
    /// `Evaluation::resume_with_parameter_ref`.
    RequiresParameterRef(UnitOffset<R::Offset>),
    /// The `Evaluation` needs an address to be relocated to proceed further.
    /// Once the caller determines what value to provide it should resume the
    /// `Evaluation` by calling `Evaluation::resume_with_relocated_address`.
    RequiresRelocatedAddress(u64),
    /// The `Evaluation` needs an address from the `.debug_addr` section.
    /// This address may also need to be relocated.
    /// Once the caller determines what value to provide it should resume the
    /// `Evaluation` by calling `Evaluation::resume_with_indexed_address`.
    RequiresIndexedAddress {
        /// The index of the address in the `.debug_addr` section,
        /// relative to the `DW_AT_addr_base` of the compilation unit.
        index: DebugAddrIndex<R::Offset>,
        /// Whether the address also needs to be relocated.
        relocate: bool,
    },
    /// The `Evaluation` needs the `ValueType` for the base type DIE at
    /// the give unit offset.  Once the caller determines what value to provide it
    /// should resume the `Evaluation` by calling
    /// `Evaluation::resume_with_base_type`.
    RequiresBaseType(UnitOffset<R::Offset>),
}

/// The bytecode for a DWARF expression or location description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Expression<R: Reader>(pub R);

impl<R: Reader> Expression<R> {
    /// Create an evaluation for this expression.
    ///
    /// The `encoding` is determined by the
    /// [`CompilationUnitHeader`](struct.CompilationUnitHeader.html) or
    /// [`TypeUnitHeader`](struct.TypeUnitHeader.html) that this expression
    /// relates to.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use gimli::Expression;
    /// # let endian = gimli::LittleEndian;
    /// # let debug_info = gimli::DebugInfo::from(gimli::EndianSlice::new(&[], endian));
    /// # let unit = debug_info.units().next().unwrap().unwrap();
    /// # let bytecode = gimli::EndianSlice::new(&[], endian);
    /// let expression = gimli::Expression(bytecode);
    /// let mut eval = expression.evaluation(unit.encoding());
    /// let mut result = eval.evaluate().unwrap();
    /// ```
    #[cfg(feature = "read")]
    #[inline]
    pub fn evaluation(self, encoding: Encoding) -> Evaluation<R> {
        Evaluation::new(self.0, encoding)
    }

    /// Return an iterator for the operations in the expression.
    pub fn operations(self, encoding: Encoding) -> OperationIter<R> {
        OperationIter {
            input: self.0,
            encoding,
        }
    }
}

/// An iterator for the operations in an expression.
#[derive(Debug, Clone, Copy)]
pub struct OperationIter<R: Reader> {
    input: R,
    encoding: Encoding,
}

impl<R: Reader> OperationIter<R> {
    /// Read the next operation in an expression.
    pub fn next(&mut self) -> Result<Option<Operation<R>>> {
        if self.input.is_empty() {
            return Ok(None);
        }
        match Operation::parse(&mut self.input, self.encoding) {
            Ok(op) => Ok(Some(op)),
            Err(e) => {
                self.input.empty();
                Err(e)
            }
        }
    }

    /// Return the current byte offset of the iterator.
    pub fn offset_from(&self, expression: &Expression<R>) -> R::Offset {
        self.input.offset_from(&expression.0)
    }
}

#[cfg(feature = "fallible-iterator")]
impl<R: Reader> fallible_iterator::FallibleIterator for OperationIter<R> {
    type Item = Operation<R>;
    type Error = Error;

    fn next(&mut self) -> ::core::result::Result<Option<Self::Item>, Self::Error> {
        OperationIter::next(self)
    }
}

/// Specification of what storage should be used for [`Evaluation`].
///
#[cfg_attr(
    feature = "read",
    doc = "
Normally you would only need to use [`StoreOnHeap`], which places the stacks and the results
on the heap using [`Vec`]. This is the default storage type parameter for [`Evaluation`].
"
)]
///
/// If you need to avoid [`Evaluation`] from allocating memory, e.g. for signal safety,
/// you can provide you own storage specification:
/// ```rust,no_run
/// # use gimli::*;
/// # let bytecode = EndianSlice::new(&[], LittleEndian);
/// # let encoding = unimplemented!();
/// # let get_register_value = |_, _| Value::Generic(42);
/// # let get_frame_base = || 0xdeadbeef;
/// #
/// struct StoreOnStack;
///
/// impl<R: Reader> EvaluationStorage<R> for StoreOnStack {
///     type Stack = [Value; 64];
///     type ExpressionStack = [(R, R); 4];
///     type Result = [Piece<R>; 1];
/// }
///
/// let mut eval = Evaluation::<_, StoreOnStack>::new_in(bytecode, encoding);
/// let mut result = eval.evaluate().unwrap();
/// while result != EvaluationResult::Complete {
///   match result {
///     EvaluationResult::RequiresRegister { register, base_type } => {
///       let value = get_register_value(register, base_type);
///       result = eval.resume_with_register(value).unwrap();
///     },
///     EvaluationResult::RequiresFrameBase => {
///       let frame_base = get_frame_base();
///       result = eval.resume_with_frame_base(frame_base).unwrap();
///     },
///     _ => unimplemented!(),
///   };
/// }
///
/// let result = eval.as_result();
/// println!("{:?}", result);
/// ```
pub trait EvaluationStorage<R: Reader> {
    /// The storage used for the evaluation stack.
    type Stack: ArrayLike<Item = Value>;
    /// The storage used for the expression stack.
    type ExpressionStack: ArrayLike<Item = (R, R)>;
    /// The storage used for the results.
    type Result: ArrayLike<Item = Piece<R>>;
}

#[cfg(feature = "read")]
impl<R: Reader> EvaluationStorage<R> for StoreOnHeap {
    type Stack = Vec<Value>;
    type ExpressionStack = Vec<(R, R)>;
    type Result = Vec<Piece<R>>;
}

/// A DWARF expression evaluator.
///
/// # Usage
/// A DWARF expression may require additional data to produce a final result,
/// such as the value of a register or a memory location.  Once initial setup
/// is complete (i.e. `set_initial_value()`, `set_object_address()`) the
/// consumer calls the `evaluate()` method.  That returns an `EvaluationResult`,
/// which is either `EvaluationResult::Complete` or a value indicating what
/// data is needed to resume the `Evaluation`.  The consumer is responsible for
/// producing that data and resuming the computation with the correct method,
/// as documented for `EvaluationResult`.  Only once an `EvaluationResult::Complete`
/// is returned can the consumer call `result()`.
///
/// This design allows the consumer of `Evaluation` to decide how and when to
/// produce the required data and resume the computation.  The `Evaluation` can
/// be driven synchronously (as shown below) or by some asynchronous mechanism
/// such as futures.
///
/// # Examples
/// ```rust,no_run
/// use gimli::{Evaluation, EvaluationResult, Expression};
/// # let bytecode = gimli::EndianSlice::new(&[], gimli::LittleEndian);
/// # let encoding = unimplemented!();
/// # let get_register_value = |_, _| gimli::Value::Generic(42);
/// # let get_frame_base = || 0xdeadbeef;
///
/// let mut eval = Evaluation::new(bytecode, encoding);
/// let mut result = eval.evaluate().unwrap();
/// while result != EvaluationResult::Complete {
///   match result {
///     EvaluationResult::RequiresRegister { register, base_type } => {
///       let value = get_register_value(register, base_type);
///       result = eval.resume_with_register(value).unwrap();
///     },
///     EvaluationResult::RequiresFrameBase => {
///       let frame_base = get_frame_base();
///       result = eval.resume_with_frame_base(frame_base).unwrap();
///     },
///     _ => unimplemented!(),
///   };
/// }
///
/// let result = eval.result();
/// println!("{:?}", result);
/// ```
#[derive(Debug)]
pub struct Evaluation<R: Reader, S: EvaluationStorage<R> = StoreOnHeap> {
    bytecode: R,
    encoding: Encoding,
    object_address: Option<u64>,
    max_iterations: Option<u32>,
    iteration: u32,
    state: EvaluationState<R>,

    // Stack operations are done on word-sized values.  We do all
    // operations on 64-bit values, and then mask the results
    // appropriately when popping.
    addr_mask: u64,

    // The stack.
    stack: ArrayVec<S::Stack>,

    // The next operation to decode and evaluate.
    pc: R,

    // If we see a DW_OP_call* operation, the previous PC and bytecode
    // is stored here while evaluating the subroutine.
    expression_stack: ArrayVec<S::ExpressionStack>,

    value_result: Option<Value>,
    result: ArrayVec<S::Result>,
}

#[cfg(feature = "read")]
impl<R: Reader> Evaluation<R> {
    /// Create a new DWARF expression evaluator.
    ///
    /// The new evaluator is created without an initial value, without
    /// an object address, and without a maximum number of iterations.
    pub fn new(bytecode: R, encoding: Encoding) -> Self {
        Self::new_in(bytecode, encoding)
    }

    /// Get the result of this `Evaluation`.
    ///
    /// # Panics
    /// Panics if this `Evaluation` has not been driven to completion.
    pub fn result(self) -> Vec<Piece<R>> {
        match self.state {
            EvaluationState::Complete => self.result.into_vec(),
            _ => {
                panic!("Called `Evaluation::result` on an `Evaluation` that has not been completed")
            }
        }
    }
}

impl<R: Reader, S: EvaluationStorage<R>> Evaluation<R, S> {
    /// Create a new DWARF expression evaluator.
    ///
    /// The new evaluator is created without an initial value, without
    /// an object address, and without a maximum number of iterations.
    pub fn new_in(bytecode: R, encoding: Encoding) -> Self {
        let pc = bytecode.clone();
        Evaluation {
            bytecode,
            encoding,
            object_address: None,
            max_iterations: None,
            iteration: 0,
            state: EvaluationState::Start(None),
            addr_mask: if encoding.address_size == 8 {
                !0u64
            } else {
                (1 << (8 * u64::from(encoding.address_size))) - 1
            },
            stack: Default::default(),
            expression_stack: Default::default(),
            pc,
            value_result: None,
            result: Default::default(),
        }
    }

    /// Set an initial value to be pushed on the DWARF expression
    /// evaluator's stack.  This can be used in cases like
    /// `DW_AT_vtable_elem_location`, which require a value on the
    /// stack before evaluation commences.  If no initial value is
    /// set, and the expression uses an opcode requiring the initial
    /// value, then evaluation will fail with an error.
    ///
    /// # Panics
    /// Panics if `set_initial_value()` has already been called, or if
    /// `evaluate()` has already been called.
    pub fn set_initial_value(&mut self, value: u64) {
        match self.state {
            EvaluationState::Start(None) => {
                self.state = EvaluationState::Start(Some(value));
            }
            _ => panic!(
                "`Evaluation::set_initial_value` was called twice, or after evaluation began."
            ),
        };
    }

    /// Set the enclosing object's address, as used by
    /// `DW_OP_push_object_address`.  If no object address is set, and
    /// the expression uses an opcode requiring the object address,
    /// then evaluation will fail with an error.
    pub fn set_object_address(&mut self, value: u64) {
        self.object_address = Some(value);
    }

    /// Set the maximum number of iterations to be allowed by the
    /// expression evaluator.
    ///
    /// An iteration corresponds approximately to the evaluation of a
    /// single operation in an expression ("approximately" because the
    /// implementation may allow two such operations in some cases).
    /// The default is not to have a maximum; once set, it's not
    /// possible to go back to this default state.  This value can be
    /// set to avoid denial of service attacks by bad DWARF bytecode.
    pub fn set_max_iterations(&mut self, value: u32) {
        self.max_iterations = Some(value);
    }

    fn pop(&mut self) -> Result<Value> {
        match self.stack.pop() {
            Some(value) => Ok(value),
            None => Err(Error::NotEnoughStackItems),
        }
    }

    fn push(&mut self, value: Value) -> Result<()> {
        self.stack.try_push(value).map_err(|_| Error::StackFull)
    }

    fn evaluate_one_operation(&mut self) -> Result<OperationEvaluationResult<R>> {
        let operation = Operation::parse(&mut self.pc, self.encoding)?;

        match operation {
            Operation::Deref {
                base_type,
                size,
                space,
            } => {
                if size > self.encoding.address_size {
                    return Err(Error::InvalidDerefSize(size));
                }
                let entry = self.pop()?;
                let addr = entry.to_u64(self.addr_mask)?;
                let addr_space = if space {
                    let entry = self.pop()?;
                    let value = entry.to_u64(self.addr_mask)?;
                    Some(value)
                } else {
                    None
                };
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::Memory,
                    EvaluationResult::RequiresMemory {
                        address: addr,
                        size,
                        space: addr_space,
                        base_type,
                    },
                ));
            }

            Operation::Drop => {
                self.pop()?;
            }
            Operation::Pick { index } => {
                let len = self.stack.len();
                let index = index as usize;
                if index >= len {
                    return Err(Error::NotEnoughStackItems);
                }
                let value = self.stack[len - index - 1];
                self.push(value)?;
            }
            Operation::Swap => {
                let top = self.pop()?;
                let next = self.pop()?;
                self.push(top)?;
                self.push(next)?;
            }
            Operation::Rot => {
                let one = self.pop()?;
                let two = self.pop()?;
                let three = self.pop()?;
                self.push(one)?;
                self.push(three)?;
                self.push(two)?;
            }

            Operation::Abs => {
                let value = self.pop()?;
                let result = value.abs(self.addr_mask)?;
                self.push(result)?;
            }
            Operation::And => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.and(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Div => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.div(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Minus => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.sub(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Mod => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.rem(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Mul => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.mul(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Neg => {
                let v = self.pop()?;
                let result = v.neg(self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Not => {
                let value = self.pop()?;
                let result = value.not(self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Or => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.or(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Plus => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.add(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::PlusConstant { value } => {
                let lhs = self.pop()?;
                let rhs = Value::from_u64(lhs.value_type(), value)?;
                let result = lhs.add(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Shl => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.shl(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Shr => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.shr(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Shra => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.shra(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Xor => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.xor(rhs, self.addr_mask)?;
                self.push(result)?;
            }

            Operation::Bra { target } => {
                let entry = self.pop()?;
                let v = entry.to_u64(self.addr_mask)?;
                if v != 0 {
                    self.pc = compute_pc(&self.pc, &self.bytecode, target)?;
                }
            }

            Operation::Eq => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.eq(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Ge => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.ge(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Gt => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.gt(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Le => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.le(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Lt => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.lt(rhs, self.addr_mask)?;
                self.push(result)?;
            }
            Operation::Ne => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                let result = lhs.ne(rhs, self.addr_mask)?;
                self.push(result)?;
            }

            Operation::Skip { target } => {
                self.pc = compute_pc(&self.pc, &self.bytecode, target)?;
            }

            Operation::UnsignedConstant { value } => {
                self.push(Value::Generic(value))?;
            }

            Operation::SignedConstant { value } => {
                self.push(Value::Generic(value as u64))?;
            }

            Operation::RegisterOffset {
                register,
                offset,
                base_type,
            } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::Register { offset },
                    EvaluationResult::RequiresRegister {
                        register,
                        base_type,
                    },
                ));
            }

            Operation::FrameOffset { offset } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::FrameBase { offset },
                    EvaluationResult::RequiresFrameBase,
                ));
            }

            Operation::Nop => {}

            Operation::PushObjectAddress => {
                if let Some(value) = self.object_address {
                    self.push(Value::Generic(value))?;
                } else {
                    return Err(Error::InvalidPushObjectAddress);
                }
            }

            Operation::Call { offset } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::AtLocation,
                    EvaluationResult::RequiresAtLocation(offset),
                ));
            }

            Operation::TLS => {
                let entry = self.pop()?;
                let index = entry.to_u64(self.addr_mask)?;
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::Tls,
                    EvaluationResult::RequiresTls(index),
                ));
            }

            Operation::CallFrameCFA => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::Cfa,
                    EvaluationResult::RequiresCallFrameCfa,
                ));
            }

            Operation::Register { register } => {
                let location = Location::Register { register };
                return Ok(OperationEvaluationResult::Complete { location });
            }

            Operation::ImplicitValue { ref data } => {
                let location = Location::Bytes {
                    value: data.clone(),
                };
                return Ok(OperationEvaluationResult::Complete { location });
            }

            Operation::StackValue => {
                let value = self.pop()?;
                let location = Location::Value { value };
                return Ok(OperationEvaluationResult::Complete { location });
            }

            Operation::ImplicitPointer { value, byte_offset } => {
                let location = Location::ImplicitPointer { value, byte_offset };
                return Ok(OperationEvaluationResult::Complete { location });
            }

            Operation::EntryValue { ref expression } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::EntryValue,
                    EvaluationResult::RequiresEntryValue(Expression(expression.clone())),
                ));
            }

            Operation::ParameterRef { offset } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::ParameterRef,
                    EvaluationResult::RequiresParameterRef(offset),
                ));
            }

            Operation::Address { address } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::RelocatedAddress,
                    EvaluationResult::RequiresRelocatedAddress(address),
                ));
            }

            Operation::AddressIndex { index } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::IndexedAddress,
                    EvaluationResult::RequiresIndexedAddress {
                        index,
                        relocate: true,
                    },
                ));
            }

            Operation::ConstantIndex { index } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::IndexedAddress,
                    EvaluationResult::RequiresIndexedAddress {
                        index,
                        relocate: false,
                    },
                ));
            }

            Operation::Piece {
                size_in_bits,
                bit_offset,
            } => {
                let location = if self.stack.is_empty() {
                    Location::Empty
                } else {
                    let entry = self.pop()?;
                    let address = entry.to_u64(self.addr_mask)?;
                    Location::Address { address }
                };
                self.result
                    .try_push(Piece {
                        size_in_bits: Some(size_in_bits),
                        bit_offset,
                        location,
                    })
                    .map_err(|_| Error::StackFull)?;
                return Ok(OperationEvaluationResult::Piece);
            }

            Operation::TypedLiteral { base_type, value } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::TypedLiteral { value },
                    EvaluationResult::RequiresBaseType(base_type),
                ));
            }
            Operation::Convert { base_type } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::Convert,
                    EvaluationResult::RequiresBaseType(base_type),
                ));
            }
            Operation::Reinterpret { base_type } => {
                return Ok(OperationEvaluationResult::Waiting(
                    EvaluationWaiting::Reinterpret,
                    EvaluationResult::RequiresBaseType(base_type),
                ));
            }
            Operation::WasmLocal { .. }
            | Operation::WasmGlobal { .. }
            | Operation::WasmStack { .. } => {
                return Err(Error::UnsupportedEvaluation);
            }
        }

        Ok(OperationEvaluationResult::Incomplete)
    }

    /// Get the result if this is an evaluation for a value.
    ///
    /// Returns `None` if the evaluation contained operations that are only
    /// valid for location descriptions.
    ///
    /// # Panics
    /// Panics if this `Evaluation` has not been driven to completion.
    pub fn value_result(&self) -> Option<Value> {
        match self.state {
            EvaluationState::Complete => self.value_result,
            _ => {
                panic!("Called `Evaluation::value_result` on an `Evaluation` that has not been completed")
            }
        }
    }

    /// Get the result of this `Evaluation`.
    ///
    /// # Panics
    /// Panics if this `Evaluation` has not been driven to completion.
    pub fn as_result(&self) -> &[Piece<R>] {
        match self.state {
            EvaluationState::Complete => &self.result,
            _ => {
                panic!(
                    "Called `Evaluation::as_result` on an `Evaluation` that has not been completed"
                )
            }
        }
    }

    /// Evaluate a DWARF expression.  This method should only ever be called
    /// once.  If the returned `EvaluationResult` is not
    /// `EvaluationResult::Complete`, the caller should provide the required
    /// value and resume the evaluation by calling the appropriate resume_with
    /// method on `Evaluation`.
    pub fn evaluate(&mut self) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Start(initial_value) => {
                if let Some(value) = initial_value {
                    self.push(Value::Generic(value))?;
                }
                self.state = EvaluationState::Ready;
            }
            EvaluationState::Ready => {}
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Complete => return Ok(EvaluationResult::Complete),
            EvaluationState::Waiting(_) => panic!(),
        };

        match self.evaluate_internal() {
            Ok(r) => Ok(r),
            Err(e) => {
                self.state = EvaluationState::Error(e);
                Err(e)
            }
        }
    }

    /// Resume the `Evaluation` with the provided memory `value`.  This will apply
    /// the provided memory value to the evaluation and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresMemory`.
    pub fn resume_with_memory(&mut self, value: Value) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::Memory) => {
                self.push(value)?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_memory` without a preceding `EvaluationResult::RequiresMemory`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `register` value.  This will apply
    /// the provided register value to the evaluation and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresRegister`.
    pub fn resume_with_register(&mut self, value: Value) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::Register { offset }) => {
                let offset = Value::from_u64(value.value_type(), offset as u64)?;
                let value = value.add(offset, self.addr_mask)?;
                self.push(value)?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_register` without a preceding `EvaluationResult::RequiresRegister`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `frame_base`.  This will
    /// apply the provided frame base value to the evaluation and continue
    /// evaluating opcodes until the evaluation is completed, reaches an error,
    /// or needs more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresFrameBase`.
    pub fn resume_with_frame_base(&mut self, frame_base: u64) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::FrameBase { offset }) => {
                self.push(Value::Generic(frame_base.wrapping_add(offset as u64)))?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_frame_base` without a preceding `EvaluationResult::RequiresFrameBase`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `value`.  This will apply
    /// the provided TLS value to the evaluation and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresTls`.
    pub fn resume_with_tls(&mut self, value: u64) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::Tls) => {
                self.push(Value::Generic(value))?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_tls` without a preceding `EvaluationResult::RequiresTls`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `cfa`.  This will
    /// apply the provided CFA value to the evaluation and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresCallFrameCfa`.
    pub fn resume_with_call_frame_cfa(&mut self, cfa: u64) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::Cfa) => {
                self.push(Value::Generic(cfa))?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_call_frame_cfa` without a preceding `EvaluationResult::RequiresCallFrameCfa`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `bytes`.  This will
    /// continue processing the evaluation with the new expression provided
    /// until the evaluation is completed, reaches an error, or needs more
    /// information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresAtLocation`.
    pub fn resume_with_at_location(&mut self, mut bytes: R) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::AtLocation) => {
                if !bytes.is_empty() {
                    let mut pc = bytes.clone();
                    mem::swap(&mut pc, &mut self.pc);
                    mem::swap(&mut bytes, &mut self.bytecode);
                    self.expression_stack.try_push((pc, bytes)).map_err(|_| Error::StackFull)?;
                }
            }
            _ => panic!(
                "Called `Evaluation::resume_with_at_location` without a precedeing `EvaluationResult::RequiresAtLocation`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `entry_value`.  This will
    /// apply the provided entry value to the evaluation and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresEntryValue`.
    pub fn resume_with_entry_value(&mut self, entry_value: Value) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::EntryValue) => {
                self.push(entry_value)?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_entry_value` without a preceding `EvaluationResult::RequiresEntryValue`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `parameter_value`.  This will
    /// apply the provided parameter value to the evaluation and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresParameterRef`.
    pub fn resume_with_parameter_ref(
        &mut self,
        parameter_value: u64,
    ) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::ParameterRef) => {
                self.push(Value::Generic(parameter_value))?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_parameter_ref` without a preceding `EvaluationResult::RequiresParameterRef`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided relocated `address`.  This will use the
    /// provided relocated address for the operation that required it, and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with
    /// `EvaluationResult::RequiresRelocatedAddress`.
    pub fn resume_with_relocated_address(&mut self, address: u64) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::RelocatedAddress) => {
                self.push(Value::Generic(address))?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_relocated_address` without a preceding `EvaluationResult::RequiresRelocatedAddress`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided indexed `address`.  This will use the
    /// provided indexed address for the operation that required it, and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with
    /// `EvaluationResult::RequiresIndexedAddress`.
    pub fn resume_with_indexed_address(&mut self, address: u64) -> Result<EvaluationResult<R>> {
        match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::IndexedAddress) => {
                self.push(Value::Generic(address))?;
            }
            _ => panic!(
                "Called `Evaluation::resume_with_indexed_address` without a preceding `EvaluationResult::RequiresIndexedAddress`"
            ),
        };

        self.evaluate_internal()
    }

    /// Resume the `Evaluation` with the provided `base_type`.  This will use the
    /// provided base type for the operation that required it, and continue evaluating
    /// opcodes until the evaluation is completed, reaches an error, or needs
    /// more information again.
    ///
    /// # Panics
    /// Panics if this `Evaluation` did not previously stop with `EvaluationResult::RequiresBaseType`.
    pub fn resume_with_base_type(&mut self, base_type: ValueType) -> Result<EvaluationResult<R>> {
        let value = match self.state {
            EvaluationState::Error(err) => return Err(err),
            EvaluationState::Waiting(EvaluationWaiting::TypedLiteral { ref value }) => {
                Value::parse(base_type, value.clone())?
            }
            EvaluationState::Waiting(EvaluationWaiting::Convert) => {
                let entry = self.pop()?;
                entry.convert(base_type, self.addr_mask)?
            }
            EvaluationState::Waiting(EvaluationWaiting::Reinterpret) => {
                let entry = self.pop()?;
                entry.reinterpret(base_type, self.addr_mask)?
            }
            _ => panic!(
                "Called `Evaluation::resume_with_base_type` without a preceding `EvaluationResult::RequiresBaseType`"
            ),
        };
        self.push(value)?;
        self.evaluate_internal()
    }

    fn end_of_expression(&mut self) -> bool {
        while self.pc.is_empty() {
            match self.expression_stack.pop() {
                Some((newpc, newbytes)) => {
                    self.pc = newpc;
                    self.bytecode = newbytes;
                }
                None => return true,
            }
        }
        false
    }

    fn evaluate_internal(&mut self) -> Result<EvaluationResult<R>> {
        while !self.end_of_expression() {
            self.iteration += 1;
            if let Some(max_iterations) = self.max_iterations {
                if self.iteration > max_iterations {
                    return Err(Error::TooManyIterations);
                }
            }

            let op_result = self.evaluate_one_operation()?;
            match op_result {
                OperationEvaluationResult::Piece => {}
                OperationEvaluationResult::Incomplete => {
                    if self.end_of_expression() && !self.result.is_empty() {
                        // We saw a piece earlier and then some
                        // unterminated piece.  It's not clear this is
                        // well-defined.
                        return Err(Error::InvalidPiece);
                    }
                }
                OperationEvaluationResult::Complete { location } => {
                    if self.end_of_expression() {
                        if !self.result.is_empty() {
                            // We saw a piece earlier and then some
                            // unterminated piece.  It's not clear this is
                            // well-defined.
                            return Err(Error::InvalidPiece);
                        }
                        self.result
                            .try_push(Piece {
                                size_in_bits: None,
                                bit_offset: None,
                                location,
                            })
                            .map_err(|_| Error::StackFull)?;
                    } else {
                        // If there are more operations, then the next operation must
                        // be a Piece.
                        match Operation::parse(&mut self.pc, self.encoding)? {
                            Operation::Piece {
                                size_in_bits,
                                bit_offset,
                            } => {
                                self.result
                                    .try_push(Piece {
                                        size_in_bits: Some(size_in_bits),
                                        bit_offset,
                                        location,
                                    })
                                    .map_err(|_| Error::StackFull)?;
                            }
                            _ => {
                                let value =
                                    self.bytecode.len().into_u64() - self.pc.len().into_u64() - 1;
                                return Err(Error::InvalidExpressionTerminator(value));
                            }
                        }
                    }
                }
                OperationEvaluationResult::Waiting(waiting, result) => {
                    self.state = EvaluationState::Waiting(waiting);
                    return Ok(result);
                }
            }
        }

        // If no pieces have been seen, use the stack top as the
        // result.
        if self.result.is_empty() {
            let entry = self.pop()?;
            self.value_result = Some(entry);
            let addr = entry.to_u64(self.addr_mask)?;
            self.result
                .try_push(Piece {
                    size_in_bits: None,
                    bit_offset: None,
                    location: Location::Address { address: addr },
                })
                .map_err(|_| Error::StackFull)?;
        }

        self.state = EvaluationState::Complete;
        Ok(EvaluationResult::Complete)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/gimli/unit/src/read/op/tests_2017.rs"));
