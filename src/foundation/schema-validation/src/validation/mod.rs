//! Offline JSON Schema validation with explicit custom retrieval.
pub(crate) mod bundler;
pub mod canonical;
pub(crate) mod compiler;
mod content_encoding;
mod content_media_type;
pub(crate) mod dereferencer;
pub mod error;
mod evaluation;
pub(crate) use crate::value::{
    cmp, numeric, unique, Array, Json, JsonValueBackend, LazyInstance, Node, NodeIdentity, Object,
};
/// Validating instances in a custom in-memory JSON representation.
///
/// Implement [`Json`], [`Node`], [`Object`], [`Array`], and [`JsonNumber`](json::JsonNumber)
/// for your representation, then build validators with [`options_for`]. Schemas are always
/// `json_codec::Value`; only instances use the custom representation. [`SerdeJson`] is the
/// built-in representation behind [`validator_for`] and the crate-level convenience functions.
///
/// Enable the `jsonb` feature for `json::Jsonb`, which reads a Postgres `jsonb` value in
/// place. It takes the container bytes without the varlena header, so detoast first:
///
/// ```rust,ignore
/// let detoasted = unsafe { pgrx::pg_sys::pg_detoast_datum_packed(datum.cast_mut_ptr()) };
/// let bytes = unsafe { pgrx::varlena_to_byte_slice(detoasted) };
/// validator.is_valid(Jsonb::root(bytes))
/// ```
///
/// The slice lives until the memory context resets. The bytes are in the server's native byte
/// order, and keys and strings are read as UTF-8. An error's instance is built on first access;
/// past 128 levels of nesting it holds `null` there and
/// `json::jsonb::take_pending_error` returns the error.
///
/// The accessors are infallible, so the representation must be total over JSON: reject nodes
/// with no JSON meaning (tags, foreign objects) before validation, or track them on a side
/// channel of the representation.
///
/// # Example
///
/// ```rust
/// use std::borrow::Cow;
///
/// use schema_validation::{
///     json::{Array, Json, JsonNumber, Node, NodeIdentity, Object},
///     JsonType,
/// };
/// use json_codec::Value;
///
/// #[derive(Default)]
/// enum ToyValue {
///     #[default]
///     Null,
///     Boolean(bool),
///     Number(f64),
///     String(String),
///     Array(Vec<ToyValue>),
///     Object(Vec<(String, ToyValue)>),
/// }
///
/// struct ToyJson;
///
/// impl Json for ToyJson {
///     // A cheap-to-`Clone` handle; `&ToyValue` plays the role `&json_codec::Value` does
///     // for the built-in representation.
///     type Node<'a> = &'a ToyValue;
///     // Property names are prepared once at schema compile time.
///     type PreparedKey = String;
///     // Scratch storage for nodes made from property names (`propertyNames`).
///     type StringBuffer = ToyValue;
///
///     fn prepare_key(key: &str) -> String {
///         key.to_owned()
///     }
///
///     fn with_string_node<T>(
///         buffer: &mut ToyValue,
///         string: &str,
///         f: impl FnOnce(&ToyValue) -> T,
///     ) -> T {
///         *buffer = ToyValue::String(string.to_owned());
///         f(buffer)
///     }
/// }
///
/// struct ToyNumber(f64);
///
/// impl JsonNumber for ToyNumber {
///     fn as_u64(&self) -> Option<u64> {
///         (self.0.fract() == 0.0 && self.0 >= 0.0).then_some(self.0 as u64)
///     }
///     fn as_i64(&self) -> Option<i64> {
///         (self.0.fract() == 0.0).then_some(self.0 as i64)
///     }
///     fn as_f64(&self) -> Option<f64> {
///         Some(self.0)
///     }
///     fn as_str(&self) -> Cow<'_, str> {
///         Cow::Owned(self.0.to_string())
///     }
///     fn to_number(&self) -> Cow<'_, json_codec::Number> {
///         Cow::Owned(json_codec::Number::from_f64(self.0).expect("finite"))
///     }
/// }
///
/// impl<'a> Node<'a, ToyJson> for &'a ToyValue {
///     type Object = &'a [(String, ToyValue)];
///     type Array = &'a [ToyValue];
///     type Number = ToyNumber;
///
///     fn as_object(&self) -> Option<&'a [(String, ToyValue)]> {
///         match self {
///             ToyValue::Object(members) => Some(members),
///             _ => None,
///         }
///     }
///     fn as_array(&self) -> Option<&'a [ToyValue]> {
///         match self {
///             ToyValue::Array(items) => Some(items),
///             _ => None,
///         }
///     }
///     fn as_string(&self) -> Option<Cow<'a, str>> {
///         match self {
///             ToyValue::String(string) => Some(Cow::Borrowed(string)),
///             _ => None,
///         }
///     }
///     fn as_number(&self) -> Option<ToyNumber> {
///         match self {
///             ToyValue::Number(number) => Some(ToyNumber(*number)),
///             _ => None,
///         }
///     }
///     fn as_boolean(&self) -> Option<bool> {
///         match self {
///             ToyValue::Boolean(boolean) => Some(*boolean),
///             _ => None,
///         }
///     }
///     fn is_null(&self) -> bool {
///         matches!(self, ToyValue::Null)
///     }
///     fn json_type(&self) -> JsonType {
///         match self {
///             ToyValue::Null => JsonType::Null,
///             ToyValue::Boolean(_) => JsonType::Boolean,
///             ToyValue::Number(_) => JsonType::Number,
///             ToyValue::String(_) => JsonType::String,
///             ToyValue::Array(_) => JsonType::Array,
///             ToyValue::Object(_) => JsonType::Object,
///         }
///     }
///     // Cold paths only: error messages, `const`/`enum` comparisons, `uniqueItems`.
///     fn to_value(&self) -> Cow<'a, Value> {
///         Cow::Owned(match self {
///             ToyValue::Null => Value::Null,
///             ToyValue::Boolean(boolean) => Value::Bool(*boolean),
///             ToyValue::Number(number) => json_codec::Number::from_f64(*number)
///                 .map(Value::Number)
///                 .expect("finite"),
///             ToyValue::String(string) => Value::String(string.clone()),
///             ToyValue::Array(items) => Value::Array(
///                 items.iter().map(|item| item.to_value().into_owned()).collect(),
///             ),
///             ToyValue::Object(members) => Value::Object(
///                 members
///                     .iter()
///                     .map(|(name, value)| (name.clone(), value.to_value().into_owned()))
///                     .collect(),
///             ),
///         })
///     }
///     // Stable per live node; see the trait docs for the exact contract.
///     fn identity(&self) -> Option<NodeIdentity> {
///         Some(NodeIdentity::new(std::ptr::from_ref::<ToyValue>(*self) as usize))
///     }
/// }
///
/// impl<'a> Object<'a, ToyJson> for &'a [(String, ToyValue)] {
///     type Node = &'a ToyValue;
///     type MemberName = &'a str;
///     type MembersIter = ToyMembersIter<'a>;
///
///     fn len(&self) -> usize {
///         <[(String, ToyValue)]>::len(self)
///     }
///     fn get(&self, key: &String) -> Option<&'a ToyValue> {
///         self.iter().find(|(name, _)| name == key).map(|(_, value)| value)
///     }
///     fn members(&self) -> ToyMembersIter<'a> {
///         ToyMembersIter(self.iter())
///     }
/// }
///
/// struct ToyMembersIter<'a>(std::slice::Iter<'a, (String, ToyValue)>);
///
/// impl<'a> Iterator for ToyMembersIter<'a> {
///     type Item = (&'a str, &'a ToyValue);
///     fn next(&mut self) -> Option<Self::Item> {
///         self.0.next().map(|(name, value)| (name.as_str(), value))
///     }
/// }
///
/// impl<'a> Array<'a, ToyJson> for &'a [ToyValue] {
///     type Node = &'a ToyValue;
///     type ElementsIter = std::slice::Iter<'a, ToyValue>;
///
///     fn len(&self) -> usize {
///         <[ToyValue]>::len(self)
///     }
///     fn elements(&self) -> std::slice::Iter<'a, ToyValue> {
///         self.iter()
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let schema = json_codec::json!({
///     "type": "object",
///     "properties": {"name": {"type": "string", "minLength": 2}},
///     "required": ["name"]
/// });
/// let validator = schema_validation::options_for::<ToyJson>().build(&schema)?;
/// # let _ = format!("{:?}", schema_validation::options_for::<ToyJson>());
///
/// let valid = ToyValue::Object(vec![("name".into(), ToyValue::String("bob".into()))]);
/// let invalid = ToyValue::Object(vec![]);
/// assert!(validator.is_valid(&valid));
/// let error = validator.validate(&invalid).expect_err("missing required");
/// assert_eq!(error.to_string(), "\"name\" is a required property");
/// # Ok(())
/// # }
/// ```
///
/// # Verifying an implementation
///
/// Enable the `conformance` feature, encode the `conformance::document()` JSON document in your
/// representation, and run `conformance::assert_conformance` on it; it checks the accessor
/// contract the validator relies on, including the subtle parts (code-point string lengths,
/// mathematical number equality, node identity stability).
///
/// # Custom keywords
///
/// [`Keyword`] is generic over the representation and operates on `F::Node` directly; register
/// implementations with [`ValidationOptions::with_keyword`](schema_validation::ValidationOptions::with_keyword).
pub mod json {
    #[cfg(feature = "conformance")]
    pub use crate::value::conformance;
    pub use crate::value::{
        cmp, unique, Array, Json, JsonNumber, JsonValueBackend, Node, NodeIdentity, Object,
    };

    #[cfg(feature = "jsonb")]
    pub use crate::value::{jsonb, Jsonb, JsonbNode};
}
mod keywords;
mod node;
mod options;
pub mod output;
pub mod paths;
pub(crate) mod properties;
pub(crate) mod regex;
mod retriever;
pub mod types {
    pub use crate::value::types::{JsonType, JsonTypeSet, JsonTypeSetIterator};
}
mod validator;

pub use crate::reference::{
    uri, Draft, Error as ReferencingError, Registry, RegistryBuilder, Resource, Retrieve, Uri,
};
pub use canonical::CanonicalizationError;
pub use error::{
    ErrorIterator, MaskedValidationError, ValidationError, ValidationErrorParts, ValidationErrors,
};
pub use evaluation::{
    AnnotationEntry, ErrorEntry, Evaluation, FlagOutput, HierarchicalOutput, ListOutput,
};
#[doc(inline)]
pub use keywords::custom::Keyword;
pub use options::{EmailOptions, FancyRegex, PatternOptions, Regex, ValidationOptions};

pub use types::{JsonType, JsonTypeSet, JsonTypeSetIterator};
pub use validator::{ValidationContext, Validator, ValidatorMap};

#[cfg(feature = "resolve-async")]
pub use crate::reference::AsyncRetrieve;

use json_codec::Value;

/// Validate `instance` against `schema` and get a `true` if the instance is valid and `false`
/// otherwise. Draft is detected automatically.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"maxLength": 5});
/// let instance = json!("foo");
/// assert!(schema_validation::is_valid(&schema, &instance));
/// ```
///
/// # Panics
///
/// This function panics if an invalid schema is passed.
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_validator_for` for async contexts, or run this in a separate blocking thread
/// using the supplied asynchronous retriever.
#[must_use]
#[inline]
pub fn is_valid(schema: &Value, instance: &Value) -> bool {
    validator_for(schema)
        .expect("Invalid schema")
        .is_valid(instance)
}

/// Validate `instance` against `schema` and return the first error if any. Draft is detected automatically.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"maxLength": 5});
/// let instance = json!("foo");
/// assert!(schema_validation::validate(&schema, &instance).is_ok());
/// ```
///
/// # Errors
///
/// Returns the first [`ValidationError`] encountered when `instance` violates `schema`.
///
/// # Panics
///
/// This function panics if an invalid schema is passed.
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_validator_for` for async contexts, or run this in a separate blocking thread
/// using the supplied asynchronous retriever.
#[inline]
pub fn validate<'i>(schema: &Value, instance: &'i Value) -> Result<(), ValidationError<'i>> {
    validator_for(schema)
        .expect("Invalid schema")
        .validate(instance)
}

/// Evaluate `instance` against `schema` and return structured validation output. Draft is detected automatically.
///
/// Returns an [`Evaluation`] containing detailed validation results in JSON Schema Output v1 format,
/// including annotations and errors across the entire validation tree.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "string", "minLength": 3});
/// let instance = json!("foo");
/// let evaluation = schema_validation::evaluate(&schema, &instance);
/// assert!(evaluation.flag().valid);
/// ```
///
/// # Panics
///
/// This function panics if an invalid schema is passed.
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_validator_for` for async contexts, or run this in a separate blocking thread
/// using the supplied asynchronous retriever.
#[must_use]
#[inline]
pub fn evaluate(schema: &Value, instance: &Value) -> Evaluation {
    validator_for(schema)
        .expect("Invalid schema")
        .evaluate(instance)
}

/// Create a validator for the input schema with automatic draft detection and default options.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({"minimum": 5});
/// let instance = json!(42);
///
/// let validator = schema_validation::validator_for(&schema)?;
/// assert!(validator.is_valid(&instance));
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the schema is invalid or external references cannot be resolved.
///
/// # Panics
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_validator_for` for async contexts, or run this in a separate blocking thread
/// using the supplied asynchronous retriever.
pub fn validator_for(schema: &Value) -> Result<Validator, ValidationError<'static>> {
    Validator::new(schema)
}

/// Create a [`ValidatorMap`] from the input schema using automatic draft detection and
/// default options.
///
/// Every reachable subschema is compiled eagerly. The root schema is always present
/// under the key `"#"`. Subschemas that fail to compile (e.g. unresolvable `$ref`) are
/// silently omitted.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "$defs": {
///         "User": {"type": "object", "required": ["name"]}
///     }
/// });
///
/// let map = schema_validation::validator_map_for(&schema)?;
///
/// let user_validator = map.get("#/$defs/User").unwrap();
/// assert!(user_validator.is_valid(&json!({"name": "Alice"})));
/// assert!(!user_validator.is_valid(&json!({})));
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the schema is invalid or external references cannot be resolved.
///
/// # Panics
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_validator_map_for` for async contexts, or run this in a separate blocking thread
/// using the supplied asynchronous retriever.
pub fn validator_map_for(schema: &Value) -> Result<ValidatorMap, ValidationError<'static>> {
    options().build_map(schema)
}

/// Embed all external `$ref` targets into a draft-appropriate container,
/// producing a Compound Schema Document that validates identically to the original.
/// Draft 4/6/7 use `definitions`; Draft 2019-09/2020-12 use `$defs`.
/// `$ref` values are preserved unchanged.
/// For mixed-draft bundles, embedded resources may include both `id` and `$id`
/// to maximize interoperability with downstream validators that differ in draft
/// handling.
///
/// **Limitation:** `$dynamicRef` is not followed during bundling.
///
/// For custom resources or retrievers, use [`options()`] and call `.bundle()`.
///
/// # Errors
///
/// Returns an error if draft detection fails, registry construction fails,
/// subresource scope resolution fails, or any `$ref` cannot be resolved.
///
/// # Panics
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_options().bundle()` for async contexts, or run this in a separate blocking thread
/// using the supplied asynchronous retriever.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "string"});
/// let bundled = schema_validation::bundle(&schema).expect("bundling failed");
/// assert_eq!(bundled, schema); // no external refs, returned unchanged
/// ```
pub fn bundle(schema: &Value) -> Result<Value, ReferencingError> {
    options().bundle(schema)
}

/// Dereference a JSON Schema by recursively replacing all `$ref` values
/// with the schemas they point to.
///
/// Circular references are left in place as `$ref` strings.
///
/// For custom retriever, registry, draft, or base URI, use [`options()`] and call `.dereference()`.
///
/// # Errors
///
/// Returns an error if any `$ref` cannot be resolved (e.g. points to an
/// external URI not present in the schema and no retriever is available).
///
/// # Example
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({
///     "$defs": {"tag": {"type": "string"}},
///     "properties": {"name": {"$ref": "#/$defs/tag"}}
/// });
/// let result = schema_validation::dereference(&schema).expect("dereference failed");
/// assert_eq!(result["properties"]["name"]["type"], "string");
/// ```
pub fn dereference(schema: &Value) -> Result<Value, ReferencingError> {
    options().dereference(schema)
}

/// Bundle a JSON Schema into a Compound Schema Document,
/// using async retrieval for external references.
///
/// Async counterpart to [`bundle`]. For custom resources or retrievers,
/// use [`async_options()`] and call `.bundle()`.
///
/// # Errors
///
/// Returns an error if draft detection fails, registry construction fails,
/// subresource scope resolution fails, or any `$ref` cannot be resolved.
#[cfg(feature = "resolve-async")]
pub async fn async_bundle(schema: &Value) -> Result<Value, ReferencingError> {
    async_options().bundle(schema).await
}

/// Dereference a JSON Schema asynchronously.
///
/// Async counterpart to [`dereference`]. For custom resources or retrievers,
/// use [`async_options()`] and call `.dereference()`.
///
/// # Errors
///
/// Returns an error if any `$ref` cannot be resolved.
#[cfg(feature = "resolve-async")]
pub async fn async_dereference(schema: &Value) -> Result<Value, ReferencingError> {
    async_options().dereference(schema).await
}

/// Create a validator for the input schema with automatic draft detection and default options,
/// using non-blocking retrieval for external references.
///
/// This is the async counterpart to [`validator_for`]. Note that only the construction is
/// asynchronous - validation itself is always synchronous.
///
/// # Examples
///
/// ```rust
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "user": { "$ref": "https://example.com/user.json" }
///     }
/// });
///
/// let validator = schema_validation::async_validator_for(&schema).await?;
/// assert!(validator.is_valid(&json!({"user": {"name": "Alice"}})));
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the schema is invalid or external references cannot be resolved.
#[cfg(feature = "resolve-async")]
pub async fn async_validator_for(schema: &Value) -> Result<Validator, ValidationError<'static>> {
    Validator::async_new(schema).await
}

/// Create a [`ValidatorMap`] from the input schema using async retrieval for external references.
///
/// Async counterpart to [`validator_map_for`]. Note that only construction is asynchronous —
/// validation itself is always synchronous.
///
/// # Examples
///
/// ```rust
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({"$defs": {"User": {"type": "object", "required": ["name"]}}});
/// let map = schema_validation::async_validator_map_for(&schema).await?;
/// assert!(map["#/$defs/User"].is_valid(&json!({"name": "Bob"})));
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the schema is invalid or external references cannot be resolved.
#[cfg(feature = "resolve-async")]
pub async fn async_validator_map_for(
    schema: &Value,
) -> Result<ValidatorMap, ValidationError<'static>> {
    async_options().build_map(schema).await
}

/// Reduce a JSON Schema to a normal form that accepts the same values.
///
/// Experimental: keyword coverage is incomplete and the API may change in minor releases.
///
/// Use [`canonical::options`](fn@canonical::options) to configure canonicalization.
///
/// A schema the canonical form cannot model exactly still succeeds, as a `Raw` result holding the
/// original document; see [Unsupported schemas](canonical#unsupported-schemas).
///
/// # Errors
///
/// Returns [`CanonicalizationError`] when the input is not a valid JSON Schema document or a reference cannot be resolved.
pub fn canonicalize(value: &Value) -> Result<canonical::CanonicalSchema, CanonicalizationError> {
    canonical::options().canonicalize(value)
}

/// Create a builder for configuring JSON Schema validation options.
///
/// This function returns a [`ValidationOptions`] struct, which allows you to set various
/// options for JSON Schema validation. You can use this builder to specify
/// the draft version, set custom formats, and more.
///
/// If [`with_draft`](ValidationOptions::with_draft) is not called, the draft is
/// auto-detected from the schema's `$schema` field — the same behaviour as [`validator_for`].
///
/// **Note:** When calling [`ValidationOptions::build`], it **must not** be called from within
/// an async runtime if the schema contains external references that require network requests,
/// or it will panic. Use `async_options` for async contexts.
///
/// # Examples
///
/// Basic usage with draft specification:
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
/// use schema_validation::Draft;
///
/// let schema = json!({"type": "string"});
/// let validator = schema_validation::options()
///     .with_draft(Draft::Draft7)
///     .build(&schema)?;
///
/// assert!(validator.is_valid(&json!("Hello")));
/// # Ok(())
/// # }
/// ```
///
/// Advanced configuration:
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({"type": "string", "format": "custom"});
/// let validator = schema_validation::options()
///     .with_format("custom", |value| value.len() == 3)
///     .should_validate_formats(true)
///     .build(&schema)?;
///
/// assert!(validator.is_valid(&json!("abc")));
/// assert!(!validator.is_valid(&json!("abcd")));
/// # Ok(())
/// # }
/// ```
///
/// See [`ValidationOptions`] for all available configuration options.
#[must_use]
pub fn options<'i>() -> ValidationOptions<'i> {
    Validator::options()
}

/// Create a builder whose validators accept instances in the JSON representation `F`.
///
/// Same configuration surface as [`options()`]; `build` yields a `Validator<F>`.
#[must_use]
pub fn options_for<'i, F: Json>() -> ValidationOptions<'i, std::sync::Arc<dyn Retrieve>, F> {
    ValidationOptions::default()
}

/// Create a builder whose validators accept instances in the JSON representation `F`, with
/// async retrieval of external references.
///
/// Same configuration surface as [`async_options()`]; `build` yields a `Validator<F>`.
#[cfg(feature = "resolve-async")]
#[must_use]
pub fn async_options_for<'i, F: Json>(
) -> ValidationOptions<'i, std::sync::Arc<dyn AsyncRetrieve>, F> {
    ValidationOptions::default()
}

