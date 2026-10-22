use crate::reference::Uri;
use crate::validation::{
    paths::{LazyLocation, Location, RefTracker},
    validator::ValidationContext,
    ValidationError,
};
use ahash::AHashMap;
use data_serialization::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct},
    Serialize,
};
use std::{fmt, sync::Arc};

/// Annotations associated with an output unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotations(Arc<json_codec::Value>);

impl Annotations {
    /// Create a new `Annotations` instance.
    #[must_use]
    pub(crate) fn new(v: json_codec::Value) -> Self {
        Annotations(Arc::new(v))
    }

    /// Create a new `Annotations` instance from an Arc.
    #[must_use]
    pub(crate) fn from_arc(v: Arc<json_codec::Value>) -> Self {
        Annotations(v)
    }

    /// Returns the inner [`json_codec::Value`] of the annotation.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> json_codec::Value {
        Arc::try_unwrap(self.0).unwrap_or_else(|arc| (*arc).clone())
    }

    /// The `json_codec::Value` of the annotation.
    #[must_use]
    pub fn value(&self) -> &json_codec::Value {
        &self.0
    }
}

impl data_serialization::Serialize for Annotations {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        self.0.serialize(serializer)
    }
}

/// Description of a validation error used within evaluation outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDescription {
    keyword: String,
    message: String,
}

impl ErrorDescription {
    #[inline]
    #[must_use]
    pub(crate) fn new(keyword: impl Into<String>, message: String) -> Self {
        Self {
            keyword: keyword.into(),
            message,
        }
    }

    /// Create an `ErrorDescription` from a `ValidationError`.
    #[inline]
    #[must_use]
    pub(crate) fn from_validation_error(e: &ValidationError<'_>) -> Self {
        ErrorDescription {
            keyword: e.kind().keyword().to_owned(),
            message: e.to_string(),
        }
    }

    /// Returns the keyword associated with this error.
    #[inline]
    #[must_use]
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// Returns the inner [`String`] of the error description.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> String {
        self.message
    }

    /// Returns the message of the error description.
    #[inline]
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ErrorDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

pub(crate) const NO_NODE: u32 = u32::MAX;

#[derive(Debug, PartialEq)]
pub(crate) struct EvaluationNode {
    pub(crate) keyword_location: Location,
    pub(crate) absolute_keyword_location: Option<Arc<Uri<String>>>,
    pub(crate) schema_location: Arc<str>,
    pub(crate) instance_location: Location,
    pub(crate) valid: bool,
    pub(crate) annotations: Option<Annotations>,
    pub(crate) dropped_annotations: Option<Annotations>,
    pub(crate) errors: Vec<ErrorDescription>,
    pub(crate) first_child: u32,
    pub(crate) next_sibling: u32,
}

/// Nodes per chunk. A power of two so the index split is a shift and a mask.
const CHUNK: usize = 32;

/// Every node of one evaluation, in chunks that are never resized, so growth copies nothing.
///
/// Children are a sibling chain rather than a `Vec` per node: subtrees are finished in the order
/// they are evaluated, so a parent cannot know where its children sit until they all exist.
#[derive(Debug, Default)]
pub(crate) struct EvaluationArena {
    chunks: Vec<Vec<EvaluationNode>>,
    len: usize,
}

impl EvaluationArena {
    #[inline]
    pub(crate) fn push(&mut self, node: EvaluationNode) -> u32 {
        let index = u32::try_from(self.len).expect("evaluation exceeded u32 nodes");
        if self.len % CHUNK == 0 {
            self.chunks.push(Vec::with_capacity(CHUNK));
        }
        self.chunks
            .last_mut()
            .expect("a chunk was just reserved")
            .push(node);
        self.len += 1;
        index
    }

    #[inline]
    pub(crate) fn node(&self, index: u32) -> &EvaluationNode {
        let index = index as usize;
        &self.chunks[index / CHUNK][index % CHUNK]
    }

    #[inline]
    fn node_mut(&mut self, index: u32) -> &mut EvaluationNode {
        let index = index as usize;
        &mut self.chunks[index / CHUNK][index % CHUNK]
    }

