//! Implementation of the `unevaluatedItems` keyword.
//!
//! This keyword validates array items that were not evaluated by other keywords like
//! `items`, `prefixItems`, `contains`, or nested schemas in combinators (`allOf`, `anyOf`, `oneOf`),
//! conditionals, and references.
//!
//! The implementation eagerly compiles a recursive `ItemsValidators` structure during
//! schema compilation, using `Arc<OnceLock>` for circular reference handling.
use referencing::{Draft, Vocabulary};
use serde_json::{Map, Value};
use std::{
    fmt,
    sync::{Arc, OnceLock},
};

use crate::{
    compiler,
    evaluation::{ChildList, ErrorDescription},
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    validator::{EvaluationResult, Validate, ValidationContext},
    Array, Json, Node, SerdeJson, ValidationError,
};

use super::CompilationResult;

/// Lazy items validators that are compiled on first access.
/// Used for $recursiveRef and circular references to handle cycles during compilation.
pub(crate) type PendingItemsValidators<F = SerdeJson> = Arc<OnceLock<ItemsValidators<F>>>;

/// Holds compiled validators for items evaluation in unevaluatedItems.
/// This structure is built during schema compilation and used during validation.
pub(crate) struct ItemsValidators<F: Json = SerdeJson> {
    /// Validator from "unevaluatedItems" keyword itself
    unevaluated: Option<SchemaNode<F>>,
    /// Validator from "contains" keyword
    contains: Option<SchemaNode<F>>,
    /// Reference validators from "$ref" keyword
    /// Uses pending pattern to handle circular references
    ref_: Option<PendingItemsValidators<F>>,
    /// Reference validators from "$dynamicRef" keyword (Draft 2020-12+)
    /// Uses pending pattern to handle circular references
    dynamic_ref: Option<PendingItemsValidators<F>>,
    /// Validators from "$recursiveRef" keyword (Draft 2019-09 only)
    recursive_ref: Option<PendingItemsValidators<F>>,
    /// Items limit - for Draft 2019-09 "items" keyword behavior
    /// If present, marks first N items as evaluated
    items_limit: Option<usize>,
    /// Items schema present - for Draft 2020-12+ "items" keyword
    /// If true, marks ALL items as evaluated
    items_all: bool,
    /// Prefix items count - from "prefixItems" keyword
    prefix_items: Option<usize>,
    /// Conditional validators from "if/then/else" keywords
    conditional: Option<Box<ConditionalValidators<F>>>,
    /// Validators from "allOf" keyword
    all_of: Option<Vec<(SchemaNode<F>, ItemsValidators<F>)>>,
    /// Validators from "anyOf" keyword
    any_of: Option<Vec<(SchemaNode<F>, ItemsValidators<F>)>>,
    /// Validators from "oneOf" keyword
    one_of: Option<Vec<(SchemaNode<F>, ItemsValidators<F>)>>,
}

// Manual impls: derives would require `F: Clone` / `F: Debug` even though `F` is a marker type.
impl<F: Json> Clone for ItemsValidators<F> {
    fn clone(&self) -> Self {
        ItemsValidators {
            unevaluated: self.unevaluated.clone(),
            contains: self.contains.clone(),
            ref_: self.ref_.clone(),
            dynamic_ref: self.dynamic_ref.clone(),
            recursive_ref: self.recursive_ref.clone(),
            items_limit: self.items_limit,
            items_all: self.items_all,
            prefix_items: self.prefix_items,
            conditional: self.conditional.clone(),
            all_of: self.all_of.clone(),
            any_of: self.any_of.clone(),
            one_of: self.one_of.clone(),
        }
    }
}

impl<F: Json> fmt::Debug for ItemsValidators<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemsValidators").finish_non_exhaustive()
    }
}

