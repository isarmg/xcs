//! Building a JSON Schema validator.
//! The main idea is to create a tree from the input JSON Schema. This tree will contain
//! everything needed to perform such validation in runtime.
use std::{collections::hash_map::Entry, sync::Arc};

use crate::reference::Uri;
use crate::validation::{
    error::ErrorIterator,
    evaluation::{
        Annotations, ChildList, ErrorDescription, Evaluation, EvaluationArena, EvaluationNode,
    },
    node::SchemaNode,
    paths::{LazyLocation, Location, RefTracker},
    Draft, Json, JsonValueBackend, NodeIdentity, ValidationError, ValidationOptions,
};
use ahash::AHashMap;
use json_codec::Value;

// Re-export LazyEvaluationPath from paths module
pub(crate) use crate::validation::paths::LazyEvaluationPath;

#[derive(Default)]
struct EvaluationPathCache {
    prefix_identifiers: AHashMap<(usize, usize), usize>,
    paths: AHashMap<(usize, usize), Location>,
}

impl EvaluationPathCache {
    fn evaluation_path(&mut self, tracker: &RefTracker<'_>, location: &Location) -> Location {
        let prefix_identifier = self.prefix_identifier(tracker);
        let key = (prefix_identifier, location.as_ptr());
        match self.paths.entry(key) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let path = tracker.evaluation_path_with_prefix(tracker.prefix(), location);
                entry.insert(path.clone());
                path
            }
        }
    }

    fn prefix_identifier(&mut self, tracker: &RefTracker<'_>) -> usize {
        if let Some(identifier) = tracker.cached_prefix_identifier() {
            return identifier;
        }
        let parent_identifier = tracker
            .parent()
            .map_or(0, |parent| self.prefix_identifier(parent));
        let key = (parent_identifier, tracker.suffix().as_ptr());
        let next_identifier = self.prefix_identifiers.len() + 1;
        let identifier = match self.prefix_identifiers.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                entry.insert(next_identifier);
                next_identifier
            }
        };
        tracker.cache_prefix_identifier(identifier);
        identifier
    }
}

/// Validation state for cycle detection and memoization.
#[derive(Default)]
pub struct ValidationContext {
    validating: Vec<(usize, NodeIdentity)>,
    /// Stack of (validators, instance) pairs currently collecting evaluated properties/items,
    /// used to break cycles from self-referential `$dynamicRef`/`$recursiveRef` under
    /// `unevaluatedProperties`/`unevaluatedItems`.
    marking: Vec<(usize, NodeIdentity)>,
    /// Lazy-initialized cache for recursive schema validation.
    is_valid_cache: Option<AHashMap<(usize, NodeIdentity), bool>>,
    /// Lazy-initialized cache for ECMA regex transformation results during format "regex" validation.
    ecma_regex_cache: Option<AHashMap<String, bool>>,
    evaluation_path_cache: Option<Box<EvaluationPathCache>>,
    evaluation_path_calls: u32,
    /// Instance location of the node being evaluated; only the `evaluate` path sets it.
    instance_location: Option<Location>,
    /// Holds every node of the tree the `evaluate` path builds.
    pub(crate) arena: EvaluationArena,
}

/// Evaluation paths are only cached once this many have been built.
///
/// Small evaluations show little path reuse, so below this threshold the map costs more than it saves.
const EVALUATION_PATH_CACHE_THRESHOLD: u32 = 4096;

impl ValidationContext {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub(crate) fn instance_location(&self) -> Option<&Location> {
        self.instance_location.as_ref()
    }

    #[inline]
    pub(crate) fn enter_instance_location(&mut self, location: Location) -> Option<Location> {
        self.instance_location.replace(location)
    }

    #[inline]
    pub(crate) fn leave_instance_location(&mut self, previous: Option<Location>) {
        self.instance_location = previous;
    }

    /// Returns `true` if cycle detected. `None` identity disables cycle tracking for this node.
    #[inline]
    pub(crate) fn enter(
        &mut self,
        node_id: usize,
        instance_identity: Option<NodeIdentity>,
    ) -> bool {
        let Some(identity) = instance_identity else {
            return false;
        };
        let key = (node_id, identity);
        if self.validating.contains(&key) {
            return true;
        }
        self.validating.push(key);
        false
    }