    pub(crate) fn child_indices(&self, index: u32) -> ChildIndices<'_> {
        ChildIndices {
            arena: self,
            current: self.node(index).first_child,
        }
    }
}

pub(crate) struct ChildIndices<'a> {
    arena: &'a EvaluationArena,
    current: u32,
}

impl Iterator for ChildIndices<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.current == NO_NODE {
            return None;
        }
        let index = self.current;
        self.current = self.arena.node(index).next_sibling;
        Some(index)
    }
}

/// The children collected for one node, as a chain into [`EvaluationArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChildList {
    first: u32,
    last: u32,
    len: u32,
    /// Whether every child pushed so far is valid.
    valid: bool,
}

impl Default for ChildList {
    fn default() -> Self {
        ChildList {
            first: NO_NODE,
            last: NO_NODE,
            len: 0,
            valid: true,
        }
    }
}

impl ChildList {
    #[inline]
    pub(crate) fn push(&mut self, arena: &mut EvaluationArena, node: EvaluationNode) {
        self.valid &= node.valid;
        let index = arena.push(node);
        if self.last == NO_NODE {
            self.first = index;
        } else {
            arena.node_mut(self.last).next_sibling = index;
        }
        self.last = index;
        self.len += 1;
    }

    pub(crate) fn len(self) -> usize {
        self.len as usize
    }

    pub(crate) fn all_valid(self) -> bool {
        self.valid
    }

    /// A list of nodes that were collected before the arena was reachable.
    #[cfg(test)]
    pub(crate) fn from_nodes(
        arena: &mut EvaluationArena,
        nodes: impl IntoIterator<Item = EvaluationNode>,
    ) -> Self {
        let mut list = ChildList::default();
        for node in nodes {
            list.push(arena, node);
        }
        list
    }

    /// A list holding `node` alone.
    #[inline]
    pub(crate) fn of(arena: &mut EvaluationArena, node: EvaluationNode) -> Self {
        let mut list = ChildList::default();
        list.push(arena, node);
        list
    }
}

impl EvaluationNode {
    pub(crate) fn valid(
        keyword_location: Location,
        absolute_keyword_location: Option<Arc<Uri<String>>>,
        schema_location: impl Into<Arc<str>>,
        instance_location: Location,
        annotations: Option<Annotations>,
        children: ChildList,
    ) -> Self {
        let schema_location = schema_location.into();
        EvaluationNode {
            keyword_location,
            absolute_keyword_location,
            schema_location,
            instance_location,
            valid: true,
            annotations,
            dropped_annotations: None,
            errors: Vec::new(),
            first_child: children.first,
            next_sibling: NO_NODE,
        }
    }

    pub(crate) fn invalid(
        keyword_location: Location,
        absolute_keyword_location: Option<Arc<Uri<String>>>,
        schema_location: impl Into<Arc<str>>,
        instance_location: Location,
        annotations: Option<Annotations>,
        errors: Vec<ErrorDescription>,
        children: ChildList,
    ) -> Self {
        let schema_location = schema_location.into();
        EvaluationNode {
            keyword_location,
            absolute_keyword_location,
            schema_location,
            instance_location,
            valid: false,
            annotations: None,
            dropped_annotations: annotations,
            errors,
            first_child: children.first,
            next_sibling: NO_NODE,
        }
    }
}

/// Result of evaluating a JSON instance against a schema.
///
/// This type provides access to structured output formats as defined in the
/// [JSON Schema specification](https://json-schema.org/draft/2020-12/json-schema-core#name-output-structure).
///
/// # Output Formats
///
/// The evaluation result can be accessed in three standard formats:
///
/// - **Flag**: Simple boolean validity indicator via [`flag()`](Self::flag)
/// - **List**: Flat list of all evaluation units via [`list()`](Self::list)
/// - **Hierarchical**: Nested tree structure via [`hierarchical()`](Self::hierarchical)
///
/// All formats are serializable to JSON using `serde_json`.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({"type": "string", "minLength": 3});
/// let validator = schema_validation::validator_for(&schema)?;
///
/// // Evaluate an instance
/// let instance = json!("ab");
/// let evaluation = validator.evaluate(&instance);
///
/// // Check validity with flag format
/// let flag = evaluation.flag();
/// assert!(!flag.valid);
///
/// // Get structured output as JSON
/// let list_output = json_codec::to_value(evaluation.list())?;
/// println!("{}", json_codec::to_string_pretty(&list_output)?);
///
/// // Iterate over errors
/// for error in evaluation.iter_errors() {
///     println!("Error at {}: {}", error.instance_location, error.error);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Evaluation {
    arena: EvaluationArena,
    root: u32,
}

