//! Transformation pass for generating Wasm catch wrappers.
//!
//! This module generates Wasm wrapper functions that catch JavaScript exceptions
//! using the WebAssembly exception handling proposal with `WebAssembly.JSTag`.
//!
//! When a function is marked with `#[wasm_bindgen(catch)]`, instead of wrapping
//! the import call in a JavaScript `handleError` function, we generate a Wasm
//! wrapper that uses `try_table` (modern EH) or `try`/`catch` (legacy EH) to
//! catch JS exceptions directly in Wasm.
//!
//! The wrapper:
//! 1. Calls the original import inside a try block
//! 2. Catches exceptions using the imported JSTag
//! 3. Stores caught exceptions in the externref table
//! 4. Calls `__wbindgen_exn_store` to signal an exception occurred
//! 5. Returns default values (0, null ref, etc.)

use crate::wit::{NonstandardWitSection, WasmBindgenAux};
use anyhow::Error;
use std::collections::HashMap;
use walrus::ir::*;
use walrus::{
    FunctionBuilder, FunctionId, LocalId, MemoryId, Module, RefType, TableId, TagId, ValType,
};

use super::ExceptionHandlingVersion;

/// The kind of wrapper being generated.
#[derive(Clone, Copy)]
enum WrapperKind {
    /// A catch wrapper for `#[wasm_bindgen(catch)]` imports.
    CatchWrapper,
    /// This rethrows recoverable exceptions with a wrapped tag so that our
    /// abort machinery will not treat them as fatal errors.
    NonAbortingWrapper {
        wrapped_js_tag: TagId,
    },
    Aborting {
        rethrow_critical: FunctionId,
    },
}

/// Intrinsics and IDs needed to generate catch wrappers.
#[derive(Clone, Copy)]
struct CatchContext {
    original_func: FunctionId,
    js_tag: TagId,
    wrapper_kind: WrapperKind,
    externref_table: Option<TableId>,
    table_alloc: Option<FunctionId>,
    exn_store: Option<FunctionId>,
    idx_local: LocalId,
    exn_local: LocalId,
    /// Address in linear memory of the `__instance_terminated` flag (u32).
    terminated_addr: i32,
    memory: MemoryId,
}

/// Run the catch handler transformation.
///
/// This finds all imports marked with `catch` and generates Wasm wrapper
/// functions that catch JavaScript exceptions using the appropriate EH mechanism.
///
/// Sets `aux.js_tag` when wrappers are generated.
pub fn run(
    module: &mut Module,
    aux: &mut WasmBindgenAux,
    wit: &NonstandardWitSection,
    eh_version: ExceptionHandlingVersion,
) -> Result<(), Error> {
    // These intrinsics are required when there are catch imports
    let externref_table = aux.externref_table;
    let table_alloc = aux.externref_alloc;
    let exn_store = aux.exn_store;
    // `catch` on a suspending import is handled by the JSPI transform's
    // rejection protocol instead of a catch wrapper, so it doesn't require
    // the exception-store intrinsics.
    let has_catch_wrappers = aux
        .imports_with_catch
        .iter()
        .any(|a| !aux.imports_with_suspending.contains(a));
    if has_catch_wrappers {
        if externref_table.is_none() {
            anyhow::bail!("externref table required for catch wrappers");
        }
        if table_alloc.is_none() {
            anyhow::bail!("externref alloc required for catch wrappers");
        }
        if exn_store.is_none() {
            anyhow::bail!("__wbindgen_exn_store required for catch wrappers");
        }
    }

    let terminated_addr = match get_terminated_addr(module) {
        Ok(addr) => addr,
        // __instance_terminated is only emitted with panic=unwind.
        // If it's missing (e.g. panic=abort with C++ exception handling
        // instructions from embind/libcxx), skip catch wrapper generation.
        Err(_) => return Ok(()),
    };
    let memory = crate::wasm_conventions::get_memory(module)?;

    // Ensure the function table is populated in aux so that the JS codegen
    // can emit the __abort_handler table-index call via expose_invoke_handler.
    if aux.function_table.is_none() {
        aux.function_table = module.tables.main_function_table().ok().flatten();
    }

    aux.legacy_exception_handling = matches!(eh_version, ExceptionHandlingVersion::Legacy);

    // Import the JSTag
    let js_tag = import_js_tag(module);
    let wrapped_js_tag = import_externref_tag(module, "__wbindgen_wrapped_jstag");

    let rethrow_critical = if eh_version == ExceptionHandlingVersion::ModernButWithPanicAbort {
        Some(import_rethrow_critical(module))
    } else {
        None
    };

    let mut wrappers = HashMap::new();

    // Generate wrappers for each import with catch
    for (_import_id, func_id, adapter_id) in wit.implements.iter() {
        let wrapper_kind = if aux.imports_with_catch.contains(adapter_id)
            && !aux.imports_with_suspending.contains(adapter_id)
        {
            WrapperKind::CatchWrapper
        } else if let Some(rethrow_critical) = rethrow_critical {
            WrapperKind::Aborting { rethrow_critical }
        } else {
            WrapperKind::NonAbortingWrapper { wrapped_js_tag }
        };

        let wrapper_id = generate_catch_wrapper(
            module,
            *func_id,
            js_tag,
            wrapper_kind,
            externref_table,
            table_alloc,
            exn_store,
            terminated_addr,
            memory,
            eh_version,
        );

        wrappers.insert(*func_id, wrapper_id);
    }

    // Rewrite all calls to the original imports to use the wrappers instead
    rewrite_calls(module, &wrappers)?;

    log::debug!("Catch handler created {} wrappers", wrappers.len());
    aux.js_tag = Some(js_tag);
    aux.wrapped_js_tag = Some(wrapped_js_tag);
    aux.rethrow_critical = rethrow_critical;

    Ok(())
}