/// Conditional validators from "if/then/else" keywords
struct ConditionalValidators<F: Json = SerdeJson> {
    condition: SchemaNode<F>,
    /// `None` for a boolean `if`, which evaluates nothing.
    if_: Option<ItemsValidators<F>>,
    then_: Option<ItemsValidators<F>>,
    else_: Option<ItemsValidators<F>>,
}

impl<F: Json> Clone for ConditionalValidators<F> {
    fn clone(&self) -> Self {
        ConditionalValidators {
            condition: self.condition.clone(),
            if_: self.if_.clone(),
            then_: self.then_.clone(),
            else_: self.else_.clone(),
        }
    }
}

impl<F: Json> fmt::Debug for ConditionalValidators<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConditionalValidators")
            .finish_non_exhaustive()
    }
}

impl<F: Json> ItemsValidators<F> {
    /// Core implementation for marking evaluated indexes.
    ///
    /// When `include_unevaluated` is `true` (used by `is_valid`/`validate`), also marks
    /// items validated by `unevaluatedItems` itself — needed so nested schemas can propagate
    /// evaluations upward. When `false` (used by `evaluate`), those items are left unmarked
    /// so `evaluate_instance()` is called on them to collect annotations.
    fn mark_evaluated_indexes_impl(
        &self,
        instance: &F::Node<'_>,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
        include_unevaluated: bool,
    ) {
        // Break cycles from self-referential `$dynamicRef`/`$recursiveRef` under
        // `unevaluatedItems`.
        let validators_id = std::ptr::from_ref::<ItemsValidators<F>>(self) as usize;
        let identity = instance.identity();
        if ctx.enter_marking(validators_id, identity) {
            return;
        }
        self.mark_evaluated_indexes_inner(instance, indexes, ctx, include_unevaluated);
        if identity.is_some() {
            ctx.exit_marking();
        }
    }

    fn mark_evaluated_indexes_inner(
        &self,
        instance: &F::Node<'_>,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
        include_unevaluated: bool,
    ) {
        // Early return optimization: if items marks ALL items, no need to check anything else
        if self.items_all {
            // Draft 2020-12+: items keyword marks ALL items as evaluated
            for idx in indexes.iter_mut() {
                *idx = true;
            }
            return;
        }

        // Handle $ref first
        if let Some(ref_) = &self.ref_ {
            initialized(ref_).mark_evaluated_indexes(instance, indexes, ctx);
        }

        // Handle $recursiveRef (Draft 2019-09 only)
        if let Some(recursive_ref) = &self.recursive_ref {
            initialized(recursive_ref).mark_evaluated_indexes(instance, indexes, ctx);
        }

        // Handle $dynamicRef (Draft 2020-12+)
        if let Some(dynamic_ref) = &self.dynamic_ref {
            initialized(dynamic_ref).mark_evaluated_indexes(instance, indexes, ctx);
        }

        // Mark items based on items/prefixItems keywords
        if let Some(limit) = self.items_limit {
            // Draft 2019-09: items (as array) marks first N items
            for idx in indexes.iter_mut().take(limit) {
                *idx = true;
            }
        }

        if let Some(limit) = self.prefix_items {
            // prefixItems marks first N items
            for idx in indexes.iter_mut().take(limit) {
                *idx = true;
            }
        }

        // Early exit if all items are already evaluated
        if indexes.iter().all(|&evaluated| evaluated) {
            return;
        }

        // Process contains and (optionally) unevaluatedItems
        if let Some(array) = instance.as_array() {
            for (item, is_evaluated) in array.elements().zip(indexes.iter_mut()) {
                if *is_evaluated {
                    continue;
                }
                // contains marks items that match
                if let Some(validator) = &self.contains {
                    if validator.is_valid(&item, ctx) {
                        *is_evaluated = true;
                        continue;
                    }
                }
                // unevaluatedItems itself can mark items.
                // Skipped when called from evaluate() so evaluate_instance() can collect annotations.
                if include_unevaluated {
                    if let Some(validator) = &self.unevaluated {
                        if validator.is_valid(&item, ctx) {
                            *is_evaluated = true;
                        }
                    }
                }
            }
        }

        // Handle conditional
        if let Some(conditional) = &self.conditional {
            conditional.mark_evaluated_indexes(instance, indexes, ctx);
        }

        // Handle allOf - each schema that validates successfully marks items
        if let Some(all_of) = &self.all_of {
            for (validator, item_validators) in all_of {
                if validator.is_valid(instance, ctx) {
                    item_validators.mark_evaluated_indexes(instance, indexes, ctx);
                }
            }
        }

        // Handle anyOf - each schema that validates successfully marks items
        if let Some(any_of) = &self.any_of {
            for (validator, item_validators) in any_of {
                if validator.is_valid(instance, ctx) {
                    item_validators.mark_evaluated_indexes(instance, indexes, ctx);
                }
            }
        }

        // Handle oneOf - only mark if exactly one schema validates
        // Short-circuit: stop checking after finding 2 matches
        if let Some(one_of) = &self.one_of {
            let mut match_count = 0;
            let mut matched_validators = None;
            for (node, validators) in one_of {
                if node.is_valid(instance, ctx) {
                    match_count += 1;
                    if match_count > 1 {
                        break; // More than one match, don't mark any indexes
                    }
                    matched_validators = Some(validators);
                }
            }
            if match_count == 1 {
                if let Some(validators) = matched_validators {
                    validators.mark_evaluated_indexes(instance, indexes, ctx);
                }
            }
        }
    }

