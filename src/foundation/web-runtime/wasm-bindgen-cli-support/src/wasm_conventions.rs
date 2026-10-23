//! A tiny crate of utilities for working with implicit Wasm codegen conventions
//! (often established by LLVM and lld).
//!
//! Examples conventions include:
//!
//! * The stack pointer
//! * The canonical linear memory that contains the stack

use std::io::Cursor;

use anyhow::{anyhow, bail, Context, Result};
use walrus::{
    ir::Value, ConstExpr, ConstOp, ElementItems, FunctionBuilder, FunctionId, FunctionKind,
    GlobalId, GlobalKind, MemoryId, Module, RawCustomSection, ValType,
};
use wasmparser::BinaryReader;

/// Get a Wasm module's canonical linear memory.
pub fn get_memory(module: &Module) -> Result<MemoryId> {
    let mut memories = module.memories.iter().map(|m| m.id());
    let memory = memories.next();
    if memories.next().is_some() {
        bail!(
            "expected a single memory, found multiple; multiple memories \
             currently not supported"
        );
    }
    memory.ok_or_else(|| {
        anyhow!(
            "module does not have a memory; must have a memory \
             to transform return pointers into Wasm multi-value"
        )
    })
}

/// Get the `__stack_pointer`.
pub fn get_stack_pointer(module: &Module) -> Option<GlobalId> {
    if let Some(g) = module
        .globals
        .iter()
        .find(|g| matches!(g.name.as_deref(), Some("__stack_pointer")))
    {
        return Some(g.id());
    }

    // PIC/dynamic-linked modules (e.g. emscripten) import the stack pointer
    // from `env.__stack_pointer` instead of defining it locally. Imported
    // globals don't carry their name in the `name` custom section, so look it
    // up via the import section.
    for import in module.imports.iter() {
        if import.module == "env" && import.name == "__stack_pointer" {
            if let walrus::ImportKind::Global(id) = import.kind {
                return Some(id);
            }
        }
    }

    let candidates = module
        .globals
        .iter()
        .filter(|g| g.ty == ValType::I32 || g.ty == ValType::I64)
        .filter(|g| g.mutable)
        // The stack pointer is guaranteed to not be initialized to 0, and it's
        // guaranteed to have an i32/i64 initializer, so find globals which are
        // locally defined and have a nonzero initializer
        .filter(|g| match g.kind {
            GlobalKind::Local(ConstExpr::Value(Value::I32(n))) => n != 0,
            GlobalKind::Local(ConstExpr::Value(Value::I64(n))) => n != 0,
            _ => false,
        })
        .collect::<Vec<_>>();

    match candidates.len() {
        0 => None,
        1 => Some(candidates[0].id()),
        2 => {
            log::warn!("Unable to accurately determine the location of `__stack_pointer`");
            Some(candidates[0].id())
        }
        _ => None,
    }
}

/// Get the `__tls_base`.
pub fn get_tls_base(module: &Module) -> Option<GlobalId> {
    let candidates = module
        .exports
        .iter()
        .filter(|ex| ex.name == "__tls_base")
        .filter_map(|ex| match ex.item {
            walrus::ExportItem::Global(id) => Some(id),
            _ => None,
        })
        .filter(|id| {
            let global = module.globals.get(*id);

            global.ty == ValType::I32 || global.ty == ValType::I64
        })
        .collect::<Vec<_>>();

    match candidates.len() {
        1 => Some(candidates[0]),
        _ => None,
    }
}