impl Evaluation {
    pub(crate) fn new(arena: EvaluationArena, root: u32) -> Self {
        Evaluation { arena, root }
    }

    #[cfg(test)]
    fn with_root(mut arena: EvaluationArena, root: EvaluationNode) -> Self {
        let root = arena.push(root);
        Evaluation::new(arena, root)
    }

    fn root(&self) -> &EvaluationNode {
        self.arena.node(self.root)
    }
    /// Returns the flag output format.
    ///
    /// This is the simplest output format, containing only a boolean indicating
    /// whether the instance is valid according to the schema.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "number"});
    /// let validator = schema_validation::validator_for(&schema)?;
    ///
    /// let evaluation = validator.evaluate(&json!(42));
    /// let flag = evaluation.flag();
    /// assert!(flag.valid);
    ///
    /// let evaluation = validator.evaluate(&json!("not a number"));
    /// let flag = evaluation.flag();
    /// assert!(!flag.valid);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn flag(&self) -> FlagOutput {
        FlagOutput {
            valid: self.root().valid,
        }
    }
    /// Whether the instance is valid against the schema.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let validator = schema_validation::validator_for(&json!({"type": "number"}))?;
    ///
    /// assert!(validator.evaluate(&json!(42)).is_valid());
    /// assert!(!validator.evaluate(&json!("oops")).is_valid());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.root().valid
    }
    /// Returns the list output format.
    ///
    /// This format provides a flat list of all evaluation units, where each unit
    /// contains information about a specific validation step including its location,
    /// validity, annotations, and errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "array",
    ///     "prefixItems": [{"type": "string"}],
    ///     "items": {"type": "integer"}
    /// });
    /// let validator = schema_validation::validator_for(&schema)?;
    /// let evaluation = validator.evaluate(&json!(["hello", "oops"]));
    ///
    /// assert_eq!(
    ///     json_codec::to_value(evaluation.list())?,
    ///     json!({
    ///         "valid": false,
    ///         "details": [
    ///             {"evaluationPath": "", "instanceLocation": "", "schemaLocation": "", "valid": false},
    ///             {
    ///                 "valid": true,
    ///                 "evaluationPath": "/type",
    ///                 "instanceLocation": "",
    ///                 "schemaLocation": "/type"
    ///             },
    ///             {
    ///                 "valid": false,
    ///                 "evaluationPath": "/items",
    ///                 "instanceLocation": "",
    ///                 "schemaLocation": "/items",
    ///                 "droppedAnnotations": true
    ///             },
    ///             {
    ///                 "valid": false,
    ///                 "evaluationPath": "/items",
    ///                 "instanceLocation": "/1",
    ///                 "schemaLocation": "/items"
    ///             },
    ///             {
    ///                 "valid": false,
    ///                 "evaluationPath": "/items/type",
    ///                 "instanceLocation": "/1",
    ///                 "schemaLocation": "/items/type",
    ///                 "errors": {"type": "\"oops\" is not of type \"integer\""}
    ///             },
    ///             {
    ///                 "valid": true,
    ///                 "evaluationPath": "/prefixItems",
    ///                 "instanceLocation": "",
    ///                 "schemaLocation": "/prefixItems",
    ///                 "annotations": 0
    ///             },
    ///             {
    ///                 "valid": true,
    ///                 "evaluationPath": "/prefixItems/0",
    ///                 "instanceLocation": "/0",
    ///                 "schemaLocation": "/prefixItems/0"
    ///             },
    ///             {
    ///                 "valid": true,
    ///                 "evaluationPath": "/prefixItems/0/type",
    ///                 "instanceLocation": "/0",
    ///                 "schemaLocation": "/prefixItems/0/type"
    ///             }
    ///         ]
    ///     })
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn list(&self) -> ListOutput<'_> {
        ListOutput {
            arena: &self.arena,
            root: self.root,
        }
    }
    /// Returns the hierarchical output format.
    ///
    /// This format represents the evaluation as a tree structure that mirrors the
    /// schema's logical structure. Each node contains its validation result along
    /// with nested child nodes representing sub-schema evaluations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "array",
    ///     "prefixItems": [{"type": "string"}],
    ///     "items": {"type": "integer"}
    /// });
    /// let validator = schema_validation::validator_for(&schema)?;
    /// let evaluation = validator.evaluate(&json!(["hello", "oops"]));
    ///
    /// assert_eq!(
    ///     json_codec::to_value(evaluation.hierarchical())?,
    ///     json!({
    ///         "valid": false,
    ///         "evaluationPath": "",
    ///         "schemaLocation": "",
    ///         "instanceLocation": "",
    ///         "details": [
    ///             {
    ///                 "valid": true,
    ///                 "evaluationPath": "/type",
    ///                 "instanceLocation": "",
    ///                 "schemaLocation": "/type"
    ///             },
    ///             {
    ///                 "valid": false,
    ///                 "evaluationPath": "/items",
    ///                 "instanceLocation": "",
    ///                 "schemaLocation": "/items",
    ///                 "droppedAnnotations": true,
    ///                 "details": [
    ///                     {
    ///                         "valid": false,
    ///                         "evaluationPath": "/items",
    ///                         "instanceLocation": "/1",
    ///                         "schemaLocation": "/items",
    ///                         "details": [
    ///                             {
    ///                                 "valid": false,
    ///                                 "evaluationPath": "/items/type",
    ///                                 "instanceLocation": "/1",
    ///                                 "schemaLocation": "/items/type",
    ///                                 "errors": {"type": "\"oops\" is not of type \"integer\""}
    ///                             }
    ///                         ]
    ///                     }
    ///                 ]
    ///             },
    ///             {
    ///                 "valid": true,
    ///                 "evaluationPath": "/prefixItems",
    ///                 "instanceLocation": "",
    ///                 "schemaLocation": "/prefixItems",
    ///                 "annotations": 0,
    ///                 "details": [
    ///                     {
    ///                         "valid": true,
    ///                         "evaluationPath": "/prefixItems/0",
    ///                         "instanceLocation": "/0",
    ///                         "schemaLocation": "/prefixItems/0",
    ///                         "details": [
    ///                             {
    ///                                 "valid": true,
    ///                                 "evaluationPath": "/prefixItems/0/type",
    ///                                 "instanceLocation": "/0",
    ///                                 "schemaLocation": "/prefixItems/0/type"
    ///                             }
    ///                         ]
    ///                     }
    ///                 ]
    ///             }
    ///         ]
    ///     })
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn hierarchical(&self) -> HierarchicalOutput<'_> {
        HierarchicalOutput {
            arena: &self.arena,
            root: self.root,
        }
    }
    /// Returns an iterator over all annotations produced during evaluation.
    ///
    /// Annotations are metadata emitted by keywords during successful validation.
    /// They can be used to collect information about which parts of a schema
    /// matched the instance. A failing schema produces no annotations, including those of
    /// its subschemas, so an invalid instance yields none.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "object",
    ///     "properties": {"name": {"type": "string"}, "age": {"type": "number", "minimum": 0}},
    ///     "required": ["name"]
    /// });
    /// let validator = schema_validation::validator_for(&schema)?;
    /// let evaluation = validator.evaluate(&json!({"name": "Alice", "age": 30}));
    ///
    /// let entries: Vec<_> = evaluation.iter_annotations().collect();
    /// assert_eq!(entries.len(), 1);
    /// assert_eq!(entries[0].schema_location, "/properties");
    /// assert_eq!(entries[0].instance_location.as_str(), "");
    ///
    /// let mut annotation_names: Vec<_> = entries[0]
    ///     .annotations
    ///     .value()
    ///     .as_array()
    ///     .expect("annotation should be an array")
    ///     .iter()
    ///     .map(|value| value.as_str().expect("annotation items should be strings"))
    ///     .collect();
    /// annotation_names.sort_unstable();
    /// assert_eq!(annotation_names, vec!["age", "name"]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn iter_annotations(&self) -> AnnotationIter<'_> {
        AnnotationIter::new(&self.arena, self.root)
    }
    /// Returns an iterator over all errors produced during evaluation.
    ///
    /// Each error entry contains information about a validation failure,
    /// including its location in both the schema and instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "object",
    ///     "required": ["name"],
    ///     "properties": {
    ///         "age": {"type": "number"}
    ///     }
    /// });
    /// let validator = schema_validation::validator_for(&schema)?;
    /// let evaluation = validator.evaluate(&json!({"name": "Bob", "age": "oops"}));
    ///
    /// let errors: Vec<_> = evaluation.iter_errors().collect();
    /// assert_eq!(errors.len(), 1);
    /// assert_eq!(errors[0].schema_location, "/properties/age/type");
    /// assert_eq!(errors[0].instance_location.as_str(), "/age");
    /// assert_eq!(errors[0].error.to_string(), "\"oops\" is not of type \"number\"");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn iter_errors(&self) -> ErrorIter<'_> {
        ErrorIter::new(&self.arena, self.root)
    }
}