/// Create a builder for configuring JSON Schema validation options.
///
/// This function returns a [`ValidationOptions`] struct which allows you to set various options for JSON Schema validation.
/// External references will be retrieved using non-blocking I/O.
///
/// # Examples
///
/// Basic usage with external references:
///
/// ```rust
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({
///     "$ref": "https://example.com/user.json"
/// });
///
/// let validator = schema_validation::async_options()
///     .build(&schema)
///     .await?;
///
/// assert!(validator.is_valid(&json!({"name": "Alice"})));
/// # Ok(())
/// # }
/// ```
///
/// Advanced configuration:
///
/// ```rust
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::{Value, json};
/// use schema_validation::{Draft, AsyncRetrieve, Uri};
///
/// // Custom async retriever
/// struct MyRetriever;
///
/// #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
/// #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
/// impl AsyncRetrieve for MyRetriever {
///     async fn retrieve(&self, uri: &Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
///         // Custom retrieval logic
///         Ok(json!({}))
///     }
/// }
///
/// let schema = json!({
///     "$ref": "https://example.com/user.json"
/// });
/// let validator = schema_validation::async_options()
///     .with_draft(Draft::Draft202012)
///     .with_retriever(MyRetriever)
///     .build(&schema)
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// On `wasm32` targets, annotate your implementation with `async_trait::async_trait(?Send)` to drop the `Send + Sync` requirement.
///
/// See [`ValidationOptions`] for all available configuration options.
#[cfg(feature = "resolve-async")]
#[must_use]
pub fn async_options<'i>() -> ValidationOptions<'i, std::sync::Arc<dyn AsyncRetrieve>> {
    Validator::async_options()
}

/// Functionality for validating JSON Schema documents against their meta-schemas.
pub mod meta {
    use crate::reference::Retrieve;
    use crate::validation::{
        error::ValidationError, Draft, Json, Node, Object, Registry, Validator,
    };
    use ahash::AHashSet;
    use json_codec::Value;
    use std::{
        any::{Any, TypeId},
        borrow::Cow,
        sync::{OnceLock, RwLock},
    };

    pub use validator_handle::MetaValidator;

    /// Create a meta-validation options builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    /// use schema_validation::{Registry, Resource};
    ///
    /// let custom_meta = Resource::from_contents(json!({
    ///     "$schema": "https://json-schema.org/draft/2020-12/schema",
    ///     "type": "object"
    /// }));
    ///
    /// let registry = Registry::new()
    ///     .add("http://example.com/meta", custom_meta)
    ///     .unwrap()
    ///     .prepare()
    ///     .unwrap();
    ///
    /// let schema = json!({
    ///     "$schema": "http://example.com/meta",
    ///     "type": "string"
    /// });
    ///
    /// assert!(schema_validation::meta::options()
    ///     .with_registry(&registry)
    ///     .is_valid(&schema));
    /// ```
    #[must_use]
    pub fn options<'a>() -> MetaSchemaOptions<'a> {
        MetaSchemaOptions::default()
    }

    /// Options for meta-schema validation.
    #[derive(Clone, Default)]
    pub struct MetaSchemaOptions<'a> {
        registry: Option<&'a Registry<'a>>,
    }

    impl<'a> MetaSchemaOptions<'a> {
        /// Use a registry for resolving custom meta-schemas.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        /// use schema_validation::{Registry, Resource};
        ///
        /// let custom_meta = Resource::from_contents(json!({
        ///     "$schema": "https://json-schema.org/draft/2020-12/schema",
        ///     "type": "object"
        /// }));
        ///
        /// let registry = Registry::new()
        ///     .add("http://example.com/meta", custom_meta)
        ///     .unwrap()
        ///     .prepare()
        ///     .unwrap();
        ///
        /// let options = schema_validation::meta::options()
        ///     .with_registry(&registry);
        /// ```
        #[must_use]
        pub fn with_registry(mut self, registry: &'a Registry<'a>) -> Self {
            self.registry = Some(registry);
            self
        }

        /// Check if a schema is valid according to its meta-schema.
        ///
        /// # Panics
        ///
        /// Panics if the meta-schema cannot be resolved.
        #[must_use]
        pub fn is_valid(&self, schema: &Value) -> bool {
            match try_meta_validator_for(schema, self.registry) {
                Ok(validator) => validator.is_valid(schema),
                Err(e) => panic!("Failed to resolve meta-schema: {e}"),
            }
        }

        /// Validate a schema according to its meta-schema.
        ///
        /// # Errors
        ///
        /// Returns [`ValidationError`] if the schema is invalid or if the meta-schema cannot be resolved.
        pub fn validate<'schema>(
            &self,
            schema: &'schema Value,
        ) -> Result<(), ValidationError<'schema>> {
            let validator = try_meta_validator_for(schema, self.registry)?;
            validator.validate(schema)
        }
    }

    mod validator_handle {
        use crate::validation::{ValidationError, Validator};
        use json_codec::Value;
        use std::{marker::PhantomData, ops::Deref};

        /// Handle to a draft-specific meta-schema [`Validator`]. Borrows cached validators on native
        /// targets and owns validators on `wasm32`.
        pub struct MetaValidator<'a>(MetaValidatorInner<'a>);

        // Native builds can hand out references to cached validators or own dynamic ones,
        // while wasm targets need owned instances because the validator type does not implement `Sync` there.
        enum MetaValidatorInner<'a> {
            #[cfg(not(target_family = "wasm"))]
            Borrowed(&'a Validator),
            Owned(Box<Validator>, PhantomData<&'a Validator>),
        }

        #[cfg_attr(target_family = "wasm", allow(clippy::elidable_lifetime_names))]
        impl<'a> MetaValidator<'a> {
            #[cfg(not(target_family = "wasm"))]
            pub(crate) fn borrowed(validator: &'a Validator) -> Self {
                Self(MetaValidatorInner::Borrowed(validator))
            }

            pub(crate) fn owned(validator: Validator) -> Self {
                Self(MetaValidatorInner::Owned(Box::new(validator), PhantomData))
            }

            /// Validate `schema` against the meta-schema, returning `true` if valid.
            #[must_use]
            pub fn is_valid(&self, schema: &Value) -> bool {
                match &self.0 {
                    _ => self.as_ref().is_valid(schema),
                }
            }

            /// Validate `schema` against the meta-schema, returning the first error if any.
            ///
            /// # Errors
            ///
            /// Returns the first [`ValidationError`] describing why the schema violates the meta-schema.
            pub fn validate<'i>(&self, schema: &'i Value) -> Result<(), ValidationError<'i>> {
                match &self.0 {
                    _ => self.as_ref().validate(schema),
                }
            }

            /// Validate `schema` against the meta-schema, yielding every error.
            #[must_use]
            pub fn iter_errors<'i>(
                &'i self,
                schema: &'i Value,
            ) -> crate::validation::ErrorIterator<'i> {
                match &self.0 {
                    _ => self.as_ref().iter_errors(schema),
                }
            }
        }

        impl AsRef<Validator> for MetaValidator<'_> {
            fn as_ref(&self) -> &Validator {
                match &self.0 {
                    #[cfg(not(target_family = "wasm"))]
                    MetaValidatorInner::Borrowed(validator) => validator,
                    MetaValidatorInner::Owned(validator, _) => validator,
                }
            }
        }

        impl Deref for MetaValidator<'_> {
            type Target = Validator;

            fn deref(&self) -> &Self::Target {
                self.as_ref()
            }
        }
    }

    pub(crate) mod validators {
        use crate::validation::Validator;
        #[cfg(not(target_family = "wasm"))]
        use std::sync::LazyLock;

        fn build_validator(schema: &json_codec::Value) -> Validator {
            crate::validation::options()
                .without_schema_validation()
                .build(schema)
                .expect("Meta-schema should be valid")
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT4_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&crate::reference::meta::DRAFT4));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft4_meta_validator() -> Validator {
            build_validator(&crate::reference::meta::DRAFT4)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT6_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&crate::reference::meta::DRAFT6));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft6_meta_validator() -> Validator {
            build_validator(&crate::reference::meta::DRAFT6)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT7_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&crate::reference::meta::DRAFT7));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft7_meta_validator() -> Validator {
            build_validator(&crate::reference::meta::DRAFT7)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT201909_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&crate::reference::meta::DRAFT201909));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft201909_meta_validator() -> Validator {
            build_validator(&crate::reference::meta::DRAFT201909)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT202012_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&crate::reference::meta::DRAFT202012));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft202012_meta_validator() -> Validator {
            build_validator(&crate::reference::meta::DRAFT202012)
        }

        // Backs the `evaluate`/`iter_errors` fallback for generated meta validators.
    }

    pub(crate) fn validator_for_draft(draft: Draft) -> MetaValidator<'static> {
        #[cfg(not(target_family = "wasm"))]
        {
            match draft {
                Draft::Draft4 => MetaValidator::borrowed(&validators::DRAFT4_META_VALIDATOR),
                Draft::Draft6 => MetaValidator::borrowed(&validators::DRAFT6_META_VALIDATOR),
                Draft::Draft7 => MetaValidator::borrowed(&validators::DRAFT7_META_VALIDATOR),
                Draft::Draft201909 => {
                    MetaValidator::borrowed(&validators::DRAFT201909_META_VALIDATOR)
                }
                // Draft202012, Unknown, or any future draft variants
                _ => MetaValidator::borrowed(&validators::DRAFT202012_META_VALIDATOR),
            }
        }
        #[cfg(target_family = "wasm")]
        {
            let validator = match draft {
                Draft::Draft4 => validators::draft4_meta_validator(),
                Draft::Draft6 => validators::draft6_meta_validator(),
                Draft::Draft7 => validators::draft7_meta_validator(),
                Draft::Draft201909 => validators::draft201909_meta_validator(),
                // Draft202012, Unknown, or any future draft variants
                _ => validators::draft202012_meta_validator(),
            };
            MetaValidator::owned(validator)
        }
    }

    /// Validate a JSON Schema document against its meta-schema and get a `true` if the schema is valid
    /// and `false` otherwise. Draft version is detected automatically.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "string",
    ///     "maxLength": 5
    /// });
    /// assert!(schema_validation::meta::is_valid(&schema));
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics if the meta-schema can't be detected.
    ///
    /// # Note
    ///
    /// This helper only works with the built-in JSON Schema drafts. For schemas that declare a
    /// custom `$schema`, construct a registry that contains your meta-schema and use
    /// [`meta::options().with_registry(...)`](schema_validation::meta::options) to validate it.
    #[must_use]
    pub fn is_valid(schema: &Value) -> bool {
        match try_meta_validator_for(schema, None) {
            Ok(validator) => validator.is_valid(schema),
            Err(error) => panic!("Failed to resolve meta-schema: {error}"),
        }
    }
    /// Validate a JSON Schema document against its meta-schema and return the first error if any.
    /// Draft version is detected automatically.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "string",
    ///     "maxLength": 5
    /// });
    /// assert!(schema_validation::meta::validate(&schema).is_ok());
    ///
    /// // Invalid schema
    /// let invalid_schema = json!({
    ///     "type": "invalid_type"
    /// });
    /// assert!(schema_validation::meta::validate(&invalid_schema).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] describing why the schema violates the detected meta-schema.
    ///
    /// # Panics
    ///
    /// This function panics if the meta-schema can't be detected.
    ///
    /// # Note
    ///
    /// Like [`is_valid`], this helper only handles the bundled JSON
    /// Schema drafts. For custom meta-schemas, use [`meta::options().with_registry(...)`](schema_validation::meta::options)
    /// so the registry can supply the meta-schema.
    pub fn validate(schema: &Value) -> Result<(), ValidationError<'_>> {
        let validator = try_meta_validator_for(schema, None)?;
        validator.validate(schema)
    }

    /// Build a validator for a JSON Schema's meta-schema.
    /// Draft version is detected automatically.
    ///
    /// Returns a [`MetaValidator`] that can be used to validate the schema or access
    /// structured validation output via the evaluate API.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({
    ///     "type": "string",
    ///     "maxLength": 5
    /// });
    ///
    /// let validator = schema_validation::meta::validator_for(&schema)
    ///     .expect("Valid meta-schema");
    ///
    /// // Use evaluate API for structured output
    /// let evaluation = validator.evaluate(&schema);
    /// assert!(evaluation.flag().valid);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] if the meta-schema cannot be resolved or built.
    ///
    /// # Panics
    ///
    /// This function panics if the meta-schema can't be detected.
    ///
    /// # Note
    ///
    /// This helper only handles the bundled JSON Schema drafts. For custom meta-schemas,
    /// use [`meta::options().with_registry(...)`](schema_validation::meta::options).
    pub fn validator_for(
        schema: &Value,
    ) -> Result<MetaValidator<'static>, ValidationError<'static>> {
        try_meta_validator_for(schema, None)
    }

    /// Validate a schema document held in a foreign representation against its meta-schema.
    /// Draft version is detected automatically.
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`], or a referencing error if the meta-schema cannot be
    /// resolved.
    ///
    /// # Panics
    ///
    /// Panics if a bundled meta-schema fails to build.
    pub fn validate_for<F: Json>(schema: F::Node<'_>) -> Result<(), ValidationError<'_>> {
        let cache = meta_cache::<F>();
        match cache.draft_of(&schema) {
            Draft::Unknown => custom_meta_validator(cache, &schema)?.validate(schema),
            draft => cache.validator(draft).validate(schema),
        }
    }

    /// Check a schema document held in a foreign representation against its meta-schema.
    /// Draft version is detected automatically.
    ///
    /// # Errors
    ///
    /// Returns a referencing error if the meta-schema cannot be resolved.
    ///
    /// # Panics
    ///
    /// Panics if a bundled meta-schema fails to build.
    pub fn is_valid_for<F: Json>(schema: F::Node<'_>) -> Result<bool, ValidationError<'static>> {
        let cache = meta_cache::<F>();
        Ok(match cache.draft_of(&schema) {
            Draft::Unknown => custom_meta_validator(cache, &schema)?.is_valid(schema),
            draft => cache.validator(draft).is_valid(schema),
        })
    }

    /// Meta-schemas outside the bundled drafts are resolved through the `$schema` chain.
    fn custom_meta_validator<F: Json>(
        cache: &MetaCache<F>,
        schema: &F::Node<'_>,
    ) -> Result<Validator<F>, ValidationError<'static>> {
        let uri = cache
            .meta_schema_uri(schema)
            .expect("`$schema` must exist when draft is Unknown")
            .into_owned();
        let (custom_meta_schema, resolved_draft) = resolve_meta_schema_chain(&uri)?;
        crate::validation::options_for::<F>()
            .with_draft(resolved_draft)
            .without_schema_validation()
            .build(&custom_meta_schema)
    }

    /// A `Validator<F>` only accepts nodes of the representation it was built for, so meta-schema
    /// state is shared per `F` rather than globally.
    struct MetaCache<F: Json> {
        schema_key: F::PreparedKey,
        validators: [OnceLock<Validator<F>>; 5],
    }

    impl<F: Json> MetaCache<F> {
        fn meta_schema_uri<'a>(&self, schema: &F::Node<'a>) -> Option<Cow<'a, str>> {
            schema.as_object()?.get(&self.schema_key)?.as_string()
        }

        fn draft_of(&self, schema: &F::Node<'_>) -> Draft {
            self.meta_schema_uri(schema)
                .map_or_else(Draft::default, |uri| Draft::from_schema_uri(&uri))
        }

        fn validator(&self, draft: Draft) -> &Validator<F> {
            let (index, meta_schema) = match draft {
                Draft::Draft4 => (0, &crate::reference::meta::DRAFT4),
                Draft::Draft6 => (1, &crate::reference::meta::DRAFT6),
                Draft::Draft7 => (2, &crate::reference::meta::DRAFT7),
                Draft::Draft201909 => (3, &crate::reference::meta::DRAFT201909),
                // Draft202012, Unknown, or any future draft variants
                _ => (4, &crate::reference::meta::DRAFT202012),
            };
            self.validators[index].get_or_init(|| {
                crate::validation::options_for::<F>()
                    .without_schema_validation()
                    .build(meta_schema)
                    .expect("Meta-schema should be valid")
            })
        }
    }

    /// Statics cannot be generic over `F`, so entries are found by type. There is one per
    /// representation reaching this code, which a linear scan covers.
    fn meta_cache<F: Json>() -> &'static MetaCache<F> {
        static CACHE: RwLock<Vec<(TypeId, &'static (dyn Any + Send + Sync))>> =
            RwLock::new(Vec::new());

        fn find<F: Json>(
            entries: &[(TypeId, &'static (dyn Any + Send + Sync))],
        ) -> Option<&'static MetaCache<F>> {
            let type_id = TypeId::of::<F>();
            entries.iter().find(|(id, _)| *id == type_id).map(|(_, e)| {
                e.downcast_ref()
                    .expect("Entries are found by representation type")
            })
        }

        if let Some(cache) = find::<F>(&CACHE.read().expect("Meta-validator cache is poisoned")) {
            return cache;
        }
        let mut entries = CACHE.write().expect("Meta-validator cache is poisoned");
        if let Some(cache) = find::<F>(&entries) {
            return cache;
        }
        let cache: &'static MetaCache<F> = Box::leak(Box::new(MetaCache {
            schema_key: F::prepare_key("$schema"),
            validators: Default::default(),
        }));
        entries.push((TypeId::of::<F>(), cache));
        cache
    }

    fn try_meta_validator_for<'a>(
        schema: &Value,
        registry: Option<&'a Registry<'a>>,
    ) -> Result<MetaValidator<'a>, ValidationError<'static>> {
        let draft = Draft::default().detect(schema);

        // For custom meta-schemas (Draft::Unknown), attempt to resolve the meta-schema
        if draft == Draft::Unknown {
            if let Some(meta_schema_uri) = schema
                .as_object()
                .and_then(|obj| obj.get("$schema"))
                .and_then(|s| s.as_str())
            {
                // Try registry first if available
                if let Some(registry) = registry {
                    let (custom_meta_schema, resolved_draft) =
                        resolve_meta_schema_with_registry(meta_schema_uri, registry)?;
                    let validator = crate::validation::options()
                        .with_draft(resolved_draft)
                        .with_registry(registry)
                        .with_base_uri(meta_schema_uri.trim_end_matches('#'))
                        .without_schema_validation()
                        .build(&custom_meta_schema)?;
                    return Ok(MetaValidator::owned(validator));
                }

                // Use default retriever
                let (custom_meta_schema, resolved_draft) =
                    resolve_meta_schema_chain(meta_schema_uri)?;
                let validator = crate::validation::options()
                    .with_draft(resolved_draft)
                    .without_schema_validation()
                    .build(&custom_meta_schema)?;
                return Ok(MetaValidator::owned(validator));
            }
        }

        Ok(validator_for_draft(draft))
    }

    fn resolve_meta_schema_with_registry(
        uri: &str,
        registry: &Registry<'_>,
    ) -> Result<(Value, Draft), ValidationError<'static>> {
        let resolver = registry.resolver(crate::reference::uri::from_str(uri)?);
        let first_resolved = resolver.lookup("")?;
        let first_meta_schema = first_resolved.contents().clone();

        let draft = walk_meta_schema_chain(uri, |current_uri| {
            let resolver = registry.resolver(crate::reference::uri::from_str(current_uri)?);
            let resolved = resolver.lookup("")?;
            Ok(resolved.contents().clone())
        })?;

        Ok((first_meta_schema, draft))
    }

    fn resolve_meta_schema_chain(uri: &str) -> Result<(Value, Draft), ValidationError<'static>> {
        let retriever = crate::validation::retriever::DefaultRetriever;
        let first_meta_uri = crate::reference::uri::from_str(uri)?;
        let first_meta_schema = retriever
            .retrieve(&first_meta_uri)
            .map_err(|e| crate::reference::Error::unretrievable(uri, e))?;

        let draft = walk_meta_schema_chain(uri, |current_uri| {
            let meta_uri = crate::reference::uri::from_str(current_uri)?;
            retriever
                .retrieve(&meta_uri)
                .map_err(|e| crate::reference::Error::unretrievable(current_uri, e))
        })?;

        Ok((first_meta_schema, draft))
    }

    pub(crate) fn walk_meta_schema_chain(
        start_uri: &str,
        mut fetch: impl FnMut(&str) -> Result<Value, crate::reference::Error>,
    ) -> Result<Draft, crate::reference::Error> {
        let mut visited = AHashSet::new();
        let mut current_uri = start_uri.to_string();

        loop {
            if !visited.insert(current_uri.clone()) {
                return Err(crate::reference::Error::circular_metaschema(current_uri));
            }

            let meta_schema = fetch(&current_uri)?;
            let draft = Draft::default().detect(&meta_schema);

            if draft != Draft::Unknown {
                return Ok(draft);
            }

            current_uri = meta_schema
                .get("$schema")
                .and_then(|s| s.as_str())
                .expect("`$schema` must exist when draft is Unknown")
                .to_string();
        }
    }
}

/// Functionality specific to JSON Schema Draft 4.
///
/// [![Draft 4](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft4.json)](https://bowtie.report/#/implementations/rust-jsonschema)
///
/// This module provides functions for creating validators and performing validation
/// according to the JSON Schema Draft 4 specification.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "number", "multipleOf": 2});
/// let instance = json!(4);
///
/// assert!(schema_validation::draft4::is_valid(&schema, &instance));
/// ```
pub mod draft4 {
    use super::{Draft, ValidationError, ValidationOptions, Validator, Value};