    /// Mark all items evaluated by this schema (including by `unevaluatedItems` itself).
    fn mark_evaluated_indexes(
        &self,
        instance: &F::Node<'_>,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        self.mark_evaluated_indexes_impl(instance, indexes, ctx, true);
    }

    /// Mark items evaluated by all keywords *except* `unevaluatedItems` itself.
    ///
    /// Used in `evaluate()` so that items that would be covered by `unevaluatedItems`
    /// are still visited by `evaluate_instance()`, allowing their annotations to be collected.
    fn mark_evaluated_indexes_by_other_keywords(
        &self,
        instance: &F::Node<'_>,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        self.mark_evaluated_indexes_impl(instance, indexes, ctx, false);
    }
}

impl<F: Json> ConditionalValidators<F> {
    fn mark_evaluated_indexes(
        &self,
        instance: &F::Node<'_>,
        indexes: &mut Vec<bool>,
        ctx: &mut ValidationContext,
    ) {
        if self.condition.is_valid(instance, ctx) {
            if let Some(if_) = &self.if_ {
                if_.mark_evaluated_indexes(instance, indexes, ctx);
            }
            if let Some(then_) = &self.then_ {
                then_.mark_evaluated_indexes(instance, indexes, ctx);
            }
        } else if let Some(else_) = &self.else_ {
            else_.mark_evaluated_indexes(instance, indexes, ctx);
        }
    }
}

/// Compile all items validators for a schema.
///
/// Recursively builds the `ItemsValidators` tree by examining all keywords that
/// can evaluate items. Handles circular references via pending nodes cached
/// by schema pointer.
fn compile_items_validators<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<ItemsValidators<F>, ValidationError<'a>> {
    let pending = compile_pending_items_validators(ctx, parent)?;
    // Only a reference cycle through this node keeps another handle to the cell
    Ok(match Arc::try_unwrap(pending) {
        Ok(cell) => cell
            .into_inner()
            .expect("pending node is initialized before it is returned"),
        Err(shared) => initialized(&shared).clone(),
    })
}