/// Import `__wbindgen_rethrow_critical` host function.
fn import_rethrow_critical(module: &mut Module) -> FunctionId {
    let ty = module.types.add(&[ValType::Ref(RefType::EXTERNREF)], &[]);
    let (func_id, _import_id) =
        module.add_import_func(crate::PLACEHOLDER_MODULE, "__wbindgen_rethrow_critical", ty);
    func_id
}

/// Import the `WebAssembly.JSTag` as a Wasm tag, reusing an existing import
/// (e.g. one created by the JSPI transform) if present.
fn import_externref_tag(module: &mut Module, name: &str) -> TagId {
    if let Some(tag) = module.imports.iter().find_map(|i| match i.kind {
        walrus::ImportKind::Tag(t) if i.module == crate::PLACEHOLDER_MODULE && i.name == name => {
            Some(t)
        }
        _ => None,
    }) {
        return tag;
    }

    // JSTag has a single externref parameter (the caught exception)
    let tag_ty = module.types.add(&[ValType::Ref(RefType::EXTERNREF)], &[]);

    // Use the module's helper to add an imported tag
    let (tag_id, _import_id) = module.add_import_tag(crate::PLACEHOLDER_MODULE, name, tag_ty);

    tag_id
}

fn import_js_tag(module: &mut Module) -> TagId {
    import_externref_tag(module, "__wbindgen_jstag")
}

/// Import (or reuse) the `WebAssembly.JSTag` import. Used by the JSPI
/// transform to catch promise rejections at the suspend point.
pub(crate) fn get_or_import_js_tag(module: &mut Module) -> TagId {
    import_js_tag(module)
}

/// Look up the `__instance_terminated` exported global and return its i32
/// constant value (the address of the termination flag in linear memory).
fn get_terminated_addr(module: &Module) -> Result<i32, Error> {
    let global_id = module
        .exports
        .iter()
        .find(|e| e.name == "__instance_terminated")
        .and_then(|e| match e.item {
            walrus::ExportItem::Global(g) => Some(g),
            _ => None,
        })
        .ok_or_else(|| {
            anyhow::anyhow!("__instance_terminated global required for catch wrappers")
        })?;
    match &module.globals.get(global_id).kind {
        walrus::GlobalKind::Local(walrus::ConstExpr::Value(walrus::ir::Value::I32(v))) => Ok(*v),
        _ => anyhow::bail!("__instance_terminated must be a local i32 constant global"),
    }
}