    #[inline]
    pub(crate) fn exit(&mut self, node_id: usize, instance_identity: Option<NodeIdentity>) {
        let Some(identity) = instance_identity else {
            return;
        };
        let popped = self.validating.pop();
        debug_assert_eq!(
            popped,
            Some((node_id, identity)),
            "ValidationContext::exit called out of order"
        );
    }

    /// Returns `true` if this `(validators, instance)` pair is already being marked (cycle).
    #[inline]
    pub(crate) fn enter_marking(
        &mut self,
        validators_id: usize,
        instance_identity: Option<NodeIdentity>,
    ) -> bool {
        let Some(identity) = instance_identity else {
            return false;
        };
        let key = (validators_id, identity);
        if self.marking.contains(&key) {
            return true;
        }
        self.marking.push(key);
        false
    }

    #[inline]
    pub(crate) fn exit_marking(&mut self) {
        self.marking.pop();
    }

    /// Containers only: the cache outlives them, and only they promise an identity no later node
    /// reuses.
    #[inline]
    pub(crate) fn get_cached_result(
        &self,
        node_id: usize,
        identity: Option<NodeIdentity>,
    ) -> Option<bool> {
        let cache = self.is_valid_cache.as_ref()?;
        cache.get(&(node_id, identity?)).copied()
    }

    /// Containers only: the cache outlives them, and only they promise an identity no later node
    /// reuses.
    #[inline]
    pub(crate) fn cache_result(
        &mut self,
        node_id: usize,
        identity: Option<NodeIdentity>,
        result: bool,
    ) {
        let Some(identity) = identity else {
            return;
        };
        self.is_valid_cache
            .get_or_insert_with(AHashMap::new)
            .insert((node_id, identity), result);
    }
    /// Evaluation path for `location` under `tracker`, cached across instance nodes.
    ///
    /// Keying on addresses is sound because none of them can be freed and reused mid-run:
    /// keyword locations and `$ref` suffixes live in the compiled schema, and cached prefixes
    /// are owned by `eval_prefix_cache`.
    pub(crate) fn evaluation_path(
        &mut self,
        tracker: &RefTracker<'_>,
        location: &Location,
    ) -> Location {
        self.evaluation_path_calls = self.evaluation_path_calls.saturating_add(1);
        if self.evaluation_path_calls < EVALUATION_PATH_CACHE_THRESHOLD {
            return tracker.evaluation_path_with_prefix(tracker.prefix(), location);
        }
        self.evaluation_path_cache
            .get_or_insert_with(Box::default)
            .evaluation_path(tracker, location)
    }

    /// Check if an ECMA regex pattern is valid.
    pub(crate) fn is_valid_ecma_regex(&mut self, pattern: &str) -> bool {
        if let Some(cache) = &self.ecma_regex_cache {
            if let Some(&result) = cache.get(pattern) {
                return result;
            }
        }
        let result = crate::pattern::is_valid_ecma_regex(pattern);
        self.ecma_regex_cache
            .get_or_insert_with(AHashMap::new)
            .insert(pattern.to_owned(), result);
        result
    }
}