/// Flag output format containing only a validity indicator.
///
/// This is the simplest output format defined in the JSON Schema specification.
/// It contains only a single boolean field indicating whether validation succeeded.
///
/// # JSON Structure
///
/// ```json
/// {
///   "valid": true
/// }
/// ```
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({"type": "string"});
/// let validator = schema_validation::validator_for(&schema)?;
/// let evaluation = validator.evaluate(&json!("hello"));
///
/// let flag = evaluation.flag();
/// assert_eq!(flag.valid, true);
///
/// let output = json_codec::to_value(flag)?;
/// assert_eq!(output, json!({"valid": true}));
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, Serialize)]
pub struct FlagOutput {
    /// Whether the instance is valid according to the schema.
    pub valid: bool,
}

/// List output format providing a flat list of evaluation units.
///
/// This format represents the evaluation result as a flat sequence where each
/// entry corresponds to a validation step. Each unit includes its evaluation path,
/// schema location, instance location, validity, and any annotations or errors.
///
/// See [`Evaluation::list`] for an example JSON payload produced by this type.
#[derive(Debug)]
pub struct ListOutput<'a> {
    arena: &'a EvaluationArena,
    root: u32,
}

/// Hierarchical output format providing a tree structure of evaluation results.
///
/// This format represents the evaluation as a nested tree that mirrors the logical
/// structure of the schema. Each node contains validation results and child nodes
/// representing nested sub-schema evaluations.
///
/// See [`Evaluation::hierarchical`] for an example JSON payload produced by this type.
#[derive(Debug)]
pub struct HierarchicalOutput<'a> {
    arena: &'a EvaluationArena,
    root: u32,
}