/// Generate a catch wrapper function for the given import.
fn generate_catch_wrapper(
    module: &mut Module,
    original_func: FunctionId,
    js_tag: TagId,
    wrapper_kind: WrapperKind,
    externref_table: Option<TableId>,
    table_alloc: Option<FunctionId>,
    exn_store: Option<FunctionId>,
    terminated_addr: i32,
    memory: MemoryId,
    eh_version: ExceptionHandlingVersion,
) -> FunctionId {
    // Get the original function's type
    let orig_ty = module.funcs.get(original_func).ty();
    let ty = module.types.get(orig_ty);
    let params = ty.params().to_vec();
    let results = ty.results().to_vec();

    // Create the wrapper function with the same signature
    let mut builder = FunctionBuilder::new(&mut module.types, &params, &results);

    // Create locals for parameters
    let param_locals: Vec<LocalId> = params.iter().map(|ty| module.locals.add(*ty)).collect();

    // Create context with intrinsics and scratch locals
    let ctx = CatchContext {
        original_func,
        js_tag,
        wrapper_kind,
        externref_table,
        table_alloc,
        exn_store,
        idx_local: module.locals.add(ValType::I32),
        exn_local: module.locals.add(ValType::Ref(RefType::EXTERNREF)),
        terminated_addr,
        memory,
    };

    match eh_version {
        ExceptionHandlingVersion::Modern | ExceptionHandlingVersion::ModernButWithPanicAbort => {
            generate_modern_eh_wrapper(&mut builder, module, &param_locals, &results, ctx);
        }
        ExceptionHandlingVersion::Legacy => {
            generate_legacy_eh_wrapper(&mut builder, module, &param_locals, &results, ctx);
        }
        ExceptionHandlingVersion::None => {
            unreachable!("generate_catch_wrapper called with ExceptionHandlingVersion::None");
        }
    }

    let wrapper_id = builder.finish(param_locals, &mut module.funcs);

    // Give the wrapper a descriptive name
    let orig_name = module
        .funcs
        .get(original_func)
        .name
        .as_deref()
        .unwrap_or("<unknown>");
    module.funcs.get_mut(wrapper_id).name = Some(format!("{orig_name} catch wrapper"));

    wrapper_id
}

/// Generate wrapper using modern EH (try_table).
///
/// Structure:
/// ```wat
/// (block $catch_all_block (result exnref)
///   (block $catch (result externref)
///     (try_table (catch $jstag $catch) (catch_all_ref $catch_all_block)
///       local.get <params>...
///       call $original
///       <termination guard>
///       return
///     )
///     unreachable
///   )
///   ;; catch $jstag path: externref on stack
///   <termination guard>
///   <emit_catch_handler>
///   return
/// )
/// ;; catch_all path: exnref on stack
/// local.set $exn_ref_local
/// <termination guard>
/// local.get $exn_ref_local
/// throw_ref
/// ```
fn generate_modern_eh_wrapper(
    builder: &mut FunctionBuilder,
    module: &mut Module,
    param_locals: &[LocalId],
    results: &[ValType],
    ctx: CatchContext,
) {
    let externref_block_ty: InstrSeqType = ValType::Ref(RefType::EXTERNREF).into();
    let exnref_block_ty: InstrSeqType = ValType::Ref(RefType::EXNREF).into();

    let exn_ref_local = module.locals.add(ValType::Ref(RefType::EXNREF));

    // Pre-allocate all block IDs so they can reference each other
    let try_body_id = builder.dangling_instr_seq(None).id();
    let catch_block_id = builder.dangling_instr_seq(externref_block_ty).id();
    let catch_all_block_id = builder.dangling_instr_seq(exnref_block_ty).id();

    // Try body: call original + termination check + return
    {
        let mut try_body = builder.instr_seq(try_body_id);
        for local in param_locals {
            try_body.local_get(*local);
        }
        try_body.call(ctx.original_func);
        emit_termination_guard(&mut try_body, ctx);
        try_body.instr(Return {});
    }

    // Catch $jstag block: contains try_table that catches to both blocks
    {
        let mut catch_block = builder.instr_seq(catch_block_id);
        catch_block.instr(TryTable {
            seq: try_body_id,
            catches: vec![
                TryTableCatch::Catch {
                    tag: ctx.js_tag,
                    label: catch_block_id,
                },
                TryTableCatch::CatchAllRef {
                    label: catch_all_block_id,
                },
            ],
        });
        catch_block.unreachable();
    }

    // Catch_all block: wraps catch block, handles jstag path, falls through for catch_all
    {
        let mut catch_all_block = builder.instr_seq(catch_all_block_id);

        // The catch $jstag block
        catch_all_block.instr(Block {
            seq: catch_block_id,
        });

        // catch $jstag path: externref on stack
        emit_termination_guard(&mut catch_all_block, ctx);
        emit_catch_handler(&mut catch_all_block, ctx, results);
        catch_all_block.instr(Return {});
    }

    // Function body: catch_all block + exnref rethrow path
    {
        let mut body = builder.func_body();

        body.instr(Block {
            seq: catch_all_block_id,
        });

        // catch_all path: exnref on stack
        body.local_set(exn_ref_local);
        emit_termination_guard(&mut body, ctx);
        body.local_get(exn_ref_local);
        body.instr(ThrowRef {});
    }
}