    /// Create a new JSON Schema validator using Draft 4 specifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let instance = json!(42);
    ///
    /// let validator = schema_validation::draft4::new(&schema)?;
    /// assert!(validator.is_valid(&instance));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not a valid Draft 4 document or if referenced resources
    /// cannot be resolved.
    pub fn new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        options().build(schema)
    }
    /// Validate an instance against a schema using Draft 4 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft4::is_valid(&schema, &valid));
    /// assert!(!schema_validation::draft4::is_valid(&schema, &invalid));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 4 validator.
    #[must_use]
    pub fn is_valid(schema: &Value, instance: &Value) -> bool {
        new(schema).expect("Invalid schema").is_valid(instance)
    }
    /// Validate an instance against a schema using Draft 4 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft4::validate(&schema, &valid).is_ok());
    /// assert!(schema_validation::draft4::validate(&schema, &invalid).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] when `instance` violates the schema.
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 4 validator.
    pub fn validate<'i>(schema: &Value, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        new(schema).expect("Invalid schema").validate(instance)
    }
    /// Creates a [`ValidationOptions`] builder pre-configured for JSON Schema Draft 4.
    ///
    /// This function provides a shorthand for `schema_validation::options().with_draft(Draft::Draft4)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = schema_validation::draft4::options()
    ///     .with_format("ends-with-42", |s| s.ends_with("42"))
    ///     .should_validate_formats(true)
    ///     .build(&schema)?;
    ///
    /// assert!(validator.is_valid(&json!("Hello 42")));
    /// assert!(!validator.is_valid(&json!("No!")));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`ValidationOptions`] for all available configuration options.
    #[must_use]
    pub fn options<'i>() -> ValidationOptions<'i> {
        crate::validation::options().with_draft(Draft::Draft4)
    }

    /// Functionality for validating JSON Schema Draft 4 documents.
    pub mod meta {
        use crate::validation::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 4 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::validation::meta::validator_for_draft(super::Draft::Draft4)
        }

        /// Validate a JSON Schema document against Draft 4 meta-schema and get a `true` if the schema is valid
        /// and `false` otherwise.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft4::meta::is_valid(&schema));
        /// ```
        #[must_use]
        #[inline]
        pub fn is_valid(schema: &Value) -> bool {
            validator().is_valid(schema)
        }

        /// Validate a JSON Schema document against Draft 4 meta-schema and return the first error if any.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft4::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(schema_validation::draft4::meta::validate(&invalid_schema).is_err());
        /// ```
        ///
        /// # Errors
        ///
        /// Returns the first [`ValidationError`] describing why the schema violates the Draft 4 meta-schema.
        #[inline]
        pub fn validate(schema: &Value) -> Result<(), ValidationError<'_>> {
            validator().validate(schema)
        }
    }
}