/// The Validate trait represents a predicate over some JSON value. Some validators are very simple
/// predicates such as "a value which is a string", whereas others may be much more complex,
/// consisting of several other validators composed together in various ways.
///
/// Much of the time all an application cares about is whether the predicate returns true or false,
/// in that case the `is_valid` function is sufficient. Sometimes applications will want more
/// detail about why a schema has failed, in which case the `validate` method can be used to
/// iterate over the errors produced by this validator. Finally, applications may be interested in
/// annotations produced by schemas over valid results, in this case the `evaluate` method can be used
/// to obtain this information.
///
/// If you are implementing `Validate` it is often sufficient to implement `validate` and
/// `is_valid`. `evaluate` is only necessary for validators which compose other validators. See the
/// documentation for `evaluate` for more information.
///
/// # Context Types
///
/// - `is_valid` takes `LightweightContext`: Only cycle detection, zero path tracking overhead.
/// - `validate`, `collect_errors`, `evaluate` take `ValidationContext`: Cycle detection + evaluation path tracking.
pub(crate) trait Validate<F: Json = JsonValueBackend>: Send + Sync {
    fn collect_errors<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
        errors: &mut Vec<ValidationError<'i>>,
    ) {
        if let Err(error) = self.validate(instance, location, tracker, ctx) {
            errors.push(error);
        }
    }

    fn is_valid(&self, instance: &F::Node<'_>, ctx: &mut ValidationContext) -> bool;

    fn validate<'i>(
        &self,
        instance: &F::Node<'i>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> Result<(), ValidationError<'i>>;

    /// `evaluate`, where the caller has already rendered this instance position.
    ///
    /// Rendering a JSON Pointer walks the whole location chain, and every schema node at one
    /// instance position renders the same one. Implementors that stay at that position should
    /// forward `instance_location` instead of rebuilding it.
    fn evaluate_with_location(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        instance_location: &Location,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let _ = instance_location;
        self.evaluate(instance, location, tracker, ctx)
    }

    fn evaluate(
        &self,
        instance: &F::Node<'_>,
        location: &LazyLocation,
        tracker: Option<&RefTracker>,
        ctx: &mut ValidationContext,
    ) -> EvaluationResult {
        let mut collected = Vec::new();
        self.collect_errors(instance, location, tracker, ctx, &mut collected);
        let errors: Vec<ErrorDescription> = collected
            .iter()
            .map(ErrorDescription::from_validation_error)
            .collect();
        if errors.is_empty() {
            EvaluationResult::valid_empty()
        } else {
            EvaluationResult::invalid_empty(errors)
        }
    }

    /// Returns the canonical location for this validator's schemaLocation output.
    ///
    /// Per JSON Schema spec, schemaLocation "MUST NOT include by-reference applicators
    /// such as `$ref` or `$dynamicRef`". For most validators, the keyword location is the
    /// canonical location, so this returns `None` by default.
    ///
    /// `RefValidator` and similar by-reference validators override this to return
    /// the target schema's location within its resource (e.g., `/$defs/item` instead of
    /// `/properties/foo/$ref`), together with its absolute URI when the resource has one.
    fn canonical_location(&self) -> Option<(&Location, Option<&Arc<Uri<String>>>)> {
        None
    }
}

/// `evaluate` for a keyword that asserts on the instance alone and annotates nothing: a valid
/// instance skips building errors.
pub(crate) fn evaluate_assertion<F: Json, V: Validate<F>>(
    validator: &V,
    instance: &F::Node<'_>,
    location: &LazyLocation,
    tracker: Option<&RefTracker>,
    ctx: &mut ValidationContext,
) -> EvaluationResult {
    if validator.is_valid(instance, ctx) {
        return EvaluationResult::valid_empty();
    }
    match validator.validate(instance, location, tracker, ctx) {
        Ok(()) => EvaluationResult::valid_empty(),
        Err(error) => {
            EvaluationResult::invalid_empty(vec![ErrorDescription::from_validation_error(&error)])
        }
    }
}

/// The result of evaluating a validator against an instance. This is a "partial" result because it does not include information about
/// where the error or annotation occurred.
#[derive(PartialEq)]
pub(crate) enum EvaluationResult {
    Valid {
        /// Annotations produced by this validator
        annotations: Option<Annotations>,
        /// Children evaluation nodes
        children: ChildList,
    },
    Invalid {
        /// Errors which caused this schema to be invalid
        errors: Vec<ErrorDescription>,
        /// Children evaluation nodes
        children: ChildList,
        /// Potential annotations that should be reported as dropped on failure
        annotations: Option<Annotations>,
    },
}

impl EvaluationResult {
    /// Create an empty `EvaluationResult` which is valid
    pub(crate) fn valid_empty() -> EvaluationResult {
        EvaluationResult::Valid {
            annotations: None,
            children: ChildList::default(),
        }
    }

    /// Create an empty `EvaluationResult` which is invalid
    pub(crate) fn invalid_empty(errors: Vec<ErrorDescription>) -> EvaluationResult {
        EvaluationResult::Invalid {
            errors,
            children: ChildList::default(),
            annotations: None,
        }
    }