impl Serialize for ListOutput<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        serialize_list(self.arena, self.root, serializer)
    }
}

impl Serialize for HierarchicalOutput<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        serialize_hierarchical(self.arena, self.root, serializer)
    }
}

fn serialize_list<S>(arena: &EvaluationArena, root: u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: data_serialization::Serializer,
{
    let mut state = serializer.serialize_struct("ListOutput", 2)?;
    state.serialize_field("valid", &arena.node(root).valid)?;
    let mut entries = Vec::new();
    collect_list_entries(arena, root, &mut entries);
    state.serialize_field("details", &entries)?;
    state.end()
}

fn serialize_hierarchical<S>(
    arena: &EvaluationArena,
    root: u32,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: data_serialization::Serializer,
{
    serialize_unit(arena, root, serializer, true)
}

fn collect_list_entries<'a>(arena: &'a EvaluationArena, index: u32, out: &mut Vec<ListEntry<'a>>) {
    // Note: The spec says "Output units which do not contain errors or annotations SHOULD be
    // excluded" but the official test suite includes all nodes. We include all nodes to match
    // the reference implementation and test suite expectations.
    out.push(ListEntry::new(arena, index));
    for child in arena.child_indices(index) {
        collect_list_entries(arena, child, out);
    }
}

fn serialize_unit<S>(
    arena: &EvaluationArena,
    index: u32,
    serializer: S,
    include_children: bool,
) -> Result<S::Ok, S::Error>
where
    S: data_serialization::Serializer,
{
    let node = arena.node(index);
    let mut state = serializer.serialize_struct("OutputUnit", 7)?;
    state.serialize_field("valid", &node.valid)?;
    state.serialize_field("evaluationPath", node.keyword_location.as_str())?;
    state.serialize_field("schemaLocation", node.schema_location.as_ref())?;
    state.serialize_field("instanceLocation", node.instance_location.as_str())?;
    if let Some(annotations) = &node.annotations {
        state.serialize_field("annotations", annotations)?;
    }
    if let Some(annotations) = &node.dropped_annotations {
        state.serialize_field("droppedAnnotations", annotations)?;
    }
    if !node.errors.is_empty() {
        state.serialize_field("errors", &ErrorEntriesSerializer(&node.errors))?;
    }
    if include_children && node.first_child != NO_NODE {
        state.serialize_field("details", &DetailsSerializer { arena, index })?;
    }
    state.end()
}

/// Wraps an absorbed keyword's failure as a child node at that keyword's own schema location,
/// so structured output keeps the correct `schemaLocation`.
pub(crate) fn absorbed_error_node(
    location: &LazyLocation,
    tracker: Option<&RefTracker>,
    keyword_location: &Location,
    absolute_location: Option<&Arc<Uri<String>>>,
    error: ErrorDescription,
    ctx: &mut ValidationContext,
) -> EvaluationNode {
    EvaluationNode::invalid(
        crate::validation::paths::evaluation_path(tracker, keyword_location, ctx),
        absolute_location.cloned(),
        format_keyword_location(keyword_location, absolute_location),
        location.into(),
        None,
        vec![error],
        ChildList::default(),
    )
}

/// `schemaLocation` of a keyword: its absolute URI when it has one, otherwise its JSON Pointer.
///
/// The URI's fragment counts from the root of the keyword's own resource, which a subschema
/// declaring its own `$id` places below the document root.
pub(crate) fn format_keyword_location(
    location: &Location,
    absolute: Option<&Arc<Uri<String>>>,
) -> Arc<str> {
    absolute.map_or_else(|| location.as_arc(), |uri| Arc::from(uri.as_str()))
}

struct ListEntry<'a> {
    arena: &'a EvaluationArena,
    index: u32,
}