/// Functionality specific to JSON Schema Draft 6.
///
/// [![Draft 6](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft6.json)](https://bowtie.report/#/implementations/rust-jsonschema)
///
/// This module provides functions for creating validators and performing validation
/// according to the JSON Schema Draft 6 specification.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "string", "format": "uri"});
/// let instance = json!("https://www.example.com");
///
/// assert!(schema_validation::draft6::is_valid(&schema, &instance));
/// ```
pub mod draft6 {
    use super::{Draft, ValidationError, ValidationOptions, Validator, Value};

    /// Create a new JSON Schema validator using Draft 6 specifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let instance = json!(42);
    ///
    /// let validator = schema_validation::draft6::new(&schema)?;
    /// assert!(validator.is_valid(&instance));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not a valid Draft 6 document or if referenced resources
    /// cannot be resolved.
    pub fn new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        options().build(schema)
    }
    /// Validate an instance against a schema using Draft 6 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft6::is_valid(&schema, &valid));
    /// assert!(!schema_validation::draft6::is_valid(&schema, &invalid));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 6 validator.
    #[must_use]
    pub fn is_valid(schema: &Value, instance: &Value) -> bool {
        new(schema).expect("Invalid schema").is_valid(instance)
    }
    /// Validate an instance against a schema using Draft 6 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft6::validate(&schema, &valid).is_ok());
    /// assert!(schema_validation::draft6::validate(&schema, &invalid).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] when `instance` violates the schema.
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 6 validator.
    pub fn validate<'i>(schema: &Value, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        new(schema).expect("Invalid schema").validate(instance)
    }
    /// Creates a [`ValidationOptions`] builder pre-configured for JSON Schema Draft 6.
    ///
    /// This function provides a shorthand for `schema_validation::options().with_draft(Draft::Draft6)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = schema_validation::draft6::options()
    ///     .with_format("ends-with-42", |s| s.ends_with("42"))
    ///     .should_validate_formats(true)
    ///     .build(&schema)?;
    ///
    /// assert!(validator.is_valid(&json!("Hello 42")));
    /// assert!(!validator.is_valid(&json!("No!")));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`ValidationOptions`] for all available configuration options.
    #[must_use]
    pub fn options<'i>() -> ValidationOptions<'i> {
        crate::validation::options().with_draft(Draft::Draft6)
    }

    /// Functionality for validating JSON Schema Draft 6 documents.
    pub mod meta {
        use crate::validation::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 6 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::validation::meta::validator_for_draft(super::Draft::Draft6)
        }

        /// Validate a JSON Schema document against Draft 6 meta-schema and get a `true` if the schema is valid
        /// and `false` otherwise.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft6::meta::is_valid(&schema));
        /// ```
        #[must_use]
        #[inline]
        pub fn is_valid(schema: &Value) -> bool {
            validator().is_valid(schema)
        }

        /// Validate a JSON Schema document against Draft 6 meta-schema and return the first error if any.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft6::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(schema_validation::draft6::meta::validate(&invalid_schema).is_err());
        /// ```
        ///
        /// # Errors
        ///
        /// Returns the first [`ValidationError`] describing why the schema violates the Draft 6 meta-schema.
        #[inline]
        pub fn validate(schema: &Value) -> Result<(), ValidationError<'_>> {
            validator().validate(schema)
        }
    }
}

