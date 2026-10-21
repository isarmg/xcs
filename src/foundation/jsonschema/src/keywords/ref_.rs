use crate::LazyInstance;
use std::borrow::Cow;

use crate::{
    compiler::{self, TargetNode},
    keywords::{BoxedValidator, CompilationResult},
    paths::{LazyLocation, Location, RefTracker},
    types::JsonType,
    validator::{EvaluationResult, Validate, ValidationContext},
    Json, ValidationError,
};
use referencing::{unescape_segment, uri, Resolved, Uri};
use json_codec::{Map, Value};
use std::sync::Arc;

/// Tracks `$ref` traversals for recursive references where the target is behind `BoxedValidator<F>`
/// (either a `PendingSchemaNode` or a cached node returned by `lookup_maybe_recursive`).
struct RefValidator<F: Json> {
    inner: BoxedValidator<F>,
    /// Path of this `$ref` keyword relative to its resource base.
    /// E.g., `/properties/foo/$ref` (not the full canonical path).
    /// Used for building the `tracker` prefix.
    ref_suffix: Location,
    /// The resource base of the `$ref` target.
    /// E.g., `/$defs/Item` when `$ref` points to `#/$defs/Item`.
    /// Used for computing validator suffixes at runtime.
    ref_target_base: Location,
    /// Absolute URI of the `$ref` target, if its resource has one.
    target_location: Option<Arc<Uri<String>>>,
}

impl<F: Json> Validate<F> for RefValidator<F> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        self.inner.is_valid(instance, ctx)
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .validate(instance, location, Some(&child_tracker), ctx)
    }

    fn collect_errors<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
        errors: &mut Vec<ValidationError<'i>>,
    ) {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .collect_errors(instance, location, Some(&child_tracker), ctx, errors);
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        self.evaluate_with_location(instance, location, &location.into(), tracker, ctx)
    }

    fn evaluate_with_location(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        instance_location: &Location,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner.evaluate_with_location(
            instance,
            location,
            instance_location,
            Some(&child_tracker),
            ctx,
        )
    }

    /// Returns `ref_target_base` for `schema_path` output.
    ///
    /// Per JSON Schema 2020-12 Core Section 12.4.2, `schema_path` "MUST NOT include
    /// by-reference applicators such as `$ref` or `$dynamicRef`".
    fn canonical_location(&self) -> Option<(&Location, Option<&Arc<Uri<String>>>)> {
        Some((&self.ref_target_base, self.target_location.as_ref()))
    }
}

/// Like `RefValidator` but holds a concrete target instead of `BoxedValidator<F>`,
/// eliminating one layer of vtable dispatch on every validation call.
/// Used for non-recursive refs: a compiled node, or a placeholder for a deferred one.
struct DirectRefValidator<T> {
    inner: T,
    ref_suffix: Location,
    ref_target_base: Location,
    target_location: Option<Arc<Uri<String>>>,
}

impl<F: Json, T: Validate<F>> Validate<F> for DirectRefValidator<T> {
    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool {
        self.inner.is_valid(instance, ctx)
    }

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>> {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .validate(instance, location, Some(&child_tracker), ctx)
    }

    fn collect_errors<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
        errors: &mut Vec<ValidationError<'i>>,
    ) {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner
            .collect_errors(instance, location, Some(&child_tracker), ctx, errors);
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        self.evaluate_with_location(instance, location, &location.into(), tracker, ctx)
    }

    fn evaluate_with_location(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        instance_location: &Location,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let child_tracker = RefTracker::new(&self.ref_suffix, &self.ref_target_base, tracker);
        self.inner.evaluate_with_location(
            instance,
            location,
            instance_location,
            Some(&child_tracker),
            ctx,
        )
    }

    fn canonical_location(&self) -> Option<(&Location, Option<&Arc<Uri<String>>>)> {
        Some((&self.ref_target_base, self.target_location.as_ref()))
    }
}

/// Extract `ref_target_base` from a resolved URI fragment.
///
/// JSON Pointer fragments (starting with `/`) become the location path.
/// Anchor fragments (plain names like `#node`) resolve to root.
fn extract_ref_target_base(alias: &referencing::Uri<String>) -> Location {
    if let Some(fragment) = alias.fragment() {
        let fragment = fragment.as_str();
        if fragment.starts_with('/') {
            // Fragment is URI percent-encoded (RFC 3986); Location stores JSON Pointers (RFC 6901).
            let decoded = percent_encoding::percent_decode_str(fragment).decode_utf8_lossy();
            return Location::from_escaped(&decoded);
        }
    }
    Location::new()
}