impl<'a> ListEntry<'a> {
    fn new(arena: &'a EvaluationArena, index: u32) -> Self {
        ListEntry { arena, index }
    }
}

impl Serialize for ListEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        serialize_unit(self.arena, self.index, serializer, false)
    }
}

struct DetailsSerializer<'a> {
    arena: &'a EvaluationArena,
    index: u32,
}

impl Serialize for DetailsSerializer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        for child in self.arena.child_indices(self.index) {
            seq.serialize_element(&SeqEntry {
                arena: self.arena,
                index: child,
            })?;
        }
        seq.end()
    }
}

struct SeqEntry<'a> {
    arena: &'a EvaluationArena,
    index: u32,
}

impl Serialize for SeqEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        serialize_unit(self.arena, self.index, serializer, true)
    }
}

/// Entry describing annotations emitted by a keyword during evaluation.
///
/// Annotations are metadata produced by keywords during successful validation.
/// They provide additional information about which schema keywords matched
/// and what values they produced.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "name": {"type": "string"},
///         "age": {"type": "number"}
///     }
/// });
/// let validator = schema_validation::validator_for(&schema)?;
/// let instance = json!({"name": "Alice", "age": 30});
/// let evaluation = validator.evaluate(&instance);
/// let entry = evaluation.iter_annotations().next().unwrap();
/// assert_eq!(entry.schema_location, "/properties");
/// assert_eq!(entry.instance_location.as_str(), "");
///
/// let mut annotation_names: Vec<_> = entry
///     .annotations
///     .value()
///     .as_array()
///     .expect("annotation should be an array")
///     .iter()
///     .map(|value| value.as_str().expect("annotation items should be strings"))
///     .collect();
/// annotation_names.sort_unstable();
/// assert_eq!(annotation_names, vec!["age", "name"]);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct AnnotationEntry<'a> {
    /// The JSON Pointer to the schema keyword that produced the annotation.
    pub schema_location: &'a str,
    /// The absolute URI of the keyword location, if available.
    pub absolute_keyword_location: Option<&'a Uri<String>>,
    /// The JSON Pointer to the instance location being validated.
    pub instance_location: &'a Location,
    /// The annotations produced by the keyword.
    pub annotations: &'a Annotations,
}