/// Functionality specific to JSON Schema Draft 7.
///
/// [![Draft 7](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft7.json)](https://bowtie.report/#/implementations/rust-jsonschema)
///
/// This module provides functions for creating validators and performing validation
/// according to the JSON Schema Draft 7 specification.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "string", "pattern": "^[a-zA-Z0-9]+$"});
/// let instance = json!("abc123");
///
/// assert!(schema_validation::draft7::is_valid(&schema, &instance));
/// ```
pub mod draft7 {
    use super::{Draft, ValidationError, ValidationOptions, Validator, Value};

    /// Create a new JSON Schema validator using Draft 7 specifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let instance = json!(42);
    ///
    /// let validator = schema_validation::draft7::new(&schema)?;
    /// assert!(validator.is_valid(&instance));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not a valid Draft 7 document or if referenced resources
    /// cannot be resolved.
    pub fn new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        options().build(schema)
    }
    /// Validate an instance against a schema using Draft 7 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft7::is_valid(&schema, &valid));
    /// assert!(!schema_validation::draft7::is_valid(&schema, &invalid));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 7 validator.
    #[must_use]
    pub fn is_valid(schema: &Value, instance: &Value) -> bool {
        new(schema).expect("Invalid schema").is_valid(instance)
    }
    /// Validate an instance against a schema using Draft 7 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft7::validate(&schema, &valid).is_ok());
    /// assert!(schema_validation::draft7::validate(&schema, &invalid).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] when `instance` violates the schema.
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 7 validator.
    pub fn validate<'i>(schema: &Value, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        new(schema).expect("Invalid schema").validate(instance)
    }
    /// Creates a [`ValidationOptions`] builder pre-configured for JSON Schema Draft 7.
    ///
    /// This function provides a shorthand for `schema_validation::options().with_draft(Draft::Draft7)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = schema_validation::draft7::options()
    ///     .with_format("ends-with-42", |s| s.ends_with("42"))
    ///     .should_validate_formats(true)
    ///     .build(&schema)?;
    ///
    /// assert!(validator.is_valid(&json!("Hello 42")));
    /// assert!(!validator.is_valid(&json!("No!")));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`ValidationOptions`] for all available configuration options.
    #[must_use]
    pub fn options<'i>() -> ValidationOptions<'i> {
        crate::validation::options().with_draft(Draft::Draft7)
    }

    /// Functionality for validating JSON Schema Draft 7 documents.
    pub mod meta {
        use crate::validation::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 7 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::validation::meta::validator_for_draft(super::Draft::Draft7)
        }

        /// Validate a JSON Schema document against Draft 7 meta-schema and get a `true` if the schema is valid
        /// and `false` otherwise.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft7::meta::is_valid(&schema));
        /// ```
        #[must_use]
        #[inline]
        pub fn is_valid(schema: &Value) -> bool {
            validator().is_valid(schema)
        }

        /// Validate a JSON Schema document against Draft 7 meta-schema and return the first error if any.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft7::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(schema_validation::draft7::meta::validate(&invalid_schema).is_err());
        /// ```
        ///
        /// # Errors
        ///
        /// Returns the first [`ValidationError`] describing why the schema violates the Draft 7 meta-schema.
        #[inline]
        pub fn validate(schema: &Value) -> Result<(), ValidationError<'_>> {
            validator().validate(schema)
        }
    }
}