    /// Set the annotation that will be returned for the current validator. If this
    /// `EvaluationResult` is invalid then this method does nothing
    pub(crate) fn annotate(&mut self, new_annotations: Annotations) {
        match self {
            Self::Valid { annotations, .. } | Self::Invalid { annotations, .. } => {
                *annotations = Some(new_annotations);
            }
        }
    }

    /// Set the error that will be returned for the current validator. If this
    /// `EvaluationResult` is valid then this method converts this application into
    /// `EvaluationResult::Invalid`
    pub(crate) fn mark_errored(&mut self, error: ErrorDescription) {
        match self {
            Self::Invalid { errors, .. } => errors.push(error),
            Self::Valid {
                annotations,
                children,
            } => {
                *self = Self::Invalid {
                    errors: vec![error],
                    children: std::mem::take(children),
                    annotations: annotations.take(),
                }
            }
        }
    }

    pub(crate) fn from_children(children: ChildList) -> EvaluationResult {
        if children.all_valid() {
            EvaluationResult::Valid {
                annotations: None,
                children,
            }
        } else {
            EvaluationResult::Invalid {
                errors: Vec::new(),
                children,
                annotations: None,
            }
        }
    }
}

impl EvaluationResult {
    pub(crate) fn from_node(arena: &mut EvaluationArena, node: EvaluationNode) -> Self {
        let valid = node.valid;
        let children = ChildList::of(arena, node);
        if valid {
            EvaluationResult::Valid {
                annotations: None,
                children,
            }
        } else {
            EvaluationResult::Invalid {
                errors: Vec::new(),
                children,
                annotations: None,
            }
        }
    }
}

/// A compiled JSON Schema validator.
///
/// This structure represents a JSON Schema that has been parsed and compiled into
/// an efficient internal representation for validation. It contains the root node
/// of the schema tree and the configuration options used during compilation.
pub struct Validator<F: Json = JsonValueBackend> {
    pub(crate) root: SchemaNode<F>,
    /// `$ref` targets compiled off the call stack. Owning them here drops a long `$ref` chain
    /// one target at a time; `root` drops first, so each target is released from this list.
    pub(crate) targets: Vec<SchemaNode<F>>,
    pub(crate) draft: Draft,
}

impl<F: Json> Clone for Validator<F> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            targets: self.targets.clone(),
            draft: self.draft,
        }
    }
}

impl<F: Json> std::fmt::Debug for Validator<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Validator")
            .field("root", &self.root)
            .field("draft", &self.draft)
            .finish_non_exhaustive()
    }
}

impl Validator<JsonValueBackend> {
    /// Create a default [`ValidationOptions`] for configuring JSON Schema validation.
    ///
    /// Use this to set the draft version and other validation parameters.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use schema_validation::Draft;
    /// # let schema = json_codec::json!({});
    /// let validator = schema_validation::options()
    ///     .with_draft(Draft::Draft7)
    ///     .build(&schema);
    /// ```
    #[must_use]
    pub fn options<'i>() -> ValidationOptions<'i> {
        ValidationOptions::default()
    }
    /// Create a default [`ValidationOptions`] configured for async validation.
    ///
    /// Use this to set the draft version and other validation parameters when working
    /// with schemas that require async reference resolution.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use json_codec::json;
    /// # use schema_validation::Draft;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let schema = json!({
    ///     "$ref": "https://example.com/schema.json"
    /// });
    ///
    /// let validator = schema_validation::async_options()
    ///     .with_draft(Draft::Draft202012)
    ///     .build(&schema)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// For sync validation, use [`options()`](schema_validation::options()) instead.
    #[cfg(feature = "resolve-async")]
    #[must_use]
    pub fn async_options<'i>(
    ) -> ValidationOptions<'i, std::sync::Arc<dyn crate::reference::AsyncRetrieve>> {
        ValidationOptions::default()
    }
    /// Create a validator using the default options.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied `schema` is invalid for the selected draft or references cannot be resolved.
    pub fn new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        Self::options().build(schema)
    }
    /// Create a validator using the default async options.
    #[cfg(feature = "resolve-async")]
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied `schema` is invalid for the selected draft or references cannot be resolved.
    pub async fn async_new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        Self::async_options().build(schema).await
    }
}