/// Attempts to statically evaluate a `ConstExpr` to a scalar `Value`.
///
/// Handles `Value` (immediate), `Global` (resolved via the module's globals),
/// and `Extended` (a small stack-machine subset: numeric constants, GlobalGet,
/// and integer add/sub/mul). Returns `None` for anything that cannot be fully
/// reduced at compile time (imported globals, unsupported opcodes, etc.).
///
/// This is needed because rustc 1.94+ / lld emits element segment offsets as
/// `global.get $__table_base` (`ConstExpr::Global`) or even
/// `global.get $__table_base; i32.const K; i32.add` (`ConstExpr::Extended`)
/// for large WASM modules, rather than a plain `i32.const N`.
fn evaluate_const_expr(expr: &ConstExpr, module: &Module) -> Option<Value> {
    match expr {
        ConstExpr::Value(v) => Some(*v),
        ConstExpr::Global(g) => {
            match &module.globals.get(*g).kind {
                GlobalKind::Local(inner) => evaluate_const_expr(inner, module),
                // Imported globals have no known static value.
                _ => None,
            }
        }
        ConstExpr::Extended(ops) => {
            let mut stack: Vec<Value> = Vec::new();
            for op in ops {
                match op {
                    ConstOp::I32Const(n) => stack.push(Value::I32(*n)),
                    ConstOp::I64Const(n) => stack.push(Value::I64(*n)),
                    ConstOp::F32Const(n) => stack.push(Value::F32(*n)),
                    ConstOp::F64Const(n) => stack.push(Value::F64(*n)),
                    ConstOp::GlobalGet(g) => {
                        let v = match &module.globals.get(*g).kind {
                            GlobalKind::Local(inner) => evaluate_const_expr(inner, module)?,
                            _ => return None,
                        };
                        stack.push(v);
                    }
                    ConstOp::I32Add => {
                        let (Value::I32(b), Value::I32(a)) = (stack.pop()?, stack.pop()?) else {
                            return None;
                        };
                        stack.push(Value::I32(a.wrapping_add(b)));
                    }
                    ConstOp::I32Sub => {
                        let (Value::I32(b), Value::I32(a)) = (stack.pop()?, stack.pop()?) else {
                            return None;
                        };
                        stack.push(Value::I32(a.wrapping_sub(b)));
                    }
                    ConstOp::I32Mul => {
                        let (Value::I32(b), Value::I32(a)) = (stack.pop()?, stack.pop()?) else {
                            return None;
                        };
                        stack.push(Value::I32(a.wrapping_mul(b)));
                    }
                    ConstOp::I64Add => {
                        let (Value::I64(b), Value::I64(a)) = (stack.pop()?, stack.pop()?) else {
                            return None;
                        };
                        stack.push(Value::I64(a.wrapping_add(b)));
                    }
                    ConstOp::I64Sub => {
                        let (Value::I64(b), Value::I64(a)) = (stack.pop()?, stack.pop()?) else {
                            return None;
                        };
                        stack.push(Value::I64(a.wrapping_sub(b)));
                    }
                    ConstOp::I64Mul => {
                        let (Value::I64(b), Value::I64(a)) = (stack.pop()?, stack.pop()?) else {
                            return None;
                        };
                        stack.push(Value::I64(a.wrapping_mul(b)));
                    }
                    _ => return None,
                }
            }
            if stack.len() == 1 {
                stack.pop()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Looks up a function table entry by index in the main function table.
pub fn get_function_table_entry(module: &Module, idx: u32) -> Result<FunctionId> {
    let table = module
        .tables
        .main_function_table()?
        .ok_or_else(|| anyhow!("no function table found in module"))?;
    let table = module.tables.get(table);
    for &segment in table.elem_segments.iter() {
        let segment = module.elements.get(segment);
        let offset = match &segment.kind {
            walrus::ElementKind::Active { offset, .. } => match evaluate_const_expr(offset, module)
            {
                Some(Value::I32(n)) => n as u32,
                Some(Value::I64(n)) => match u32::try_from(n) {
                    Ok(n) => n,
                    // Cannot represent this segment's offset as a table index; skip it.
                    Err(_) => continue,
                },
                // For PIC/dynamic-linked modules (e.g. emscripten output),
                // the segment offset is `global.get __table_base`, an imported
                // global that resolves at instantiation. The Rust-side
                // descriptor encodes the absolute table index assuming a base
                // of 0, so treat an imported base offset as 0 here.
                None if matches!(
                    offset,
                    ConstExpr::Global(g) if matches!(
                        module.globals.get(*g).kind,
                        GlobalKind::Import(_)
                    )
                ) =>
                {
                    0
                }
                // Cannot statically evaluate this segment's offset; skip it.
                _ => continue,
            },
            _ => continue,
        };

        // Guard: if idx < offset this segment does not contain idx.
        let local_idx = match idx.checked_sub(offset) {
            Some(i) => i as usize,
            None => continue,
        };

        let slot = match &segment.items {
            ElementItems::Functions(items) => items.get(local_idx).map(Some),
            ElementItems::Expressions(_, items) => items.get(local_idx).map(|item| {
                if let ConstExpr::RefFunc(target) = item {
                    Some(target)
                } else {
                    None
                }
            }),
        };

        match slot {
            Some(slot) => {
                return slot.copied().context("function table entry wasn't filled");
            }
            None => continue,
        }
    }
    bail!("failed to find `{idx}` in function table");
}

pub fn get_start(module: &mut Module) -> Result<FunctionId, Option<FunctionId>> {
    match module.start {
        Some(start) => match module.funcs.get_mut(start).kind {
            FunctionKind::Import(_) => Err(Some(start)),
            FunctionKind::Local(_) => Ok(start),
            FunctionKind::Uninitialized(_) => unimplemented!(),
        },
        None => Err(None),
    }
}

pub fn get_or_insert_start_builder(module: &mut Module) -> &mut FunctionBuilder {
    let prev_start = get_start(module);

    let id = match prev_start {
        Ok(id) => id,
        Err(prev_start) => {
            let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);

            if let Some(prev_start) = prev_start {
                builder.func_body().call(prev_start);
            }

            let id = builder.finish(Vec::new(), &mut module.funcs);
            module.start = Some(id);
            id
        }
    };

    module
        .funcs
        .get_mut(id)
        .kind
        .unwrap_local_mut()
        .builder_mut()
}

pub fn target_feature(module: &Module, feature: &str) -> Result<bool> {
    // Taken from <https://github.com/bytecodealliance/wasm-tools/blob/f1898f46bb9d96f0f09682415cb6ccfd6a4dca79/crates/wasmparser/src/limits.rs#L27>.
    anyhow::ensure!(feature.len() <= 100_000, "feature name too long");

    // Try to find an existing section.
    let section = module
        .customs
        .iter()
        .find(|(_, custom)| custom.name() == "target_features");

    if let Some((_, section)) = section {
        let section: &RawCustomSection = section
            .as_any()
            .downcast_ref()
            .context("failed to read section")?;
        let mut reader = BinaryReader::new(&section.data, 0);
        // The first integer contains the target feature count.
        let count = reader.read_var_u32()?;

        // Try to find if the target feature is already present.
        for _ in 0..count {
            // First byte is the prefix.
            let prefix = reader.read_u8()?;
            // Read the feature.
            let length = reader.read_var_u32()?;
            let this_feature = reader.read_bytes(length as usize)?;

            // If we found the target feature, we are done here.
            if this_feature == feature.as_bytes() {
                // Make sure we set any existing prefix to "enabled".
                if prefix == b'-' {
                    return Ok(false);
                }

                return Ok(true);
            }
        }

        Ok(false)
    } else {
        Ok(false)
    }
}

pub fn insert_target_feature(module: &mut Module, new_feature: &str) -> Result<()> {
    // Taken from <https://github.com/bytecodealliance/wasm-tools/blob/f1898f46bb9d96f0f09682415cb6ccfd6a4dca79/crates/wasmparser/src/limits.rs#L27>.
    anyhow::ensure!(new_feature.len() <= 100_000, "feature name too long");

    // Try to find an existing section.
    let section = module
        .customs
        .iter_mut()
        .find(|(_, custom)| custom.name() == "target_features");

    // If one exists, check if the target feature is already present.
    let section = if let Some((_, section)) = section {
        let section: &mut RawCustomSection = section
            .as_any_mut()
            .downcast_mut()
            .context("failed to read section")?;
        let mut reader = BinaryReader::new(&section.data, 0);
        // The first integer contains the target feature count.
        let count = reader.read_var_u32()?;

        // Try to find if the target feature is already present.
        for _ in 0..count {
            // First byte is the prefix.
            let prefix_index = reader.current_position();
            let prefix = reader.read_u8()?;
            // Read the feature.
            let length = reader.read_var_u32()?;
            let feature = reader.read_bytes(length as usize)?;

            // If we found the target feature, we are done here.
            if feature == new_feature.as_bytes() {
                // Make sure we set any existing prefix to "enabled".
                if prefix == b'-' {
                    section.data[prefix_index] = b'+';
                }

                return Ok(());
            }
        }

        section
    } else {
        let mut data = Vec::new();
        leb128::write::unsigned(&mut data, 0).unwrap();
        let id = module.customs.add(RawCustomSection {
            name: String::from("target_features"),
            data,
        });
        module.customs.get_mut(id).unwrap()
    };

    // If we couldn't find the target feature, insert it.

    // The first byte contains an integer describing the target feature count, which we increase by one.
    let mut data = Cursor::new(&section.data);
    let count = leb128::read::unsigned(&mut data).unwrap();
    let mut new_count = Vec::new();
    leb128::write::unsigned(&mut new_count, count + 1).unwrap();
    section.data.splice(0..data.position() as usize, new_count);
    // Then we insert the "enabled" prefix at the end.
    section.data.push(b'+');
    // The next byte contains the length of the target feature string.
    leb128::write::unsigned(&mut section.data, new_feature.len() as u64).unwrap();
    // Lastly the target feature string is inserted.
    section.data.extend(new_feature.as_bytes());

    Ok(())
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/wasm-bindgen-cli-support/unit/src/wasm_conventions/tests_408.rs"));