/// Functionality specific to JSON Schema Draft 2019-09.
///
/// [![Draft 2019-09](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft2019-09.json)](https://bowtie.report/#/implementations/rust-jsonschema)
///
/// This module provides functions for creating validators and performing validation
/// according to the JSON Schema Draft 2019-09 specification.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "array", "minItems": 2, "uniqueItems": true});
/// let instance = json!([1, 2]);
///
/// assert!(schema_validation::draft201909::is_valid(&schema, &instance));
/// ```
pub mod draft201909 {
    use super::{Draft, ValidationError, ValidationOptions, Validator, Value};

    /// Create a new JSON Schema validator using Draft 2019-09 specifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let instance = json!(42);
    ///
    /// let validator = schema_validation::draft201909::new(&schema)?;
    /// assert!(validator.is_valid(&instance));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not a valid Draft 2019-09 document or if referenced resources
    /// cannot be resolved.
    pub fn new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        options().build(schema)
    }
    /// Validate an instance against a schema using Draft 2019-09 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft201909::is_valid(&schema, &valid));
    /// assert!(!schema_validation::draft201909::is_valid(&schema, &invalid));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 2019-09 validator.
    #[must_use]
    pub fn is_valid(schema: &Value, instance: &Value) -> bool {
        new(schema).expect("Invalid schema").is_valid(instance)
    }
    /// Validate an instance against a schema using Draft 2019-09 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft201909::validate(&schema, &valid).is_ok());
    /// assert!(schema_validation::draft201909::validate(&schema, &invalid).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] when `instance` violates the schema.
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 2019-09 validator.
    pub fn validate<'i>(schema: &Value, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        new(schema).expect("Invalid schema").validate(instance)
    }
    /// Creates a [`ValidationOptions`] builder pre-configured for JSON Schema Draft 2019-09.
    ///
    /// This function provides a shorthand for `schema_validation::options().with_draft(Draft::Draft201909)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = schema_validation::draft201909::options()
    ///     .with_format("ends-with-42", |s| s.ends_with("42"))
    ///     .should_validate_formats(true)
    ///     .build(&schema)?;
    ///
    /// assert!(validator.is_valid(&json!("Hello 42")));
    /// assert!(!validator.is_valid(&json!("No!")));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`ValidationOptions`] for all available configuration options.
    #[must_use]
    pub fn options<'i>() -> ValidationOptions<'i> {
        crate::validation::options().with_draft(Draft::Draft201909)
    }

    /// Functionality for validating JSON Schema Draft 2019-09 documents.
    pub mod meta {
        use crate::validation::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 2019-09 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::validation::meta::validator_for_draft(super::Draft::Draft201909)
        }
        /// Validate a JSON Schema document against Draft 2019-09 meta-schema and get a `true` if the schema is valid
        /// and `false` otherwise.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft201909::meta::is_valid(&schema));
        /// ```
        #[must_use]
        #[inline]
        pub fn is_valid(schema: &Value) -> bool {
            validator().is_valid(schema)
        }

        /// Validate a JSON Schema document against Draft 2019-09 meta-schema and return the first error if any.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft201909::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(schema_validation::draft201909::meta::validate(&invalid_schema).is_err());
        /// ```
        ///
        /// # Errors
        ///
        /// Returns the first [`ValidationError`] describing why the schema violates the Draft 2019-09 meta-schema.
        #[inline]
        pub fn validate(schema: &Value) -> Result<(), ValidationError<'_>> {
            validator().validate(schema)
        }
    }
}