// `F::Node` is a borrow handle (`&Value` for `serde_json`); taking it by value keeps the serde
// call ergonomics (`&Value`, not `&&Value`) across every representation.
#[allow(clippy::needless_pass_by_value)]
impl<F: Json> Validator<F> {
    /// Validate `instance` against `schema` and return the first error if any.
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] describing why `instance` does not satisfy the schema.
    #[inline]
    pub fn validate<'i>(&self, instance: F::Node<'i>) -> Result<(), ValidationError<'i>> {
        let mut ctx = ValidationContext::new();
        self.root
            .validate(&instance, &LazyLocation::new(), None, &mut ctx)
    }
    /// Run validation against `instance` and return an iterator over [`ValidationError`] in the error case.
    #[inline]
    #[must_use]
    pub fn iter_errors<'i>(&'i self, instance: F::Node<'i>) -> ErrorIterator<'i> {
        let mut ctx = ValidationContext::new();
        let mut errors = Vec::new();
        self.root
            .collect_errors(&instance, &LazyLocation::new(), None, &mut ctx, &mut errors);
        ErrorIterator::from_iterator(errors.into_iter())
    }
    /// Run validation against `instance` but return a boolean result instead of an iterator.
    /// It is useful for cases, where it is important to only know the fact if the data is valid or not.
    /// This approach is much faster, than [`Validator::validate`].
    #[must_use]
    #[inline]
    pub fn is_valid(&self, instance: F::Node<'_>) -> bool {
        let mut ctx = ValidationContext::new();
        self.root.is_valid(&instance, &mut ctx)
    }
    /// Evaluate the schema and expose structured output formats.
    #[must_use]
    #[inline]
    pub fn evaluate(&self, instance: F::Node<'_>) -> Evaluation {
        let mut ctx = ValidationContext::new();
        let root = self
            .root
            .evaluate_instance(&instance, &LazyLocation::new(), None, &mut ctx);
        let root = ctx.arena.push(root);
        Evaluation::new(std::mem::take(&mut ctx.arena), root)
    }
    /// The [`Draft`] this validator applies: the one set via `with_draft`, else the one `$schema`
    /// declares, else the default.
    #[must_use]
    pub fn draft(&self) -> Draft {
        self.draft
    }
}

/// A map of compiled JSON Schema validators keyed by URI-fragment JSON pointers.
///
/// Built via [`ValidationOptions::build_map`] or the [`validator_map_for`](schema_validation::validator_map_for)
/// convenience function.
///
/// Each key is a URI-fragment JSON pointer (e.g. `"#"`, `"#/$defs/User"`).
/// The root schema is always present under the key `"#"`.
pub struct ValidatorMap<F: Json = JsonValueBackend> {
    pub(crate) validators: AHashMap<String, Validator<F>>,
}

impl<F: Json> std::fmt::Debug for ValidatorMap<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorMap")
            .field("validators", &self.validators)
            .finish()
    }
}

impl<F: Json> ValidatorMap<F> {
    /// Returns the validator for the given URI-fragment pointer, or `None` if not found.
    #[must_use]
    pub fn get(&self, pointer: &str) -> Option<&Validator<F>> {
        self.validators.get(pointer)
    }

    /// Returns `true` if the map contains a validator for the given pointer.
    #[must_use]
    pub fn contains_key(&self, pointer: &str) -> bool {
        self.validators.contains_key(pointer)
    }

    /// Returns an iterator over all URI-fragment pointers in the map.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.validators.keys().map(String::as_str)
    }

    /// Returns the number of compiled subschema validators in the map.
    #[must_use]
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Returns `true` if the map contains no validators.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }
}

impl<F: Json> std::ops::Index<&str> for ValidatorMap<F> {
    type Output = Validator<F>;

    /// # Panics
    ///
    /// Panics if the pointer is not found in the map.
    fn index(&self, pointer: &str) -> &Validator<F> {
        self.validators
            .get(pointer)
            .unwrap_or_else(|| panic!("JSON pointer '{pointer}' not found in ValidatorMap"))
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/validator/tests_655.rs"
));