/// Generate wrapper using legacy EH (try/catch/catch_all).
///
/// Structure:
/// ```wat
/// (try (result <results>)
///   local.get <params>...
///   call $original
///   <termination guard>
/// catch $jstag
///   ;; externref is on stack from catch
///   <termination guard>
///   <emit_catch_handler>
/// catch_all
///   <termination guard>
///   rethrow 0
/// end)
/// ```
fn generate_legacy_eh_wrapper(
    builder: &mut FunctionBuilder,
    module: &mut Module,
    param_locals: &[LocalId],
    results: &[ValType],
    ctx: CatchContext,
) {
    // Block type for results
    let result_block_ty: InstrSeqType = match results.len() {
        0 => None.into(),
        1 => results[0].into(),
        _ => {
            let ty_id = module.types.add(&[], results);
            ty_id.into()
        }
    };

    // Catch $jstag handler receives externref and produces results
    let catch_params = vec![ValType::Ref(RefType::EXTERNREF)];
    let catch_block_ty: InstrSeqType = module.types.add(&catch_params, results).into();

    // Try body: call original + termination check
    let try_body_id = builder.dangling_instr_seq(result_block_ty).id();
    {
        let mut try_body = builder.instr_seq(try_body_id);
        for local in param_locals {
            try_body.local_get(*local);
        }
        try_body.call(ctx.original_func);
        emit_termination_guard(&mut try_body, ctx);
    }

    // Catch $jstag handler: check termination then handle exception
    let catch_handler_id = builder.dangling_instr_seq(catch_block_ty).id();
    {
        let mut catch_handler = builder.instr_seq(catch_handler_id);
        emit_termination_guard(&mut catch_handler, ctx);
        emit_catch_handler(&mut catch_handler, ctx, results);
    }

    // Catch_all handler: check termination then rethrow
    let catch_all_handler_id = builder.dangling_instr_seq(None).id();
    {
        let mut catch_all_handler = builder.instr_seq(catch_all_handler_id);
        emit_termination_guard(&mut catch_all_handler, ctx);
        catch_all_handler.instr(Rethrow { relative_depth: 0 });
    }

    // Add try instruction to function body
    {
        let mut body = builder.func_body();
        body.instr(Try {
            seq: try_body_id,
            catches: vec![
                LegacyCatch::Catch {
                    tag: ctx.js_tag,
                    handler: catch_handler_id,
                },
                LegacyCatch::CatchAll {
                    handler: catch_all_handler_id,
                },
            ],
        });
    }
}