/// Absolute URI of the target at `location` within the resource `base` names.
///
/// `None` when that resource has no URI of its own, i.e. only the default base URI.
fn target_location(base: &Uri<String>, location: &Location) -> Option<Arc<Uri<String>>> {
    if base.scheme().as_str() == compiler::DEFAULT_SCHEME {
        return None;
    }
    let mut fragment = uri::EncodedBuffer::new();
    fragment.encode_str::<uri::Path>(location.as_str());
    Some(Arc::new(base.with_fragment(Some(fragment.as_estr()))))
}

/// Whether `alias` names its target by an anchor rather than a JSON Pointer.
fn is_named(alias: &referencing::Uri<String>) -> bool {
    alias.fragment().is_some_and(|fragment| {
        !fragment.as_str().is_empty() && !fragment.as_str().starts_with('/')
    })
}

/// Location of a named target within its resource.
fn named_target_base<F: Json>(
    ctx: &compiler::Context<F>,
    resolved: &Resolved<'_>,
) -> Result<Location, referencing::Error> {
    let resolver = resolved.resolver();
    let resource = resolver.lookup("")?;
    Ok(ctx
        .anchor_location(resource.contents(), resolved.contents())
        .expect("A named target lives in its resolver's resource"))
}

/// Length of the `pointer` prefix leading to the innermost `$id` subschema it passes through.
///
/// Zero when the pointer stays in the resource `alias` names.
fn embedded_resource_start<F: Json>(
    ctx: &compiler::Context<F>,
    alias: &Uri<String>,
    pointer: &Location,
    resolved: &Resolved<'_>,
) -> Result<usize, referencing::Error> {
    let target_base = resolved.resolver().base_uri();
    if alias
        .as_str()
        .strip_prefix(target_base.as_str())
        .is_some_and(|fragment| fragment.is_empty() || fragment.starts_with('#'))
    {
        return Ok(0);
    }
    let alias_base = alias
        .as_str()
        .split_once('#')
        .map_or(alias.as_str(), |(base, _)| base);
    let resource = resolved.resolver().lookup("")?.contents();
    let document = ctx.lookup(alias_base)?;
    let mut node = document.contents();
    let mut offset = 0;
    let mut start = 0;
    for segment in pointer.as_str().split('/').skip(1) {
        offset += 1 + segment.len();
        let next = match node {
            Value::Object(map) => map.get(unescape_segment(segment).as_ref()),
            Value::Array(items) => segment.parse::<usize>().ok().and_then(|idx| items.get(idx)),
            _ => None,
        };
        let Some(next) = next else {
            break;
        };
        node = next;
        if std::ptr::eq(node, resource) {
            start = offset;
        }
    }
    Ok(start)
}

fn compile_reference_validator<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &Map<String, Value>,
    reference: &str,
    keyword: &str,
) -> Option<CompilationResult<'a, F>> {
    // An empty reference ("" per RFC 3986) targets the enclosing resource - skip to avoid
    // infinite recursion.
    if reference.is_empty() {
        return None;
    }
    let (alias, ref_target_base) = match ctx
        .ref_target(reference, extract_ref_target_base)
        .map_err(ValidationError::from)
    {
        Ok(target) => target,
        Err(error) => return Some(Err(error)),
    };

    let ref_suffix = ctx.suffix().join(keyword);

    let resolved = match ctx.lookup(reference) {
        Ok(resolved) => resolved,
        Err(error) => return Some(Err(ValidationError::from(error))),
    };
    // A pointer counts from the resource the reference names, an anchor from the one it is in.
    let (ref_target_base, target_location, resource_start) = if is_named(&alias) {
        match named_target_base(ctx, &resolved) {
            Ok(location) => {
                let target = target_location(&resolved.resolver().base_uri(), &location);
                (location, target, 0)
            }
            Err(error) => return Some(Err(ValidationError::from(error))),
        }
    } else {
        match embedded_resource_start(ctx, &alias, &ref_target_base, &resolved) {
            Ok(0) => {
                let target = target_location(&alias, &ref_target_base);
                (ref_target_base, target, 0)
            }
            Ok(start) => {
                let within = Location::from_escaped(&ref_target_base.as_str()[start..]);
                let target = target_location(&resolved.resolver().base_uri(), &within);
                (ref_target_base, target, start)
            }
            Err(error) => return Some(Err(ValidationError::from(error))),
        }
    };

    // Direct self-reference - skip to avoid infinite recursion. This compares node identity
    // rather than URIs because the location pointer is relative to the enclosing document
    // while the base URI may come from an `$id`-bearing subresource; composing the two
    // yields a URI that does not denote this schema and can collide with the target's.
    if resolved
        .contents()
        .as_object()
        .is_some_and(|target| std::ptr::eq(target, parent))
    {
        return None;
    }

    match ctx.lookup_maybe_recursive(reference, resolved.resolver()) {
        Ok(Some(validator)) => {
            return Some(Ok(Box::new(RefValidator {
                inner: validator,
                ref_suffix,
                ref_target_base,
                target_location,
            })));
        }
        Ok(None) => {}
        Err(error) => return Some(Err(error)),
    }

    if let Err(error) = ctx.mark_seen(reference) {
        return Some(Err(ValidationError::from(error)));
    }

    if ctx.defers_ref_target(&resolved, &alias) {
        let placeholder = ctx.defer_ref_target(resolved, ref_target_base.clone(), alias);
        return Some(Ok(Box::new(DirectRefValidator {
            inner: placeholder,
            ref_suffix,
            ref_target_base,
            target_location,
        })));
    }

    let (contents, resolver, draft) = resolved.into_inner();
    let vocabularies = resolver.find_vocabularies(draft, contents);
    let resource_ref = draft.create_resource_ref(contents);
    let inner_ctx = match ctx.with_resolver_and_draft(
        resolver,
        resource_ref.draft(),
        vocabularies,
        ref_target_base.clone(),
    ) {
        Ok(inner_ctx) => inner_ctx.with_resource_start(resource_start),
        Err(error) => return Some(Err(error)),
    };
    Some(
        ctx.compile_ref_target(|| compiler::compile_with_alias(&inner_ctx, resource_ref, alias))
            .map(|target| match target {
                TargetNode::Owned(node) => Box::new(DirectRefValidator {
                    inner: node,
                    ref_suffix,
                    ref_target_base,
                    target_location,
                }) as Box<dyn Validate<F>>,
                TargetNode::Shared(_) => Box::new(RefValidator {
                    inner: target.into_validator(),
                    ref_suffix,
                    ref_target_base,
                    target_location,
                }),
            })
            .map_err(ValidationError::to_owned),
    )
}