/// The same compilation, handing back the cell cyclic references share.
fn compile_pending_items_validators<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<PendingItemsValidators<F>, ValidationError<'a>> {
    // Create a pending node and cache it before compiling to handle circular refs
    let pending = Arc::new(OnceLock::new());
    ctx.cache_pending_items_validators_for_schema(parent, pending.clone());

    let applicator = ctx.has_vocabulary(&Vocabulary::Applicator);

    let unevaluated = compile_unevaluated(ctx, parent)?;
    let contains = if applicator {
        compile_contains(ctx, parent)?
    } else {
        None
    };
    let ref_ = compile_ref(ctx, parent)?;
    let dynamic_ref = compile_dynamic_ref(ctx, parent)?;
    let recursive_ref = compile_recursive_ref(ctx, parent)?;

    // Determine items behavior based on draft
    let (items_limit, items_all) = if applicator {
        compile_items(ctx, parent)?
    } else {
        (None, false)
    };
    let prefix_items = if applicator {
        compile_prefix_items(ctx, parent)?
    } else {
        None
    };

    let conditional = if applicator {
        compile_conditional(ctx, parent)?
    } else {
        None
    };
    let all_of = if applicator {
        compile_all_of(ctx, parent)?
    } else {
        None
    };
    let any_of = if applicator {
        compile_any_of(ctx, parent)?
    } else {
        None
    };
    let one_of = if applicator {
        compile_one_of(ctx, parent)?
    } else {
        None
    };

    let validators = ItemsValidators {
        unevaluated,
        contains,
        ref_,
        dynamic_ref,
        recursive_ref,
        items_limit,
        items_all,
        prefix_items,
        conditional,
        all_of,
        any_of,
        one_of,
    };

    pending
        .set(validators)
        .expect("pending node should not be initialized yet");
    ctx.remove_pending_items_validators_for_schema(parent);

    Ok(pending)
}

/// Every cell is initialized before compilation returns, so validation never sees an empty one.
fn initialized<F: Json>(pending: &PendingItemsValidators<F>) -> &ItemsValidators<F> {
    pending
        .get()
        .expect("pending node is initialized before validation")
}