/// Entry describing errors emitted by a keyword during evaluation.
///
/// Error entries contain information about validation failures, including
/// the locations in both the schema and instance where the error occurred.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "type": "object",
///     "required": ["name"],
///     "properties": {
///         "age": {"type": "number"}
///     }
/// });
/// let validator = schema_validation::validator_for(&schema)?;
/// let instance = json!({"name": "test", "age": "oops"});
/// let evaluation = validator.evaluate(&instance);
/// let entry = evaluation.iter_errors().next().unwrap();
/// assert_eq!(entry.schema_location, "/properties/age/type");
/// assert_eq!(entry.instance_location.as_str(), "/age");
/// assert_eq!(entry.error.to_string(), "\"oops\" is not of type \"number\"");
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ErrorEntry<'a> {
    /// The JSON Pointer to the schema keyword that produced the error.
    pub schema_location: &'a str,
    /// The absolute URI of the keyword location, if available.
    pub absolute_keyword_location: Option<&'a Uri<String>>,
    /// The JSON Pointer to the instance location that failed validation.
    pub instance_location: &'a Location,
    /// The error description.
    pub error: &'a ErrorDescription,
}

impl fmt::Display for ErrorEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.instance_location.is_empty() {
            self.error.fmt(f)
        } else {
            write!(f, "{}: {}", self.instance_location, self.error)
        }
    }
}

struct NodeIter<'a> {
    arena: &'a EvaluationArena,
    stack: Vec<u32>,
    /// Whether to walk into the subtrees of failing nodes.
    enter_invalid: bool,
}

impl<'a> NodeIter<'a> {
    fn new(arena: &'a EvaluationArena, root: u32) -> Self {
        NodeIter {
            arena,
            stack: vec![root],
            enter_invalid: true,
        }
    }

    /// Only nodes whose every ancestor, and the node itself, is valid.
    fn valid(arena: &'a EvaluationArena, root: u32) -> Self {
        NodeIter {
            arena,
            stack: vec![root],
            enter_invalid: false,
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a EvaluationNode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let index = self.stack.pop()?;
            let node = self.arena.node(index);
            if !node.valid && !self.enter_invalid {
                continue;
            }
            let start = self.stack.len();
            self.stack.extend(self.arena.child_indices(index));
            self.stack[start..].reverse();
            return Some(node);
        }
    }
}

/// Iterator over annotations produced during evaluation.
///
/// This iterator traverses the evaluation tree and yields [`AnnotationEntry`]
/// for each node that produced annotations during validation.
///
/// A failing schema produces no annotations, including those of its subschemas.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "name": {"type": "string"},
///         "age": {"type": "number"}
///     }
/// });
/// let validator = schema_validation::validator_for(&schema)?;
/// let evaluation = validator.evaluate(&json!({"name": "Alice", "age": 30}));
///
/// let annotations: Vec<_> = evaluation.iter_annotations().collect();
/// assert_eq!(annotations.len(), 1);
/// assert_eq!(annotations[0].instance_location.as_str(), "");
///
/// let mut annotation_names: Vec<_> = annotations[0]
///     .annotations
///     .value()
///     .as_array()
///     .expect("annotation should be an array")
///     .iter()
///     .map(|value| value.as_str().expect("annotation items should be strings"))
///     .collect();
/// annotation_names.sort_unstable();
/// assert_eq!(annotation_names, vec!["age", "name"]);
/// # Ok(())
/// # }
/// ```
pub struct AnnotationIter<'a> {
    nodes: NodeIter<'a>,
}