fn compile_recursive_validator<'a, F: Json>(
    ctx: &compiler::Context<F>,
    reference: &str,
) -> CompilationResult<'a, F> {
    let ref_suffix = ctx.suffix().join("$recursiveRef");
    let (alias, ref_target_base) = ctx
        .ref_target(reference, extract_ref_target_base)
        .map_err(ValidationError::from)?;
    let resolved = ctx
        .lookup_recursive_reference()
        .map_err(ValidationError::from)?;
    let target_location = target_location(&resolved.resolver().base_uri(), &ref_target_base);

    match ctx.lookup_maybe_recursive(reference, resolved.resolver()) {
        Ok(Some(validator)) => {
            return Ok(Box::new(RefValidator {
                inner: validator,
                ref_suffix,
                ref_target_base,
                target_location,
            }));
        }
        Ok(None) => {}
        Err(error) => return Err(error),
    }

    if let Err(error) = ctx.mark_seen(reference) {
        return Err(ValidationError::from(error));
    }

    let (contents, resolver, draft) = resolved.into_inner();
    let vocabularies = resolver.find_vocabularies(draft, contents);
    let resource_ref = draft.create_resource_ref(contents);
    let target_base = ref_target_base.clone();
    let inner_ctx =
        ctx.with_resolver_and_draft(resolver, resource_ref.draft(), vocabularies, target_base)?;
    compiler::compile_with_alias(&inner_ctx, resource_ref, alias)
        .map(|target| {
            Box::new(RefValidator {
                inner: target.into_validator(),
                ref_suffix,
                ref_target_base,
                target_location,
            }) as Box<dyn Validate<F>>
        })
        .map_err(ValidationError::to_owned)
}

fn invalid_reference<'a, F: Json>(
    ctx: &compiler::Context<F>,
    keyword: &str,
    schema: &'a Value,
) -> ValidationError<'a> {
    let location = ctx.location().join(keyword);
    ValidationError::single_type_error(
        location.clone(),
        location.clone(),
        location,
        LazyInstance::Ready(Cow::Borrowed(schema)),
        JsonType::String,
    )
}

#[inline]
pub(crate) fn compile_impl<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
    keyword: &str,
) -> Option<CompilationResult<'a, F>> {
    if let Some(reference) = schema.as_str() {
        compile_reference_validator(ctx, parent, reference, keyword)
    } else {
        Some(Err(invalid_reference(ctx, keyword, schema)))
    }
}

#[inline]
pub(crate) fn compile_dynamic_ref<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    compile_impl(ctx, parent, schema, "$dynamicRef")
}

#[inline]
pub(crate) fn compile_ref<'a, F: Json>(
    ctx: &compiler::Context<F>,
    parent: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    compile_impl(ctx, parent, schema, "$ref")
}

#[inline]
pub(crate) fn compile_recursive_ref<'a, F: Json>(
    ctx: &compiler::Context<F>,
    _: &'a Map<String, Value>,
    schema: &'a Value,
) -> Option<CompilationResult<'a, F>> {
    Some(
        schema
            .as_str()
            .ok_or_else(|| invalid_reference(ctx, "$recursiveRef", schema))
            .and_then(|reference| compile_recursive_validator(ctx, reference)),
    )
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/keywords/ref_/tests_492.rs"));