/// Functionality specific to JSON Schema Draft 2020-12.
///
/// [![Draft 2020-12](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft2020-12.json)](https://bowtie.report/#/implementations/rust-jsonschema)
///
/// This module provides functions for creating validators and performing validation
/// according to the JSON Schema Draft 2020-12 specification.
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use json_codec::json;
///
/// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]});
/// let instance = json!({"name": "John Doe"});
///
/// assert!(schema_validation::draft202012::is_valid(&schema, &instance));
/// # Ok(())
/// # }
/// ```
pub mod draft202012 {
    use super::{Draft, ValidationError, ValidationOptions, Validator, Value};

    /// Create a new JSON Schema validator using Draft 2020-12 specifications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let instance = json!(42);
    ///
    /// let validator = schema_validation::draft202012::new(&schema)?;
    /// assert!(validator.is_valid(&instance));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not a valid Draft 2020-12 document or if referenced resources
    /// cannot be resolved.
    pub fn new(schema: &Value) -> Result<Validator, ValidationError<'static>> {
        options().build(schema)
    }
    /// Validate an instance against a schema using Draft 2020-12 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft202012::is_valid(&schema, &valid));
    /// assert!(!schema_validation::draft202012::is_valid(&schema, &invalid));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 2020-12 validator.
    #[must_use]
    pub fn is_valid(schema: &Value, instance: &Value) -> bool {
        new(schema).expect("Invalid schema").is_valid(instance)
    }
    /// Validate an instance against a schema using Draft 2020-12 specifications without creating a validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    ///
    /// let schema = json!({"minimum": 5});
    /// let valid = json!(42);
    /// let invalid = json!(3);
    ///
    /// assert!(schema_validation::draft202012::validate(&schema, &valid).is_ok());
    /// assert!(schema_validation::draft202012::validate(&schema, &invalid).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the first [`ValidationError`] when `instance` violates the schema.
    ///
    /// # Panics
    ///
    /// Panics if `schema` cannot be compiled into a Draft 2020-12 validator.
    pub fn validate<'i>(schema: &Value, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        new(schema).expect("Invalid schema").validate(instance)
    }
    /// Creates a [`ValidationOptions`] builder pre-configured for JSON Schema Draft 2020-12.
    ///
    /// This function provides a shorthand for `schema_validation::options().with_draft(Draft::Draft202012)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = schema_validation::draft202012::options()
    ///     .with_format("ends-with-42", |s| s.ends_with("42"))
    ///     .should_validate_formats(true)
    ///     .build(&schema)?;
    ///
    /// assert!(validator.is_valid(&json!("Hello 42")));
    /// assert!(!validator.is_valid(&json!("No!")));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`ValidationOptions`] for all available configuration options.
    #[must_use]
    pub fn options<'i>() -> ValidationOptions<'i> {
        crate::validation::options().with_draft(Draft::Draft202012)
    }

    /// Functionality for validating JSON Schema Draft 2020-12 documents.
    pub mod meta {
        use crate::validation::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 2020-12 meta-schema validator. Native targets borrow
        /// cached statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::validation::meta::validator_for_draft(super::Draft::Draft202012)
        }

        /// Validate a JSON Schema document against Draft 2020-12 meta-schema and get a `true` if the schema is valid
        /// and `false` otherwise.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft202012::meta::is_valid(&schema));
        /// ```
        #[must_use]
        #[inline]
        pub fn is_valid(schema: &Value) -> bool {
            validator().is_valid(schema)
        }

        /// Validate a JSON Schema document against Draft 2020-12 meta-schema and return the first error if any.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use json_codec::json;
        ///
        /// let schema = json!({
        ///     "type": "string",
        ///     "maxLength": 5
        /// });
        /// assert!(schema_validation::draft202012::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(schema_validation::draft202012::meta::validate(&invalid_schema).is_err());
        /// ```
        ///
        /// # Errors
        ///
        /// Returns the first [`ValidationError`] describing why the schema violates the Draft 2020-12 meta-schema.
        #[inline]
        pub fn validate(schema: &Value) -> Result<(), ValidationError<'_>> {
            validator().validate(schema)
        }
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/lib/tests_util_4365.rs"
));

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/lib/tests_4700.rs"
));

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/lib/all_async_tests_4371.rs"
));