fn compile_unevaluated<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<SchemaNode<F>>, ValidationError<'a>> {
    if let Some(subschema) = parent.get("unevaluatedItems") {
        let unevaluated_ctx = ctx.new_at_location("unevaluatedItems");
        Ok(Some(
            compiler::compile(&unevaluated_ctx, unevaluated_ctx.as_resource_ref(subschema))
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_contains<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<SchemaNode<F>>, ValidationError<'a>> {
    if let Some(subschema) = parent.get("contains") {
        let contains_ctx = ctx.new_at_location("contains");
        Ok(Some(
            compiler::compile(&contains_ctx, contains_ctx.as_resource_ref(subschema))
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_ref<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<PendingItemsValidators<F>>, ValidationError<'a>> {
    let Some(Value::String(reference)) = parent.get("$ref") else {
        return Ok(None);
    };

    let resolved = ctx.lookup(reference).map_err(ValidationError::from)?;

    let (contents, resolver, draft) = resolved.into_inner();
    if let Value::Object(subschema) = &contents {
        let vocabularies = resolver.find_vocabularies(draft, contents);
        let ref_ctx =
            ctx.with_resolver_and_draft(resolver, draft, vocabularies, ctx.location().clone())?;

        // Circular reference: the target is already being compiled - return its pending node.
        if let Some(pending) = ref_ctx.get_pending_items_validators_for_schema(subschema) {
            return Ok(Some(pending));
        }

        Ok(Some(
            compile_pending_items_validators(&ref_ctx, subschema)
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_dynamic_ref<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &Map<String, Value>,
) -> Result<Option<PendingItemsValidators<F>>, ValidationError<'a>> {
    let Some(Value::String(reference)) = parent.get("$dynamicRef") else {
        return Ok(None);
    };

    let resolved = ctx.lookup(reference).map_err(ValidationError::from)?;

    let (contents, resolver, draft) = resolved.into_inner();
    if let Value::Object(subschema) = &contents {
        let vocabularies = resolver.find_vocabularies(draft, contents);
        let ref_ctx =
            ctx.with_resolver_and_draft(resolver, draft, vocabularies, ctx.location().clone())?;

        // Circular reference: the target is already being compiled - return its pending node.
        if let Some(pending) = ref_ctx.get_pending_items_validators_for_schema(subschema) {
            return Ok(Some(pending));
        }

        Ok(Some(
            compile_pending_items_validators(&ref_ctx, subschema)
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_recursive_ref<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &Map<String, Value>,
) -> Result<Option<PendingItemsValidators<F>>, ValidationError<'a>> {
    if !parent.contains_key("$recursiveRef") {
        return Ok(None);
    }

    // For $recursiveRef, we need to resolve the reference and check if it's already being compiled
    let resolved = ctx
        .lookup_recursive_reference()
        .map_err(ValidationError::from)?;

    // Create context for the resolved reference and check its cache key
    let (contents, resolver, draft) = resolved.into_inner();
    if let Value::Object(subschema) = &contents {
        let vocabularies = resolver.find_vocabularies(draft, contents);
        let ref_ctx =
            ctx.with_resolver_and_draft(resolver, draft, vocabularies, ctx.location().clone())?;

        // Check if we're already compiling this schema (circular reference)
        if let Some(pending) = ref_ctx.get_pending_items_validators_for_schema(subschema) {
            return Ok(Some(pending));
        }

        // Not circular, compile normally
        Ok(Some(
            compile_pending_items_validators(&ref_ctx, subschema)
                .map_err(ValidationError::to_owned)?,
        ))
    } else {
        Ok(None)
    }
}

fn compile_items<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<(Option<usize>, bool), ValidationError<'a>> {
    if let Some(subschema) = parent.get("items") {
        if ctx.draft() == Draft::Draft201909
            || ctx.draft() == Draft::Draft7
            || ctx.draft() == Draft::Draft6
            || ctx.draft() == Draft::Draft4
        {
            // Older drafts: items can be array or object
            let limit = if parent.contains_key("additionalItems") || subschema.is_object() {
                usize::MAX
            } else {
                subschema.as_array().map_or(usize::MAX, std::vec::Vec::len)
            };
            Ok((Some(limit), false))
        } else {
            // Draft 2020-12+: items is always a schema that applies to all items
            Ok((None, true))
        }
    } else {
        Ok((None, false))
    }
}

fn compile_prefix_items<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<usize>, ValidationError<'a>> {
    // `prefixItems` arrived in 2020-12; an earlier draft reads it as an unknown keyword, and an
    // unknown keyword evaluates nothing.
    if !ctx.draft().is_known_keyword("prefixItems") {
        return Ok(None);
    }
    if let Some(Some(items)) = parent.get("prefixItems").map(Value::as_array) {
        Ok(Some(items.len()))
    } else {
        Ok(None)
    }
}

/// Compile the item validators for a `then`/`else` branch, entering its subresource so a nested
/// `$id` shifts the base URI for anything resolved inside it.
fn compile_branch<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<ItemsValidators<F>>, ValidationError<'a>> {
    let Some(value) = parent.get(keyword) else {
        return Ok(None);
    };
    let Value::Object(schema) = value else {
        return Ok(None);
    };
    let branch_ctx = ctx.new_at_location(keyword);
    let inner_ctx = branch_ctx
        .in_subresource(branch_ctx.as_resource_ref(value))
        .map_err(ValidationError::from)?;
    Ok(Some(
        compile_items_validators(&inner_ctx, schema).map_err(ValidationError::to_owned)?,
    ))
}

fn compile_conditional<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<Box<ConditionalValidators<F>>>, ValidationError<'a>> {
    let Some(subschema @ (Value::Object(_) | Value::Bool(_))) = parent.get("if") else {
        return Ok(None);
    };
    let if_ctx = ctx.new_at_location("if");
    let if_resource = if_ctx.as_resource_ref(subschema);
    let condition = compiler::compile(&if_ctx, if_resource).map_err(ValidationError::to_owned)?;
    let if_ = if let Value::Object(if_parent) = subschema {
        let if_inner_ctx = if_ctx
            .in_subresource(if_resource)
            .map_err(ValidationError::from)?;
        Some(
            compile_items_validators(&if_inner_ctx, if_parent)
                .map_err(ValidationError::to_owned)?,
        )
    } else {
        None
    };

    Ok(Some(Box::new(ConditionalValidators {
        condition,
        if_,
        then_: compile_branch(ctx, parent, "then")?,
        else_: compile_branch(ctx, parent, "else")?,
    })))
}

type CompiledItemsSubschemas<F> = Vec<(SchemaNode<F>, ItemsValidators<F>)>;

fn compile_all_of<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<CompiledItemsSubschemas<F>>, ValidationError<'a>> {
    if let Some(Some(subschemas)) = parent.get("allOf").map(Value::as_array) {
        let all_of_ctx = ctx.new_at_location("allOf");
        let mut result = Vec::with_capacity(subschemas.len());

        for (idx, subschema) in subschemas.iter().enumerate() {
            if let Value::Object(parent) = subschema {
                let subschema_ctx = all_of_ctx.new_at_location(idx);
                let resource = subschema_ctx.as_resource_ref(subschema);
                let node = compiler::compile(&subschema_ctx, resource)
                    .map_err(ValidationError::to_owned)?;
                let inner_ctx = subschema_ctx
                    .in_subresource(resource)
                    .map_err(ValidationError::from)?;
                result.push((
                    node,
                    compile_items_validators(&inner_ctx, parent)
                        .map_err(ValidationError::to_owned)?,
                ));
            }
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn compile_any_of<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<CompiledItemsSubschemas<F>>, ValidationError<'a>> {
    if let Some(Some(subschemas)) = parent.get("anyOf").map(Value::as_array) {
        let any_of_ctx = ctx.new_at_location("anyOf");
        let mut result = Vec::with_capacity(subschemas.len());

        for (idx, subschema) in subschemas.iter().enumerate() {
            if let Value::Object(parent) = subschema {
                let subschema_ctx = any_of_ctx.new_at_location(idx);
                let resource = subschema_ctx.as_resource_ref(subschema);
                let node = compiler::compile(&subschema_ctx, resource)
                    .map_err(ValidationError::to_owned)?;
                let inner_ctx = subschema_ctx
                    .in_subresource(resource)
                    .map_err(ValidationError::from)?;
                result.push((
                    node,
                    compile_items_validators(&inner_ctx, parent)
                        .map_err(ValidationError::to_owned)?,
                ));
            }
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

fn compile_one_of<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    parent: &'a Map<String, Value>,
) -> Result<Option<CompiledItemsSubschemas<F>>, ValidationError<'a>> {
    if let Some(Some(subschemas)) = parent.get("oneOf").map(Value::as_array) {
        let one_of_ctx = ctx.new_at_location("oneOf");
        let mut result = Vec::with_capacity(subschemas.len());

        for (idx, subschema) in subschemas.iter().enumerate() {
            if let Value::Object(parent) = subschema {
                let subschema_ctx = one_of_ctx.new_at_location(idx);
                let resource = subschema_ctx.as_resource_ref(subschema);
                let node = compiler::compile(&subschema_ctx, resource)
                    .map_err(ValidationError::to_owned)?;
                let inner_ctx = subschema_ctx
                    .in_subresource(resource)
                    .map_err(ValidationError::from)?;
                result.push((
                    node,
                    compile_items_validators(&inner_ctx, parent)
                        .map_err(ValidationError::to_owned)?,
                ));
            }
        }

        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Validator for the `unevaluatedItems` keyword.
pub(crate) struct UnevaluatedItemsValidator<F: Json = SerdeJson> {
    location: Location,
    validators: ItemsValidators<F>,
}

impl UnevaluatedItemsValidator {
    pub(crate) fn compile<'a, F: Json>(
        ctx: &'a compiler::Context<F>,
        parent: &'a Map<String, Value>,
    ) -> CompilationResult<'a, F> {
        let validators =
            compile_items_validators(ctx, parent).map_err(ValidationError::to_owned)?;

        Ok(Box::new(UnevaluatedItemsValidator {
            location: ctx.location().join("unevaluatedItems"),
            validators,
        }))
    }
}

impl<F: Json> Validate<F> for UnevaluatedItemsValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        if let Some(array) = instance.as_array() {
            let mut indexes = vec![false; array.len()];
            self.validators
                .mark_evaluated_indexes(instance, &mut indexes, ctx);

            for (item, is_evaluated) in array.elements().zip(indexes) {
                if !is_evaluated {
                    if let Some(validator) = &self.validators.unevaluated {
                        if !validator.is_valid(&item, ctx) {
                            return false;
                        }
                    } else {
                        // unevaluatedItems: false and item not evaluated
                        return false;
                    }
                }
            }
        }
        true
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        if let Some(array) = instance.as_array() {
            let mut indexes = vec![false; array.len()];
            self.validators
                .mark_evaluated_indexes(instance, &mut indexes, ctx);
            let mut unevaluated = vec![];

            for (item, is_evaluated) in array.elements().zip(indexes) {
                if !is_evaluated {
                    let is_valid = if let Some(validator) = &self.validators.unevaluated {
                        validator.is_valid(&item, ctx)
                    } else {
                        false
                    };

                    if !is_valid {
                        unevaluated.push(item.to_value().to_string());
                    }
                }
            }

            if !unevaluated.is_empty() {
                return Err(ValidationError::unevaluated_items(
                    self.location.clone(),
                    crate::paths::capture_evaluation_path(tracker, &self.location),
                    location.into(),
                    instance.lazy_value(),
                    unevaluated,
                ));
            }
        }
        Ok(())
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        if let Some(array) = instance.as_array() {
            let mut indexes = vec![false; array.len()];
            self.validators
                .mark_evaluated_indexes_by_other_keywords(instance, &mut indexes, ctx);
            let mut children = ChildList::default();
            let mut unevaluated = Vec::new();
            let mut invalid = false;

            for (idx, (item, is_evaluated)) in array.elements().zip(indexes.iter()).enumerate() {
                if *is_evaluated {
                    continue;
                }
                if let Some(validator) = &self.validators.unevaluated {
                    let child =
                        validator.evaluate_instance_below(&item, &location.push(idx), tracker, ctx);
                    if !child.valid {
                        invalid = true;
                        unevaluated.push(item.to_value().to_string());
                    }
                    children.push(&mut ctx.arena, child);
                } else {
                    invalid = true;
                    unevaluated.push(item.to_value().to_string());
                }
            }

            let mut errors = Vec::new();
            if !unevaluated.is_empty() {
                errors.push(ErrorDescription::from_validation_error(
                    &ValidationError::unevaluated_items(
                        self.location.clone(),
                        crate::paths::capture_evaluation_path(tracker, &self.location),
                        location.into(),
                        instance.lazy_value(),
                        unevaluated,
                    ),
                ));
            }

            if invalid {
                EvaluationResult::Invalid {
                    errors,
                    children,
                    annotations: None,
                }
            } else {
                EvaluationResult::Valid {
                    annotations: None,
                    children,
                }
            }
        } else {
            EvaluationResult::valid_empty()
        }
    }
}

pub(crate) fn compile<'a, F: Json>(
    ctx: &'a compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    match schema.as_bool() {
        Some(true) => None,
        _ => Some(UnevaluatedItemsValidator::compile(ctx, parent)),
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/unevaluated_items/tests_885.rs"));