/// Emit the exception handling code based on wrapper kind.
///
/// Expects the caught externref to be on the stack.
///
/// For `CatchWrapper`: stores the exception and returns default values.
///
/// For `NonAbortingWrapper`: rethrows the exception with the wrapped tag.
/// Foreign exceptions will unwind the stack and so they are recoverable.
/// However, they cannot be caught with `catch_unwind`. The `throw` and
/// `rethrow` intrinsics also wrap their error in this tag.
fn emit_catch_handler(
    builder: &mut walrus::InstrSeqBuilder,
    ctx: CatchContext,
    results: &[ValType],
) {
    match ctx.wrapper_kind {
        WrapperKind::NonAbortingWrapper { wrapped_js_tag } => {
            // Rethrow the exception with the wrapped tag
            builder.instr(Throw {
                tag: wrapped_js_tag,
            });
        }
        WrapperKind::CatchWrapper => {
            // Store the externref in the externref table and call exn_store
            // We validated at the start of `run` when imports_with_catch is non-empty so just unwrap here
            let table_alloc = ctx.table_alloc.unwrap();
            let externref_table = ctx.externref_table.unwrap();
            let exn_store = ctx.exn_store.unwrap();
            builder.local_set(ctx.exn_local);
            builder.call(table_alloc);
            builder.local_tee(ctx.idx_local);
            builder.local_get(ctx.exn_local);
            builder.table_set(externref_table);
            builder.local_get(ctx.idx_local);
            builder.call(exn_store);
            push_default_values(builder, results);
        }
        WrapperKind::Aborting { rethrow_critical } => {
            // The caught externref (the original error) is on the stack.
            // rethrow it wrapped in
            // `new Error("Critical error", { cause: origError })`.
            builder.call(rethrow_critical);
            // unreachable needed to keep the instruction sequence well-typed.
            builder.unreachable();
        }
    }
}

/// Emit the termination guard pattern:
///
/// ```wat
/// i32.const $terminated_addr
/// i32.load
/// if
///   unreachable
/// end
/// ```
///
/// This checks if `__instance_terminated` is nonzero and traps if so, ensuring
/// that instance termination is never swallowed by an outer catch handler.
fn emit_termination_guard(builder: &mut walrus::InstrSeqBuilder, ctx: CatchContext) {
    let mem_arg: MemArg = MemArg {
        align: 4,
        offset: 0,
    };
    builder
        .i32_const(ctx.terminated_addr)
        .load(ctx.memory, LoadKind::I32 { atomic: false }, mem_arg)
        .if_else(
            None,
            |then| {
                then.unreachable();
            },
            |_else| {},
        );
}

/// Push default values for the given result types onto the stack.
fn push_default_values(builder: &mut walrus::InstrSeqBuilder, results: &[ValType]) {
    for ty in results {
        match ty {
            ValType::I32 => {
                builder.i32_const(0);
            }
            ValType::I64 => {
                builder.i64_const(0);
            }
            ValType::F32 => {
                builder.f32_const(0.0);
            }
            ValType::F64 => {
                builder.f64_const(0.0);
            }
            ValType::V128 => {
                panic!("v128 return type in catch wrapper not implemented");
            }
            ValType::Ref(ref_ty) => {
                builder.ref_null(*ref_ty);
            }
        }
    }
}

/// Rewrite all calls to original import functions to use the wrappers instead.
fn rewrite_calls(
    module: &mut Module,
    wrappers: &HashMap<FunctionId, FunctionId>,
) -> Result<(), Error> {
    if wrappers.is_empty() {
        return Ok(());
    }

    // Get the set of wrapper function IDs to avoid rewriting calls within wrappers
    let wrapper_ids: std::collections::HashSet<_> = wrappers.values().copied().collect();

    // Visit all local functions and rewrite calls
    for (func_id, func) in module.funcs.iter_local_mut() {
        // Don't rewrite calls within the wrappers themselves
        if wrapper_ids.contains(&func_id) {
            continue;
        }

        let entry = func.entry_block();
        dfs_pre_order_mut(&mut CallRewriter { wrappers }, func, entry);
    }

    Ok(())
}

struct CallRewriter<'a> {
    wrappers: &'a HashMap<FunctionId, FunctionId>,
}

impl VisitorMut for CallRewriter<'_> {
    fn start_instr_seq_mut(&mut self, seq: &mut InstrSeq) {
        for (instr, _) in seq.instrs.iter_mut() {
            let func = match instr {
                Instr::Call(Call { func }) => func,
                Instr::ReturnCall(ReturnCall { func }) => func,
                _ => continue,
            };
            if let Some(wrapper) = self.wrappers.get(func) {
                *func = *wrapper;
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/wasm-bindgen-cli-support/unit/src/transforms/catch_handler/tests_619.rs"));