impl<'a> AnnotationIter<'a> {
    fn new(arena: &'a EvaluationArena, root: u32) -> Self {
        AnnotationIter {
            nodes: NodeIter::valid(arena, root),
        }
    }
}

impl<'a> Iterator for AnnotationIter<'a> {
    type Item = AnnotationEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for node in self.nodes.by_ref() {
            if let Some(annotations) = node.annotations.as_ref() {
                return Some(AnnotationEntry {
                    schema_location: &node.schema_location,
                    absolute_keyword_location: node.absolute_keyword_location.as_deref(),
                    instance_location: &node.instance_location,
                    annotations,
                });
            }
        }
        None
    }
}

/// Iterator over errors produced during evaluation.
///
/// This iterator traverses the evaluation tree and yields [`ErrorEntry`]
/// for each error encountered during validation.
///
/// Nodes can have multiple errors, and this iterator will yield all of them
/// in depth-first order.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "type": "object",
///     "required": ["name"],
///     "properties": {
///         "name": {"type": "string"},
///         "age": {"type": "number", "minimum": 0}
///     }
/// });
/// let validator = schema_validation::validator_for(&schema)?;
/// let evaluation = validator.evaluate(&json!({"age": -5}));
///
/// let errors: Vec<_> = evaluation.iter_errors().collect();
/// assert_eq!(errors.len(), 2);
/// assert_eq!(errors[0].schema_location, "/required");
/// assert_eq!(errors[0].instance_location.as_str(), "");
/// assert_eq!(errors[1].schema_location, "/properties/age/minimum");
/// assert_eq!(errors[1].instance_location.as_str(), "/age");
/// # Ok(())
/// # }
/// ```
pub struct ErrorIter<'a> {
    nodes: NodeIter<'a>,
    current: Option<(&'a EvaluationNode, usize)>,
}

impl<'a> ErrorIter<'a> {
    fn new(arena: &'a EvaluationArena, root: u32) -> Self {
        ErrorIter {
            nodes: NodeIter::new(arena, root),
            current: None,
        }
    }
}

impl<'a> Iterator for ErrorIter<'a> {
    type Item = ErrorEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((node, idx)) = self.current {
                if idx < node.errors.len() {
                    let entry = ErrorEntry {
                        schema_location: &node.schema_location,
                        absolute_keyword_location: node.absolute_keyword_location.as_deref(),
                        instance_location: &node.instance_location,
                        error: &node.errors[idx],
                    };
                    self.current = Some((node, idx + 1));
                    return Some(entry);
                }
                self.current = None;
            }

            {
                let node = self.nodes.next()?;
                if node.errors.is_empty() {
                    continue;
                }
                self.current = Some((node, 0));
            }
        }
    }
}

struct ErrorEntriesSerializer<'a>(&'a [ErrorDescription]);

impl<'a> Serialize for ErrorEntriesSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: data_serialization::Serializer,
    {
        let mut grouped: Vec<(&'a str, Vec<&'a str>)> = Vec::new();
        let mut indexes: AHashMap<&'a str, usize> = AHashMap::new();

        for error in self.0 {
            let keyword = error.keyword();
            let msg = error.message();
            if let Some(&idx) = indexes.get(keyword) {
                grouped[idx].1.push(msg);
            } else {
                indexes.insert(keyword, grouped.len());
                grouped.push((keyword, vec![msg]));
            }
        }

        let mut map = serializer.serialize_map(Some(grouped.len()))?;
        for (keyword, messages) in grouped {
            if messages.len() == 1 {
                map.serialize_entry(keyword, messages[0])?;
            } else {
                map.serialize_entry(keyword, &messages)?;
            }
        }
        map.end()
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/evaluation/tests_1230.rs"
));

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/evaluation/public_api_2148.rs"
));
