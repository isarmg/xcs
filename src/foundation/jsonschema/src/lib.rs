#![allow(clippy::unnecessary_wraps)]
// Runtime helpers stay `pub` for the macros-gated `__private` re-exports; without `macros` they are crate-internal.
#![cfg_attr(not(feature = "macros"), allow(unreachable_pub))]
//! A high-performance JSON Schema validator for Rust.
//!
//! - 📚 Support for popular JSON Schema drafts
//! - 🔧 Custom keywords and format validators
//! - ⚡ [Compile-time validators](#compile-time-validator-macro), also for
//!   [Python](#python-extension-modules) and [Ruby](#ruby-extension-modules) extension modules
//! - 🌐 Blocking & non-blocking remote reference fetching (network/file)
//! - 🎨 Structured Output v1 reports (flag/list/hierarchical)
//! - ✨ Meta-schema validation for schema documents
//! - 🧮 Schema canonicalization (experimental; see the [`canonical`] module)
//! - 🧩 Validation of custom in-memory JSON representations
//! - 🚀 WebAssembly support
//!
//! ## Supported drafts
//!
//! Compliance levels vary across drafts, with newer versions having some unimplemented keywords.
//!
//! - ![Draft 2020-12](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft2020-12.json)
//! - ![Draft 2019-09](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft2019-09.json)
//! - ![Draft 7](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft7.json)
//! - ![Draft 6](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft6.json)
//! - ![Draft 4](https://img.shields.io/endpoint?url=https%3A%2F%2Fbowtie.report%2Fbadges%2Frust-jsonschema%2Fcompliance%2Fdraft4.json)
//!
//! # Validation
//!
//! The `jsonschema` crate offers two main approaches to validation: one-off validation and reusable validators.
//! When external references are involved, the validator can be constructed using either blocking or non-blocking I/O.
//!
//!
//! For simple use cases where you need to validate an instance against a schema once, use [`is_valid`] or [`validate`] functions:
//!
//! ```rust
//! use json_codec::json;
//!
//! let schema = json!({"type": "string"});
//! let instance = json!("Hello, world!");
//!
//! assert!(jsonschema::is_valid(&schema, &instance));
//! assert!(jsonschema::validate(&schema, &instance).is_ok());
//! ```
//!
//! For better performance, especially when validating multiple instances against the same schema, build a validator once and reuse it:
//! If your schema contains external references, you can choose between blocking and non-blocking construction:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"type": "string"});
//! // Blocking construction - will fetch external references synchronously
//! let validator = jsonschema::validator_for(&schema)?;
//! // Non-blocking construction - will fetch external references asynchronously
//! # #[cfg(feature = "resolve-async")]
//! # async fn async_example() -> Result<(), Box<dyn std::error::Error>> {
//! # let schema = json!({"type": "string"});
//! let validator = jsonschema::async_validator_for(&schema).await?;
//! # Ok(())
//! # }
//!
//!  // Once constructed, validation is always synchronous as it works with in-memory data
//! assert!(validator.is_valid(&json!("Hello, world!")));
//! assert!(!validator.is_valid(&json!(42)));
//! assert!(validator.validate(&json!(42)).is_err());
//!
//! // Iterate over all errors
//! let instance = json!(42);
//! for error in validator.iter_errors(&instance) {
//!     eprintln!("Error: {}", error);
//!     eprintln!("Location: {}", error.instance_path());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Note on `format` keyword
//!
//! By default, format validation is draft‑dependent. To opt in for format checks, you can configure your validator like this:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use json_codec::json;
//! #
//! # let schema = json!({"type": "string"});
//! let validator = jsonschema::draft202012::options()
//!     .should_validate_formats(true)
//!     .build(&schema)?;
//! # Ok(())
//! # }
//! ```
//!
//! Once built, any `format` keywords in your schema will be actively validated according to the chosen draft.
//!
//! `idn-hostname` and `idn-email` need the `idna` feature, which is on by default. Without it they
//! are treated as unknown formats.
//!
//! # Reading errors
//!
//! [`ErrorIterator::into_errors`] collects the whole set into a [`ValidationErrors`], which
//! implements `Display` and [`std::error::Error`], so reporting every failure takes no formatting
//! loop of its own:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let validator = jsonschema::validator_for(&json!({
//!     "type": "object",
//!     "properties": {"n": {"minimum": 5}},
//!     "required": ["name"]
//! }))?;
//! let instance = json!({"n": 1});
//! let errors = validator.iter_errors(&instance).into_errors();
//!
//! assert_eq!(errors.len(), 2);
//! // Validation errors:
//! // 01: "name" is a required property
//! // 02: /n: 1 is less than the minimum of 5
//! println!("{errors}");
//! # Ok(())
//! # }
//! ```
//!
//! [`ValidationError::kind`] carries the failed keyword's operands, so a caller does not need to
//! re-read the schema to describe what was expected:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//! use jsonschema::error::ValidationErrorKind;
//!
//! let validator = jsonschema::validator_for(&json!({"minimum": 5}))?;
//! let instance = json!(1);
//! let error = validator.validate(&instance).expect_err("must be invalid");
//!
//! match error.kind() {
//!     ValidationErrorKind::Minimum { limit } => assert_eq!(limit, &json!(5)),
//!     other => panic!("unexpected kind: {other:?}"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! [`ValidationError::absolute_keyword_location`] gives the resolvable URI of the keyword that
//! produced the error, and [`ValidationError::masked`] hides the instance value in the message.
//!
//! # Compile-Time Validator Macro
//!
//! The `validator` attribute macro (enabled by the `macros` feature) compiles schemas at build time:
//! Generated validators are significantly faster than runtime validators, so prefer them when the schema is known at
//! build time.
//!
//! ```ignore
//! // Inline schema
//! #[jsonschema::validator(schema = r#"{"maxLength": 5}"#)]
//! struct Short;
//!
//! // Or load it from a file:
//! // #[jsonschema::validator(path = "schema.json")]
//!
//! let instance = json_codec::json!("value");
//! assert!(Short::is_valid(&instance));
//! Short::validate(&instance)?;
//! ```
//!
//! Supported macro attributes; each mirrors the matching [`ValidationOptions`] builder method,
//! which documents its behavior and defaults:
//!
//! - schema source (exactly one required): `path = "..."` (file) or `schema = r#"..."#` (inline)
//! - `backend = SerdeJson|Pyo3|Magnus` (default: `SerdeJson`), the representation the generated
//!   validator reads; `Pyo3` needs the `pyo3` feature (see
//!   [Python Extension Modules](#python-extension-modules)) and `Magnus` the `magnus` feature (see
//!   [Ruby Extension Modules](#ruby-extension-modules))
//! - `draft = Draft202012` (or another variant) -> [`ValidationOptions::with_draft`]
//! - `base_uri = "..."` -> [`ValidationOptions::with_base_uri`]
//! - `resources = { "<uri>" => { schema = r#"..."# } | { path = "..." } }`
//! - `vocabularies = ["<uri>"]` -> [`ValidationOptions::with_vocabulary`]
//! - `validate_formats = true|false` -> [`ValidationOptions::should_validate_formats`]
//! - `ignore_unknown_formats = true|false` -> [`ValidationOptions::should_ignore_unknown_formats`]
//! - `formats = { "name" => crate::path::to::fn }` -> [`ValidationOptions::with_format`]
//! - `keywords = { "name" => crate::path::to::factory }` -> [`ValidationOptions::with_keyword`]
//! - `content_media_types = { "type" => crate::path::to::fn }` -> [`ValidationOptions::with_content_media_type`]
//! - `content_encodings = { "name" => { check = ..., convert = ... } }` -> [`ValidationOptions::with_content_encoding`]
//! - `pattern_options = { ... }` -> [`PatternOptions`]
//! - `email_options = { ... }` -> [`EmailOptions`]
//! - `methods = { is_valid = true, validate = false, iter_errors = false }`, which methods to
//!   generate (all default to `true`; at least one must stay enabled)
//!
//! ## Python Extension Modules
//!
//! With `backend = Pyo3`, the generated validator reads Python objects in place, for extension
//! modules that know their schemas at build time. The methods take `&Bound<'_, PyAny>` and return
//! `PyResult<...>`, which is an error when the instance holds a value with no JSON counterpart,
//! such as a `set`:
//!
//! ```ignore
//! use pyo3::prelude::*;
//!
//! #[jsonschema::validator(path = "event.json", backend = Pyo3)]
//! struct Event;
//!
//! #[pyfunction]
//! fn is_valid_event(instance: &Bound<'_, PyAny>) -> PyResult<bool> {
//!     Event::is_valid(instance)
//! }
//! ```
//!
//! A `keywords` factory for this backend returns `Box<dyn for<'i> Keyword<'i, json::Pyo3>>`. A
//! complete extension with its build and test commands lives in
//! [`examples/pyo3-extension`](https://github.com/Stranger6667/jsonschema/tree/master/examples/pyo3-extension).
//!
//! Building the extension with `PyO3`'s `abi3` features limits it to the stable Python API, where
//! list and tuple elements are read through function calls; a build per Python version reads them
//! in place, which is faster on array-heavy instances.
//!
//! ## Ruby Extension Modules
//!
//! With `backend = Magnus`, the generated validator reads Ruby objects in place, for extension
//! modules that know their schemas at build time. The methods take `&magnus::Value` and return
//! `Result<..., magnus::Error>`, which is an error when the instance holds a value with no JSON
//! counterpart, such as a `Regexp`. They must run on a thread holding the GVL, as any Ruby call:
//!
//! ```ignore
//! use magnus::{function, Error, Ruby, Value};
//!
//! #[jsonschema::validator(path = "event.json", backend = Magnus)]
//! struct Event;
//!
//! fn valid_event(instance: Value) -> Result<bool, Error> {
//!     Event::is_valid(&instance)
//! }
//!
//! #[magnus::init]
//! fn init(ruby: &Ruby) -> Result<(), Error> {
//!     ruby.define_global_function("valid_event?", function!(valid_event, 1));
//!     Ok(())
//! }
//! ```
//!
//! A `keywords` factory for this backend returns `Box<dyn for<'i> Keyword<'i, json::Magnus>>`. A
//! complete extension with its build and test commands lives in
//! [`examples/magnus-extension`](https://github.com/Stranger6667/jsonschema/tree/master/examples/magnus-extension).
//!
//! ## Limitations
//!
//! - `is_valid`, `validate`, and `iter_errors` are generated; `evaluate` is not implemented yet.
//! - Custom keywords cannot override built-in ones: a `keywords` entry named like a built-in keyword runs in
//!   addition to the built-in check, not instead of it.
//! - When an instance violates several keywords, the first error reported by `validate()` may differ from the runtime
//!   validator's, since generated checks are not ordered by keyword priority; `is_valid` and validity are unaffected.
//!
//! # Structured Output
//!
//! The `evaluate()` method provides access to structured validation output formats defined by
//! [JSON Schema Output v1](https://github.com/json-schema-org/json-schema-spec/blob/main/specs/output/jsonschema-validation-output-machines.md).
//! This is useful when you need detailed information about the validation process beyond simple pass/fail results.
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": {"type": "string"},
//!         "age": {"type": "number", "minimum": 0}
//!     },
//!     "required": ["name"]
//! });
//!
//! let validator = jsonschema::validator_for(&schema)?;
//! let instance = json!({"name": "Alice", "age": 30});
//!
//! // Evaluate the instance
//! let evaluation = validator.evaluate(&instance);
//!
//! // Flag format: Simple boolean validity
//! let flag = evaluation.flag();
//! assert!(flag.valid);
//!
//! // List format: Flat list of all evaluation steps
//! let list_output = json_codec::to_value(evaluation.list())?;
//! println!("List output: {}", json_codec::to_string_pretty(&list_output)?);
//!
//! // Hierarchical format: Nested tree structure
//! let hierarchical_output = json_codec::to_value(evaluation.hierarchical())?;
//! println!(
//!     "Hierarchical output: {}",
//!     json_codec::to_string_pretty(&hierarchical_output)?
//! );
//!
//! // Iterate over annotations collected during validation
//! for annotation in evaluation.iter_annotations() {
//!     println!("Annotation at {}: {:?}",
//!         annotation.instance_location,
//!         annotation.annotations
//!     );
//! }
//!
//! // Iterate over errors (if any)
//! for error in evaluation.iter_errors() {
//!     println!("Error: {}", error.error);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The structured output formats are particularly useful for:
//! - **Debugging**: Understanding exactly which schema keywords matched or failed
//! - **User feedback**: Providing detailed, actionable error messages
//! - **Annotations**: Collecting metadata produced by successful validation
//! - **Tooling**: Building development tools that work with JSON Schema
//!
//! For example, validating `["hello", "oops"]` against a schema with both `prefixItems` and
//! `items` produces list output similar to:
//!
//! ```json
//! {
//!   "valid": false,
//!   "details": [
//!     {"valid": false, "evaluationPath": "", "schemaLocation": "", "instanceLocation": ""},
//!     {
//!       "valid": false,
//!       "evaluationPath": "/items",
//!       "schemaLocation": "/items",
//!       "instanceLocation": "",
//!       "droppedAnnotations": true
//!     },
//!     {
//!       "valid": false,
//!       "evaluationPath": "/items",
//!       "schemaLocation": "/items",
//!       "instanceLocation": "/1"
//!     },
//!     {
//!       "valid": false,
//!       "evaluationPath": "/items/type",
//!       "schemaLocation": "/items/type",
//!       "instanceLocation": "/1",
//!       "errors": {"type": "\"oops\" is not of type \"integer\""}
//!     },
//!     {"valid": true, "evaluationPath": "/prefixItems", "schemaLocation": "/prefixItems", "instanceLocation": "", "annotations": 0}
//!   ]
//! }
//! ```
//!
//! ## Output Formats
//!
//! ### Flag Format
//!
//! The simplest format, containing only a boolean validity indicator:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use json_codec::json;
//! # let schema = json!({"type": "string"});
//! # let validator = jsonschema::validator_for(&schema)?;
//! let evaluation = validator.evaluate(&json!("hello"));
//! let flag = evaluation.flag();
//!
//! let output = json_codec::to_value(flag)?;
//! // Output: {"valid": true}
//! # Ok(())
//! # }
//! ```
//!
//! ### List Format
//!
//! A flat list of all evaluation units, where each unit describes a validation step:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use json_codec::json;
//! let schema = json!({
//!     "allOf": [
//!         {"type": "number"},
//!         {"minimum": 0}
//!     ]
//! });
//! let validator = jsonschema::validator_for(&schema)?;
//! let evaluation = validator.evaluate(&json!(42));
//!
//! let list = evaluation.list();
//! let output = json_codec::to_value(list)?;
//! // Output includes all evaluation steps in a flat array
//! # Ok(())
//! # }
//! ```
//!
//! ### Hierarchical Format
//!
//! A nested tree structure that mirrors the schema's logical structure:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use json_codec::json;
//! let schema = json!({
//!     "allOf": [
//!         {"type": "number"},
//!         {"minimum": 0}
//!     ]
//! });
//! let validator = jsonschema::validator_for(&schema)?;
//! let evaluation = validator.evaluate(&json!(42));
//!
//! let hierarchical = evaluation.hierarchical();
//! let output = json_codec::to_value(hierarchical)?;
//! // Output has nested "details" arrays for sub-schema evaluations
//! # Ok(())
//! # }
//! ```
//!
//! # Meta-Schema Validation
//!
//! The crate provides functionality to validate JSON Schema documents themselves against their meta-schemas.
//! This ensures your schema documents are valid according to the JSON Schema specification.
//!
//! ```rust
//! use json_codec::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": {"type": "string"},
//!         "age": {"type": "integer", "minimum": 0}
//!     }
//! });
//!
//! // Validate schema with automatic draft detection
//! assert!(jsonschema::meta::is_valid(&schema));
//! assert!(jsonschema::meta::validate(&schema).is_ok());
//!
//! // Invalid schema example
//! let invalid_schema = json!({
//!     "type": "invalid_type",  // must be one of the valid JSON Schema types
//!     "minimum": "not_a_number"
//! });
//! assert!(!jsonschema::meta::is_valid(&invalid_schema));
//! assert!(jsonschema::meta::validate(&invalid_schema).is_err());
//! ```
//!
//! # Configuration
//!
//! `jsonschema` provides several ways to configure and use JSON Schema validation.
//!
//! ## Draft-specific Modules
//!
//! The library offers modules for specific JSON Schema draft versions:
//!
//! - [`draft4`]
//! - [`draft6`]
//! - [`draft7`]
//! - [`draft201909`]
//! - [`draft202012`]
//!
//! Each module provides:
//! - A `new` function to create a validator
//! - An `is_valid` function for validation with a boolean result
//! - An `validate` function for getting the first validation error
//! - An `options` function to create a draft-specific configuration builder
//! - A `meta` module for draft-specific meta-schema validation
//!
//! Here's how you can explicitly use a specific draft version:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"type": "string"});
//!
//! // Instance validation
//! let validator = jsonschema::draft7::new(&schema)?;
//! assert!(validator.is_valid(&json!("Hello")));
//!
//! // Meta-schema validation
//! assert!(jsonschema::draft7::meta::is_valid(&schema));
//! # Ok(())
//! # }
//! ```
//!
//! You can also use the convenience [`is_valid`] and [`validate`] functions:
//!
//! ```rust
//! use json_codec::json;
//!
//! let schema = json!({"type": "number", "minimum": 0});
//! let instance = json!(42);
//!
//! assert!(jsonschema::draft202012::is_valid(&schema, &instance));
//! assert!(jsonschema::draft202012::validate(&schema, &instance).is_ok());
//! ```
//!
//! For more advanced configuration, you can use the draft-specific `options` function:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"type": "string", "format": "ends-with-42"});
//! let validator = jsonschema::draft202012::options()
//!     .with_format("ends-with-42", |s| s.ends_with("42"))
//!     .should_validate_formats(true)
//!     .build(&schema)?;
//!
//! assert!(validator.is_valid(&json!("Hello 42")));
//! assert!(!validator.is_valid(&json!("No!")));
//! # Ok(())
//! # }
//! ```
//!
//! ## General Configuration
//!
//! For configuration options that are not draft-specific, `jsonschema` provides a builder via `jsonschema::options()`.
//!
//! Here's an example:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"type": "string"});
//! let validator = jsonschema::options()
//!     // Add configuration options here
//!     .build(&schema)?;
//!
//! assert!(validator.is_valid(&json!("Hello")));
//! # Ok(())
//! # }
//! ```
//!
//! For a complete list of configuration options and their usage, please refer to the [`ValidationOptions`] struct.
//!
//! ## Automatic Draft Detection
//!
//! If you don't need to specify a particular draft version, you can use `jsonschema::validator_for`
//! which automatically detects the appropriate draft:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"$schema": "http://json-schema.org/draft-07/schema#", "type": "string"});
//! let validator = jsonschema::validator_for(&schema)?;
//!
//! assert!(validator.is_valid(&json!("Hello")));
//! # Ok(())
//! # }
//! ```
//!
//! # External References
//!
//! By default, `jsonschema` resolves HTTP references using `reqwest` and file references from the local file system.
//! Both blocking and non-blocking retrieval is supported during validator construction. Note that the validation
//! itself is always synchronous as it operates on in-memory data only.
//!
//! ```rust
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"$schema": "http://json-schema.org/draft-07/schema#", "type": "string"});
//!
//! // Building a validator with blocking retrieval (default)
//! let validator = jsonschema::validator_for(&schema)?;
//!
//! // Building a validator with non-blocking retrieval (requires `resolve-async` feature)
//! # #[cfg(feature = "resolve-async")]
//! let validator = jsonschema::async_validator_for(&schema).await?;
//!
//! // Validation is always synchronous
//! assert!(validator.is_valid(&json!("Hello")));
//! # Ok(())
//! # }
//! ```
//!
//! To enable HTTPS support, add the `rustls-tls` feature to `reqwest` in your `Cargo.toml`:
//!
//! ```toml
//! reqwest = { version = "*", features = ["rustls-tls"] }
//! ```
//!
//! You can disable the default behavior using crate features:
//!
//! - Disable HTTP resolving: `default-features = false, features = ["resolve-file"]`
//! - Disable file resolving: `default-features = false, features = ["resolve-http", "tls-aws-lc-rs"]`
//! - Enable async resolution: `features = ["resolve-async"]`
//! - Disable all resolving: `default-features = false`
//!
//! ## Custom retrievers
//!
//! A retriever is for documents fetched on demand — from a database, an embedded asset, a cache.
//! When the set of documents is known up front, put them in a [`Registry`] instead; the example
//! below is a retriever only because it has to be one to demonstrate the trait.
//!
//! You can implement custom retrievers for both blocking and non-blocking retrieval:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::{collections::HashMap, sync::Arc};
//! use jsonschema::{Retrieve, Uri};
//! use json_codec::{json, Value};
//!
//! struct InMemoryRetriever {
//!     schemas: HashMap<String, Value>,
//! }
//!
//! impl Retrieve for InMemoryRetriever {
//!
//!    fn retrieve(
//!        &self,
//!        uri: &Uri<String>,
//!    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
//!         self.schemas
//!             .get(uri.as_str())
//!             .cloned()
//!             .ok_or_else(|| format!("Schema not found: {uri}").into())
//!     }
//! }
//!
//! let mut schemas = HashMap::new();
//! schemas.insert(
//!     "https://example.com/person.json".to_string(),
//!     json!({
//!         "type": "object",
//!         "properties": {
//!             "name": { "type": "string" },
//!             "age": { "type": "integer" }
//!         },
//!         "required": ["name", "age"]
//!     }),
//! );
//!
//! let retriever = InMemoryRetriever { schemas };
//!
//! let schema = json!({
//!     "$ref": "https://example.com/person.json"
//! });
//!
//! let validator = jsonschema::options()
//!     .with_retriever(retriever)
//!     .build(&schema)?;
//!
//! assert!(validator.is_valid(&json!({
//!     "name": "Alice",
//!     "age": 30
//! })));
//!
//! assert!(!validator.is_valid(&json!({
//!     "name": "Bob"
//! })));
//! #    Ok(())
//! # }
//! ```
//!
//! And non-blocking version with the `resolve-async` feature enabled:
//!
//! ```rust,no_run
//! # #[cfg(all(
//! #     feature = "resolve-async",
//! #     any(
//! #         not(target_arch = "wasm32"),
//! #         all(target_arch = "wasm32", any(target_os = "unknown", target_os = "none")),
//! #     ),
//! # ))]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use jsonschema::{AsyncRetrieve, Registry, Resource, Uri};
//! use json_codec::{Value, json};
//!
//! struct HttpRetriever;
//!
//! #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
//! #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
//! impl AsyncRetrieve for HttpRetriever {
//!     async fn retrieve(
//!         &self,
//!         uri: &Uri<String>,
//!     ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
//!         reqwest::get(uri.as_str())
//!             .await?
//!             .json()
//!             .await
//!             .map_err(Into::into)
//!     }
//! }
//!
//! // Then use it to build a validator
//! let validator = jsonschema::async_options()
//!     .with_retriever(HttpRetriever)
//!     .build(&json!({"$ref": "https://example.com/user.json"}))
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! On `wasm32` targets, use `async_trait::async_trait(?Send)` so your retriever can rely on `Rc`, `JsFuture`, or other non-thread-safe types.
//!
//! ## Validating against schema definitions
//!
//! When working with large schemas containing multiple definitions (e.g., Open API schemas, DAP schemas),
//! you may want to validate data against a specific definition rather than the entire schema. This can be
//! achieved by registering the root schema as a resource and creating a wrapper schema that references
//! the target definition:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//! use jsonschema::{Registry, Resource};
//!
//! // Root schema with multiple definitions
//! let root_schema = json!({
//!     "$id": "https://example.com/root",
//!     "definitions": {
//!         "User": {
//!             "type": "object",
//!             "properties": {
//!                 "name": {"type": "string"},
//!                 "age": {"type": "integer", "minimum": 0}
//!             },
//!             "required": ["name"]
//!         },
//!         "Product": {
//!             "type": "object",
//!             "properties": {
//!                 "id": {"type": "integer"},
//!                 "title": {"type": "string"}
//!             },
//!             "required": ["id", "title"]
//!         }
//!     }
//! });
//!
//! // Create a schema that references the specific definition you want to validate against
//! let user_schema = json!({"$ref": "https://example.com/root#/definitions/User"});
//!
//! let registry = Registry::new()
//!     .add("https://example.com/root", root_schema)?
//!     .prepare()?;
//!
//! // Build validator for the specific definition via the shared prepared registry
//! let validator = jsonschema::options()
//!     .with_registry(&registry)
//!     .build(&user_schema)?;
//!
//! // Now validate data against just the User definition
//! assert!(validator.is_valid(&json!({"name": "Alice", "age": 30})));
//! assert!(!validator.is_valid(&json!({"age": 25})));  // Missing required "name"
//! # Ok(())
//! # }
//! ```
//!
//! This pattern is particularly useful when:
//! - Working with API schemas that define multiple request/response types
//! - Validating configuration snippets against specific sections of a larger schema
//! - Testing individual schema components in isolation
//!
//! ## Offline validation
//!
//! [`ValidationOptions::offline`] disables retrieval, so a reference that is not already in the
//! registry fails at construction instead of being fetched:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use json_codec::json;
//!
//! let schema = json!({"$ref": "https://example.com/schema.json"});
//! assert!(jsonschema::options().offline().build(&schema).is_err());
//! # Ok(())
//! # }
//! ```
//!
//! ## Bundling and dereferencing
//!
//! [`bundle`] embeds every referenced resource into the schema under `$defs` (`definitions` for
//! Draft 4/6/7), keeping each `$id` and leaving `$ref` values unchanged. The result is a Compound
//! Schema Document that needs no retrieval.
//!
//! [`dereference`] instead replaces each `$ref` with the schema it points to, for consumers that
//! do not resolve references. Circular references are left in place.
//!
//! # Regular Expression Configuration
//!
//! The `jsonschema` crate allows configuring the regular expression engine used for validating
//! keywords like `pattern` or `patternProperties`.
//!
//! By default, the crate uses [`fancy-regex`](https://docs.rs/fancy-regex), which supports advanced
//! regular expression features such as lookaround and backreferences.
//!
//! The primary motivation for switching to the `regex` engine is security and performance:
//! it guarantees linear-time matching, preventing potential Denial of Service attacks from malicious patterns
//! in user-provided schemas while offering better performance with a smaller feature set.
//!
//! You can configure the engine at **runtime** using the [`PatternOptions`] API:
//!
//! ### Example: Configure `fancy-regex` with Backtracking Limit
//!
//! ```rust
//! use json_codec::json;
//! use jsonschema::PatternOptions;
//!
//! let schema = json!({
//!     "type": "string",
//!     "pattern": "^(a+)+$"
//! });
//!
//! let validator = jsonschema::options()
//!     .with_pattern_options(
//!         PatternOptions::fancy_regex()
//!             .backtrack_limit(10_000)
//!     )
//!     .build(&schema)
//!     .expect("A valid schema");
//! ```
//!
//! ### Example: Use the `regex` Engine Instead
//!
//! ```rust
//! use json_codec::json;
//! use jsonschema::PatternOptions;
//!
//! let schema = json!({
//!     "type": "string",
//!     "pattern": "^a+$"
//! });
//!
//! let validator = jsonschema::options()
//!     .with_pattern_options(PatternOptions::regex())
//!     .build(&schema)
//!     .expect("A valid schema");
//! ```
//!
//! ### Notes
//!
//! - If neither engine is explicitly set, `fancy-regex` is used by default.
//! - Regular expressions that rely on advanced features like `(?<=...)` (lookbehind) or backreferences (`\1`) will fail with the `regex` engine.
//!
//! # Custom Keywords
//!
//! `jsonschema` allows you to extend its functionality by implementing custom validation logic through custom keywords.
//! This feature is particularly useful when you need to validate against domain-specific rules that aren't covered by the standard JSON Schema keywords.
//! Keywords are generic over the instance representation ([`json`] module).
//!
//! To implement a custom keyword, you need to:
//! 1. Create a struct that implements the [`Keyword`] trait
//! 2. Create a factory function or closure that produces instances of your custom keyword
//! 3. Register the custom keyword with the [`Validator`] instance using the [`ValidationOptions::with_keyword`] method
//!
//! Here's a complete example:
//!
//! ```rust
//! use jsonschema::{paths::Location, Keyword, ValidationError};
//! use json_codec::{json, Map, Value};
//!
//! struct EvenNumberValidator;
//!
//! impl<'i> Keyword<'i> for EvenNumberValidator {
//!     fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
//!         if let Some(n) = instance.as_u64() {
//!             if n % 2 == 0 {
//!                 return Ok(());
//!             }
//!         }
//!         Err(ValidationError::custom("value must be an even integer"))
//!     }
//!
//!     fn is_valid(&self, instance: &'i Value) -> bool {
//!         instance.as_u64().map_or(false, |n| n % 2 == 0)
//!     }
//! }
//!
//! fn even_number_factory<'a>(
//!     _parent: &'a Map<String, Value>,
//!     value: &'a Value,
//!     _path: Location,
//! ) -> Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>> {
//!     if value.as_bool() == Some(true) {
//!         Ok(Box::new(EvenNumberValidator))
//!     } else {
//!         Err(ValidationError::schema("The 'even-number' keyword must be set to true"))
//!     }
//! }
//!
//! let schema = json!({"even-number": true, "type": "integer"});
//! let validator = jsonschema::options()
//!     .with_keyword("even-number", even_number_factory)
//!     .build(&schema)
//!     .expect("Invalid schema");
//!
//! assert!(validator.is_valid(&json!(2)));
//! assert!(!validator.is_valid(&json!(3)));
//! assert!(!validator.is_valid(&json!("not a number")));
//! ```
//!
//! In this example, we've created a custom `even-number` keyword that validates whether a number is even.
//! The `EvenNumberValidator` implements the actual validation logic, while the `even_number_factory`
//! creates instances of the validator and allows for additional configuration based on the keyword's value in the schema.
//!
//! You can also use a closure instead of a factory function for simpler cases:
//!
//! ```rust
//! # use jsonschema::{paths::Location, Keyword, ValidationError};
//! # use json_codec::{json, Map, Value};
//! #
//! # struct EvenNumberValidator;
//! #
//! # impl<'i> Keyword<'i> for EvenNumberValidator {
//! #     fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
//! #         Ok(())
//! #     }
//! #
//! #     fn is_valid(&self, instance: &'i Value) -> bool {
//! #         true
//! #     }
//! # }
//! let schema = json!({"even-number": true, "type": "integer"});
//! let validator = jsonschema::options()
//!     .with_keyword("even-number", |_, _, _| {
//!         Ok(Box::new(EvenNumberValidator))
//!     })
//!     .build(&schema)
//!     .expect("Invalid schema");
//! ```
//!
//! # Custom Formats
//!
//! JSON Schema allows for format validation through the `format` keyword. While `jsonschema`
//! provides built-in validators for standard formats, you can also define custom format validators
//! for domain-specific string formats.
//!
//! To implement a custom format validator:
//!
//! 1. Define a function or a closure that takes a `&str` and returns a `bool`.
//! 2. Register the function with `jsonschema::options().with_format()`.
//!
//! ```rust
//! use json_codec::json;
//!
//! // Step 1: Define the custom format validator function
//! fn ends_with_42(s: &str) -> bool {
//!     s.ends_with("42!")
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Step 2: Create a schema using the custom format
//! let schema = json!({
//!     "type": "string",
//!     "format": "ends-with-42"
//! });
//!
//! // Step 3: Build the validator with the custom format
//! let validator = jsonschema::options()
//!     .with_format("ends-with-42", ends_with_42)
//!     .with_format("ends-with-43", |s| s.ends_with("43!"))
//!     .should_validate_formats(true)
//!     .build(&schema)?;
//!
//! // Step 4: Validate instances
//! assert!(validator.is_valid(&json!("Hello42!")));
//! assert!(!validator.is_valid(&json!("Hello43!")));
//! assert!(!validator.is_valid(&json!(42))); // Not a string
//! #    Ok(())
//! # }
//! ```
//!
//! ### Notes on Custom Format Validators
//!
//! - Custom format validators are only called for string instances.
//! - In newer drafts, `format` is purely an annotation and won’t do any checking unless you
//!   opt in by calling `.should_validate_formats(true)` on your options builder. If you omit
//!   it, all `format` keywords are ignored at validation time.
//!
//! # Arbitrary Precision Numbers
//!
//! Enable the `arbitrary-precision` feature for exact validation of numbers beyond standard numeric ranges:
//!
//! ```toml
//! jsonschema = { version = "x.y.z", features = ["arbitrary-precision"] }
//! ```
//!
//! This provides:
//! - Arbitrarily large integers (e.g., `18446744073709551616`)
//! - Exact decimal precision without `f64` rounding (e.g., `0.1`, `0.3`)
//!
//! **Important**: Precision is only preserved when parsing JSON from strings. Using Rust literals
//! or the `json!()` macro converts numbers to `f64`, losing precision.
//!
//! ```rust
//! # use jsonschema::Validator;
//! // Precision preserved - parsed from JSON string
//! let schema = json_codec::from_str(r#"{"minimum": 0.1}"#)?;
//! let instance = json_codec::from_str("0.3")?;
//! let validator = Validator::new(&schema)?;
//! assert!(validator.is_valid(&instance));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Custom JSON Representations
//!
//! Validators can accept instances in any in-memory JSON representation, not just
//! `json_codec::Value`. Implement the traits in the [`json`] module for your representation and
//! build validators with [`options_for`]:
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # type MyJson = jsonschema::json::SerdeJson;
//! # let schema = json_codec::json!({"type": "string"});
//! # let my_instance = &json_codec::json!("data");
//! let validator = jsonschema::options_for::<MyJson>().build(&schema)?;
//! assert!(validator.is_valid(my_instance));
//! # Ok(())
//! # }
//! ```
//!
//! See the [`json`] module for the full trait contract, a worked example, and how to verify an
//! implementation. The Python bindings use this mechanism to validate Python objects directly.
//!
//! # WebAssembly support
//!
//! `jsonschema` supports WebAssembly with different capabilities based on the target platform:
//!
//! ## Browser/JavaScript (`wasm32-unknown-unknown`)
//!
//! When targeting browser or JavaScript environments, external reference resolution is not
//! supported by default due to platform limitations:
//!    - No filesystem access (`resolve-file` feature is not available)
//!    - No synchronous HTTP requests (`resolve-http` feature is not available)
//!
//! To use `jsonschema` in these environments, disable default features:
//!
//! ```toml
//! jsonschema = { version = "x.y.z", default-features = false }
//! ```
//!
//! Note: Attempting to compile with `resolve-http` or `resolve-file` features on
//! `wasm32-unknown-unknown` will result in a compile error.
//!
//! For external references in browser environments, implement a custom retriever that uses
//! browser APIs (like `fetch`). See the [External References](#external-references) section.
//!
//! ## WASI (`wasm32-wasip1` / `wasm32-wasip2`)
//!
//! WASI environments (preview 1 and preview 2) can compile schemas and run validators, but the bundled
//! HTTP retriever depends on `reqwest`’s blocking client, which isn't available on these targets. Use
//! file access and custom retrievers instead.
//!
//! **Supported:**
//! - Blocking file resolution (`resolve-file` feature)
//! - Custom blocking retrievers (including wrapping async operations)
//! - Custom async retrievers via the `resolve-async` feature (for example, `jsonschema::async_options`
//!   together with your own async runtime)
//!
//! **Not Supported:**
//! - The bundled HTTP retriever (depends on `reqwest`’s blocking client)
//!
//! ```toml
//! jsonschema = { version = "x.y.z", default-features = false, features = ["resolve-file"] }
//! ```
//!
//! **Workaround for HTTP:** Implement a custom blocking or async [`Retrieve`] that uses your preferred
//! HTTP client, and enable `resolve-async` if you want to build validators through `async_options()`
//! on WASI.

#[cfg(all(
    target_arch = "wasm32",
    target_os = "unknown",
    any(feature = "resolve-file", feature = "resolve-http")
))]
compile_error!(
    "Features 'resolve-http' and 'resolve-file' are not supported on wasm32-unknown-unknown"
);
#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "resolve-http",
    not(any(feature = "tls-aws-lc-rs", feature = "tls-ring"))
))]
compile_error!(
    "Feature `resolve-http` requires a TLS provider: enable `tls-aws-lc-rs` \
(default) or `tls-ring`."
);

pub(crate) mod bundler;
pub mod canonical;
pub(crate) mod compiler;
mod content_encoding;
mod content_media_type;
pub(crate) mod dereferencer;
pub mod error;
mod evaluation;
pub(crate) use jsonschema_value::{
    cmp, numeric, unique, Array, Json, LazyInstance, Node, NodeIdentity, Object, JsonValueBackend,
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
/// use jsonschema::{
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
/// let validator = jsonschema::options_for::<ToyJson>().build(&schema)?;
/// # let _ = format!("{:?}", jsonschema::options_for::<ToyJson>());
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
/// implementations with [`ValidationOptions::with_keyword`](crate::ValidationOptions::with_keyword).
pub mod json {
    #[cfg(feature = "conformance")]
    pub use jsonschema_value::conformance;
    pub use jsonschema_value::{
        cmp, unique, Array, Json, JsonNumber, Node, NodeIdentity, Object, JsonValueBackend,
    };
    #[cfg(feature = "pyo3")]
    pub use jsonschema_value::{
        inspect, narrow_array, narrow_object, object_values, stock_enum_value, Pyo3,
    };
    #[cfg(feature = "jsonb")]
    pub use jsonschema_value::{jsonb, Jsonb, JsonbNode};
    #[cfg(feature = "magnus")]
    pub use jsonschema_value::{
        magnus_child, magnus_invalidate_members_cache, magnus_is_object, magnus_object_values,
        magnus_probe_root, magnus_string_node, magnus_take_pending_error, Magnus,
        MagnusPendingErrorScope, PendingError, RbNode,
    };
}
mod http;
mod keywords;
#[cfg(all(feature = "macros", not(target_family = "wasm")))]
mod meta_codegen;
mod node;
mod options;
pub mod output;
pub mod paths;
pub(crate) mod properties;
pub(crate) mod regex;
mod retriever;
pub mod types {
    pub use jsonschema_value::types::{JsonType, JsonTypeSet, JsonTypeSetIterator};
}
mod validator;

pub use canonical::CanonicalizationError;
pub use error::{
    ErrorIterator, MaskedValidationError, ValidationError, ValidationErrorParts, ValidationErrors,
};
pub use evaluation::{
    AnnotationEntry, ErrorEntry, Evaluation, FlagOutput, HierarchicalOutput, ListOutput,
};
pub use http::HttpOptions;
#[doc(inline)]
#[cfg(feature = "macros")]
pub use jsonschema_macros::validator;
pub use keywords::custom::Keyword;
pub use options::{EmailOptions, FancyRegex, PatternOptions, Regex, ValidationOptions};
pub use referencing::{
    uri, Draft, Error as ReferencingError, Registry, RegistryBuilder, Resource, Retrieve, Uri,
};
#[cfg(all(feature = "resolve-http", not(target_arch = "wasm32")))]
pub use retriever::{HttpRetriever, HttpRetrieverError};
pub use types::{JsonType, JsonTypeSet, JsonTypeSetIterator};
pub use validator::{ValidationContext, Validator, ValidatorMap};

#[cfg(feature = "resolve-async")]
pub use referencing::AsyncRetrieve;
#[cfg(all(
    feature = "resolve-http",
    feature = "resolve-async",
    not(target_arch = "wasm32")
))]
pub use retriever::AsyncHttpRetriever;

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
/// assert!(jsonschema::is_valid(&schema, &instance));
/// ```
///
/// # Panics
///
/// This function panics if an invalid schema is passed.
///
/// This function **must not** be called from within an async runtime if the schema contains
/// external references that require network requests, or it will panic when attempting to block.
/// Use `async_validator_for` for async contexts, or run this in a separate blocking thread
/// via `tokio::task::spawn_blocking`.
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
/// assert!(jsonschema::validate(&schema, &instance).is_ok());
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
/// via `tokio::task::spawn_blocking`.
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
/// let evaluation = jsonschema::evaluate(&schema, &instance);
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
/// via `tokio::task::spawn_blocking`.
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
/// let validator = jsonschema::validator_for(&schema)?;
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
/// via `tokio::task::spawn_blocking`.
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
/// let map = jsonschema::validator_map_for(&schema)?;
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
/// via `tokio::task::spawn_blocking`.
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
/// via `tokio::task::spawn_blocking`.
///
/// # Examples
///
/// ```rust
/// use json_codec::json;
///
/// let schema = json!({"type": "string"});
/// let bundled = jsonschema::bundle(&schema).expect("bundling failed");
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
/// let result = jsonschema::dereference(&schema).expect("dereference failed");
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
/// let validator = jsonschema::async_validator_for(&schema).await?;
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
/// let map = jsonschema::async_validator_map_for(&schema).await?;
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
/// use jsonschema::Draft;
///
/// let schema = json!({"type": "string"});
/// let validator = jsonschema::options()
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
/// let validator = jsonschema::options()
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
/// let validator = jsonschema::async_options()
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
/// use jsonschema::{Draft, AsyncRetrieve, Uri};
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
/// let validator = jsonschema::async_options()
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
    use crate::{error::ValidationError, Draft, Json, Node, Object, Registry, Validator};
    use ahash::AHashSet;
    use referencing::Retrieve;
    use json_codec::Value;
    use std::{
        any::{Any, TypeId},
        borrow::Cow,
        sync::{OnceLock, RwLock},
    };

    pub use validator_handle::MetaValidator;

    /// Meta-schema validators that read a schema held as a Python object.
    ///
    /// The bundled drafts are compiled in, so a schema already in Python form is checked without
    /// being converted to [`json_codec::Value`]. A `$schema` outside them is reached through
    /// [`is_valid_for`] / [`validate_for`] instead.
    ///
    /// Needs the `macros` feature; without it, use [`is_valid_for`] / [`validate_for`] with
    /// [`json::Pyo3`](crate::json::Pyo3).
    #[cfg(all(feature = "macros", feature = "pyo3", not(target_family = "wasm")))]
    pub mod pyo3 {
        use crate::Draft;
        use ::pyo3::{types::PyAny, Borrowed};

        pub use crate::meta_codegen::pyo3::{is_valid_fn, validate_fn, IsValidFn, ValidateFn};

        /// The draft whose meta-schema `schema` names in `$schema`.
        ///
        /// `Draft::Unknown` means a URI outside the bundled drafts, which the functions above
        /// cannot answer for.
        #[must_use]
        pub fn draft_of(schema: Borrowed<'_, '_, PyAny>) -> Draft {
            super::meta_cache::<crate::json::Pyo3>().draft_of(&schema)
        }
    }

    /// Meta-schema validators that read a schema held as a Ruby object.
    ///
    /// The bundled drafts are compiled in, so a schema already in Ruby form is checked without
    /// being converted to [`json_codec::Value`]. A `$schema` outside them is reached through
    /// [`is_valid_for`] / [`validate_for`] instead.
    ///
    /// Needs the `macros` feature; without it, use [`is_valid_for`] / [`validate_for`] with
    /// [`json::Magnus`](crate::json::Magnus).
    #[cfg(all(feature = "macros", feature = "magnus", not(target_family = "wasm")))]
    pub mod magnus {
        use crate::{json::RbNode, Draft};

        pub use crate::meta_codegen::magnus::{is_valid_fn, validate_fn, IsValidFn, ValidateFn};

        /// The draft whose meta-schema `schema` names in `$schema`.
        ///
        /// `Draft::Unknown` means a URI outside the bundled drafts, which the functions above
        /// cannot answer for.
        #[must_use]
        pub fn draft_of(schema: RbNode<'_>) -> Draft {
            super::meta_cache::<crate::json::Magnus>().draft_of(&schema)
        }
    }

    /// Create a meta-validation options builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use json_codec::json;
    /// use jsonschema::{Registry, Resource};
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
    /// assert!(jsonschema::meta::options()
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
        /// use jsonschema::{Registry, Resource};
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
        /// let options = jsonschema::meta::options()
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
        use crate::{ValidationError, Validator};
        use json_codec::Value;
        use std::{marker::PhantomData, ops::Deref};

        /// Handle to a draft-specific meta-schema [`Validator`]. Borrows cached validators on native
        /// targets and owns validators on `wasm32`.
        pub struct MetaValidator<'a>(MetaValidatorInner<'a>);

        // Native builds can hand out references to cached validators or own dynamic ones,
        // while wasm targets need owned instances because the validator type does not implement `Sync` there.
        // Under `macros`, native builds dispatch to compile-time generated validators; the cached
        // runtime validator is resolved lazily by draft only for the `evaluate` path (`AsRef`).
        enum MetaValidatorInner<'a> {
            #[cfg(all(not(feature = "macros"), not(target_family = "wasm")))]
            Borrowed(&'a Validator),
            Owned(Box<Validator>, PhantomData<&'a Validator>),
            #[cfg(all(feature = "macros", not(target_family = "wasm")))]
            Generated {
                draft: crate::Draft,
                is_valid: fn(&Value) -> bool,
                validate: crate::meta_codegen::ValidateFn,
                iter_errors: crate::meta_codegen::IterErrorsFn,
            },
        }

        // `borrowed` is the only method using `'a`; it is absent on wasm and under `macros`.
        #[cfg_attr(
            any(target_family = "wasm", feature = "macros"),
            allow(clippy::elidable_lifetime_names)
        )]
        impl<'a> MetaValidator<'a> {
            #[cfg(all(not(feature = "macros"), not(target_family = "wasm")))]
            pub(crate) fn borrowed(validator: &'a Validator) -> Self {
                Self(MetaValidatorInner::Borrowed(validator))
            }

            pub(crate) fn owned(validator: Validator) -> Self {
                Self(MetaValidatorInner::Owned(Box::new(validator), PhantomData))
            }

            #[cfg(all(feature = "macros", not(target_family = "wasm")))]
            pub(crate) fn generated(draft: crate::Draft) -> Self {
                Self(MetaValidatorInner::Generated {
                    draft,
                    is_valid: crate::meta_codegen::is_valid_fn(draft),
                    validate: crate::meta_codegen::validate_fn(draft),
                    iter_errors: crate::meta_codegen::iter_errors_fn(draft),
                })
            }

            /// Validate `schema` against the meta-schema, returning `true` if valid.
            #[must_use]
            pub fn is_valid(&self, schema: &Value) -> bool {
                match &self.0 {
                    #[cfg(all(feature = "macros", not(target_family = "wasm")))]
                    MetaValidatorInner::Generated { is_valid, .. } => is_valid(schema),
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
                    #[cfg(all(feature = "macros", not(target_family = "wasm")))]
                    MetaValidatorInner::Generated { validate, .. } => validate(schema),
                    _ => self.as_ref().validate(schema),
                }
            }

            /// Validate `schema` against the meta-schema, yielding every error.
            #[must_use]
            pub fn iter_errors<'i>(&'i self, schema: &'i Value) -> crate::ErrorIterator<'i> {
                match &self.0 {
                    #[cfg(all(feature = "macros", not(target_family = "wasm")))]
                    MetaValidatorInner::Generated { iter_errors, .. } => iter_errors(schema),
                    _ => self.as_ref().iter_errors(schema),
                }
            }
        }

        impl AsRef<Validator> for MetaValidator<'_> {
            fn as_ref(&self) -> &Validator {
                match &self.0 {
                    #[cfg(all(not(feature = "macros"), not(target_family = "wasm")))]
                    MetaValidatorInner::Borrowed(validator) => validator,
                    MetaValidatorInner::Owned(validator, _) => validator,
                    // `evaluate` has no generated counterpart; fall back to the runtime validator.
                    #[cfg(all(feature = "macros", not(target_family = "wasm")))]
                    MetaValidatorInner::Generated { draft, .. } => {
                        crate::meta::validators::runtime_validator_for_draft(*draft)
                    }
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
        use crate::Validator;
        #[cfg(not(target_family = "wasm"))]
        use std::sync::LazyLock;

        fn build_validator(schema: &json_codec::Value) -> Validator {
            crate::options()
                .without_schema_validation()
                .build(schema)
                .expect("Meta-schema should be valid")
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT4_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&referencing::meta::DRAFT4));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft4_meta_validator() -> Validator {
            build_validator(&referencing::meta::DRAFT4)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT6_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&referencing::meta::DRAFT6));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft6_meta_validator() -> Validator {
            build_validator(&referencing::meta::DRAFT6)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT7_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&referencing::meta::DRAFT7));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft7_meta_validator() -> Validator {
            build_validator(&referencing::meta::DRAFT7)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT201909_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&referencing::meta::DRAFT201909));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft201909_meta_validator() -> Validator {
            build_validator(&referencing::meta::DRAFT201909)
        }

        #[cfg(not(target_family = "wasm"))]
        pub(crate) static DRAFT202012_META_VALIDATOR: LazyLock<Validator> =
            LazyLock::new(|| build_validator(&referencing::meta::DRAFT202012));
        #[cfg(target_family = "wasm")]
        pub(crate) fn draft202012_meta_validator() -> Validator {
            build_validator(&referencing::meta::DRAFT202012)
        }

        // Backs the `evaluate`/`iter_errors` fallback for generated meta validators.
        #[cfg(all(feature = "macros", not(target_family = "wasm")))]
        pub(crate) fn runtime_validator_for_draft(draft: crate::Draft) -> &'static Validator {
            use crate::Draft;
            match draft {
                Draft::Draft4 => &DRAFT4_META_VALIDATOR,
                Draft::Draft6 => &DRAFT6_META_VALIDATOR,
                Draft::Draft7 => &DRAFT7_META_VALIDATOR,
                Draft::Draft201909 => &DRAFT201909_META_VALIDATOR,
                _ => &DRAFT202012_META_VALIDATOR,
            }
        }
    }

    pub(crate) fn validator_for_draft(draft: Draft) -> MetaValidator<'static> {
        #[cfg(all(feature = "macros", not(target_family = "wasm")))]
        {
            MetaValidator::generated(draft)
        }
        #[cfg(all(not(feature = "macros"), not(target_family = "wasm")))]
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
    /// assert!(jsonschema::meta::is_valid(&schema));
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
    /// [`meta::options().with_registry(...)`](crate::meta::options) to validate it.
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
    /// assert!(jsonschema::meta::validate(&schema).is_ok());
    ///
    /// // Invalid schema
    /// let invalid_schema = json!({
    ///     "type": "invalid_type"
    /// });
    /// assert!(jsonschema::meta::validate(&invalid_schema).is_err());
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
    /// Schema drafts. For custom meta-schemas, use [`meta::options().with_registry(...)`](crate::meta::options)
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
    /// let validator = jsonschema::meta::validator_for(&schema)
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
    /// use [`meta::options().with_registry(...)`](crate::meta::options).
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
        crate::options_for::<F>()
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
                Draft::Draft4 => (0, &referencing::meta::DRAFT4),
                Draft::Draft6 => (1, &referencing::meta::DRAFT6),
                Draft::Draft7 => (2, &referencing::meta::DRAFT7),
                Draft::Draft201909 => (3, &referencing::meta::DRAFT201909),
                // Draft202012, Unknown, or any future draft variants
                _ => (4, &referencing::meta::DRAFT202012),
            };
            self.validators[index].get_or_init(|| {
                crate::options_for::<F>()
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
                    let validator = crate::options()
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
                let validator = crate::options()
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
        let resolver = registry.resolver(referencing::uri::from_str(uri)?);
        let first_resolved = resolver.lookup("")?;
        let first_meta_schema = first_resolved.contents().clone();

        let draft = walk_meta_schema_chain(uri, |current_uri| {
            let resolver = registry.resolver(referencing::uri::from_str(current_uri)?);
            let resolved = resolver.lookup("")?;
            Ok(resolved.contents().clone())
        })?;

        Ok((first_meta_schema, draft))
    }

    fn resolve_meta_schema_chain(uri: &str) -> Result<(Value, Draft), ValidationError<'static>> {
        let retriever = crate::retriever::DefaultRetriever;
        let first_meta_uri = referencing::uri::from_str(uri)?;
        let first_meta_schema = retriever
            .retrieve(&first_meta_uri)
            .map_err(|e| referencing::Error::unretrievable(uri, e))?;

        let draft = walk_meta_schema_chain(uri, |current_uri| {
            let meta_uri = referencing::uri::from_str(current_uri)?;
            retriever
                .retrieve(&meta_uri)
                .map_err(|e| referencing::Error::unretrievable(current_uri, e))
        })?;

        Ok((first_meta_schema, draft))
    }

    pub(crate) fn walk_meta_schema_chain(
        start_uri: &str,
        mut fetch: impl FnMut(&str) -> Result<Value, referencing::Error>,
    ) -> Result<Draft, referencing::Error> {
        let mut visited = AHashSet::new();
        let mut current_uri = start_uri.to_string();

        loop {
            if !visited.insert(current_uri.clone()) {
                return Err(referencing::Error::circular_metaschema(current_uri));
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
/// assert!(jsonschema::draft4::is_valid(&schema, &instance));
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
    /// let validator = jsonschema::draft4::new(&schema)?;
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
    /// assert!(jsonschema::draft4::is_valid(&schema, &valid));
    /// assert!(!jsonschema::draft4::is_valid(&schema, &invalid));
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
    /// assert!(jsonschema::draft4::validate(&schema, &valid).is_ok());
    /// assert!(jsonschema::draft4::validate(&schema, &invalid).is_err());
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
    /// This function provides a shorthand for `jsonschema::options().with_draft(Draft::Draft4)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = jsonschema::draft4::options()
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
        crate::options().with_draft(Draft::Draft4)
    }

    /// Functionality for validating JSON Schema Draft 4 documents.
    pub mod meta {
        use crate::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 4 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::meta::validator_for_draft(super::Draft::Draft4)
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
        /// assert!(jsonschema::draft4::meta::is_valid(&schema));
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
        /// assert!(jsonschema::draft4::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(jsonschema::draft4::meta::validate(&invalid_schema).is_err());
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
/// assert!(jsonschema::draft6::is_valid(&schema, &instance));
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
    /// let validator = jsonschema::draft6::new(&schema)?;
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
    /// assert!(jsonschema::draft6::is_valid(&schema, &valid));
    /// assert!(!jsonschema::draft6::is_valid(&schema, &invalid));
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
    /// assert!(jsonschema::draft6::validate(&schema, &valid).is_ok());
    /// assert!(jsonschema::draft6::validate(&schema, &invalid).is_err());
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
    /// This function provides a shorthand for `jsonschema::options().with_draft(Draft::Draft6)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = jsonschema::draft6::options()
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
        crate::options().with_draft(Draft::Draft6)
    }

    /// Functionality for validating JSON Schema Draft 6 documents.
    pub mod meta {
        use crate::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 6 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::meta::validator_for_draft(super::Draft::Draft6)
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
        /// assert!(jsonschema::draft6::meta::is_valid(&schema));
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
        /// assert!(jsonschema::draft6::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(jsonschema::draft6::meta::validate(&invalid_schema).is_err());
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
/// assert!(jsonschema::draft7::is_valid(&schema, &instance));
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
    /// let validator = jsonschema::draft7::new(&schema)?;
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
    /// assert!(jsonschema::draft7::is_valid(&schema, &valid));
    /// assert!(!jsonschema::draft7::is_valid(&schema, &invalid));
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
    /// assert!(jsonschema::draft7::validate(&schema, &valid).is_ok());
    /// assert!(jsonschema::draft7::validate(&schema, &invalid).is_err());
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
    /// This function provides a shorthand for `jsonschema::options().with_draft(Draft::Draft7)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = jsonschema::draft7::options()
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
        crate::options().with_draft(Draft::Draft7)
    }

    /// Functionality for validating JSON Schema Draft 7 documents.
    pub mod meta {
        use crate::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 7 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::meta::validator_for_draft(super::Draft::Draft7)
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
        /// assert!(jsonschema::draft7::meta::is_valid(&schema));
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
        /// assert!(jsonschema::draft7::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(jsonschema::draft7::meta::validate(&invalid_schema).is_err());
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
/// assert!(jsonschema::draft201909::is_valid(&schema, &instance));
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
    /// let validator = jsonschema::draft201909::new(&schema)?;
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
    /// assert!(jsonschema::draft201909::is_valid(&schema, &valid));
    /// assert!(!jsonschema::draft201909::is_valid(&schema, &invalid));
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
    /// assert!(jsonschema::draft201909::validate(&schema, &valid).is_ok());
    /// assert!(jsonschema::draft201909::validate(&schema, &invalid).is_err());
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
    /// This function provides a shorthand for `jsonschema::options().with_draft(Draft::Draft201909)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = jsonschema::draft201909::options()
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
        crate::options().with_draft(Draft::Draft201909)
    }

    /// Functionality for validating JSON Schema Draft 2019-09 documents.
    pub mod meta {
        use crate::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 2019-09 meta-schema validator. Native targets borrow cached
        /// statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::meta::validator_for_draft(super::Draft::Draft201909)
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
        /// assert!(jsonschema::draft201909::meta::is_valid(&schema));
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
        /// assert!(jsonschema::draft201909::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(jsonschema::draft201909::meta::validate(&invalid_schema).is_err());
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
/// assert!(jsonschema::draft202012::is_valid(&schema, &instance));
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
    /// let validator = jsonschema::draft202012::new(&schema)?;
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
    /// assert!(jsonschema::draft202012::is_valid(&schema, &valid));
    /// assert!(!jsonschema::draft202012::is_valid(&schema, &invalid));
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
    /// assert!(jsonschema::draft202012::validate(&schema, &valid).is_ok());
    /// assert!(jsonschema::draft202012::validate(&schema, &invalid).is_err());
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
    /// This function provides a shorthand for `jsonschema::options().with_draft(Draft::Draft202012)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use json_codec::json;
    ///
    /// let schema = json!({"type": "string", "format": "ends-with-42"});
    /// let validator = jsonschema::draft202012::options()
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
        crate::options().with_draft(Draft::Draft202012)
    }

    /// Functionality for validating JSON Schema Draft 2020-12 documents.
    pub mod meta {
        use crate::{meta::MetaValidator, ValidationError};
        use json_codec::Value;

        /// Returns a handle to the Draft 2020-12 meta-schema validator. Native targets borrow
        /// cached statics while `wasm32` builds an owned validator.
        #[must_use]
        pub fn validator() -> MetaValidator<'static> {
            crate::meta::validator_for_draft(super::Draft::Draft202012)
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
        /// assert!(jsonschema::draft202012::meta::is_valid(&schema));
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
        /// assert!(jsonschema::draft202012::meta::validate(&schema).is_ok());
        ///
        /// // Invalid schema
        /// let invalid_schema = json!({
        ///     "type": "invalid_type"
        /// });
        /// assert!(jsonschema::draft202012::meta::validate(&invalid_schema).is_err());
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

#[cfg(feature = "macros")]
#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "pyo3")]
    pub mod pyo3 {
        pub use pyo3::{intern, types::PyString, Borrowed, Bound, PyAny, PyResult};
    }

    // Wraps a `backend = Pyo3` validator, so a build without the feature reports that alone.
    #[cfg(feature = "pyo3")]
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __pyo3_backend {
        ($($generated:tt)*) => { $($generated)* };
    }

    #[cfg(not(feature = "pyo3"))]
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __pyo3_backend {
        ($($generated:tt)*) => {
            ::core::compile_error!("`backend = Pyo3` needs the `pyo3` feature of `jsonschema`");
        };
    }

    pub use crate::__pyo3_backend as pyo3_backend;

    #[cfg(feature = "magnus")]
    pub mod magnus {
        pub use ::magnus::{rb_sys::AsRawValue, Error, Value};
    }

    // Wraps a `backend = Magnus` validator, so a build without the feature reports that alone.
    #[cfg(feature = "magnus")]
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __magnus_backend {
        ($($generated:tt)*) => { $($generated)* };
    }

    #[cfg(not(feature = "magnus"))]
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __magnus_backend {
        ($($generated:tt)*) => {
            ::core::compile_error!("`backend = Magnus` needs the `magnus` feature of `jsonschema`");
        };
    }

    pub use crate::__magnus_backend as magnus_backend;

    pub use ::json_codec;

    pub mod fancy_regex {
        pub use fancy_regex::{Regex, RegexBuilder};
    }
    pub mod regex {
        pub use jsonschema_regex::contains_ecma_whitespace;
        pub use regex::{Regex, RegexBuilder};
    }
    pub mod unique_items {
        pub use crate::unique::is_unique;
    }
    pub mod cmp {
        pub use crate::cmp::{equal, equal_numbers};
    }
    pub mod custom {
        use crate::{
            json::{Json, Node},
            paths::Location,
        };

        #[must_use]
        pub fn location(pointer: &str) -> Location {
            Location::from_escaped(pointer)
        }

        /// Run a custom keyword and fill in error context exactly like the runtime validator's `CustomKeyword` wrapper.
        pub fn validate<'i, F: Json>(
            keyword: &dyn crate::Keyword<'i, F>,
            instance: &F::Node<'i>,
            instance_path: Location,
            schema_path: &str,
            keyword_name: &str,
        ) -> Option<crate::ValidationError<'i>> {
            match keyword.validate(instance.clone()) {
                Ok(()) => None,
                Err(error) => Some(error.with_generated_context(
                    instance.to_value(),
                    instance_path,
                    Location::from_escaped(schema_path),
                    keyword_name,
                )),
            }
        }

        /// Run a custom keyword's `iter_errors`, filling in context exactly like [`validate`] does.
        pub fn collect_errors<'i, F: Json>(
            keyword: &dyn crate::Keyword<'i, F>,
            instance: &F::Node<'i>,
            instance_path: &Location,
            schema_path: &str,
            keyword_name: &str,
            errors: &mut Vec<crate::ValidationError<'i>>,
        ) {
            let mut found = keyword.iter_errors(instance.clone()).peekable();
            if found.peek().is_none() {
                return;
            }
            let value = instance.to_value();
            for error in found {
                errors.push(error.with_generated_context(
                    value.clone(),
                    instance_path.clone(),
                    Location::from_escaped(schema_path),
                    keyword_name,
                ));
            }
        }
    }
    pub mod types {
        /// Integer check per drafts 6+: floats with zero fractional part count.
        #[must_use]
        pub fn is_integer(n: &json_codec::Number) -> bool {
            jsonschema_value::types::number_is_integer(n)
        }
        /// Integer check per draft 4: numbers written with a decimal point are not integers.
        #[must_use]
        pub fn is_integer_draft4(n: &json_codec::Number) -> bool {
            crate::keywords::legacy::type_draft_4::is_integer(n)
        }
    }
    pub mod format {
        #[cfg(feature = "idna")]
        pub use crate::keywords::format::is_valid_idn_hostname;
        pub use crate::keywords::format::{
            is_valid_date, is_valid_datetime, is_valid_duration, is_valid_hostname,
            is_valid_hostname_rfc1034, is_valid_ipv4, is_valid_ipv6, is_valid_iri,
            is_valid_iri_reference, is_valid_json_pointer, is_valid_regex,
            is_valid_relative_json_pointer, is_valid_time, is_valid_uri, is_valid_uri_reference,
            is_valid_uri_template, is_valid_uuid,
        };

        // Without `idna` the format is unknown, and an unknown format admits every instance.
        #[cfg(not(feature = "idna"))]
        #[must_use]
        pub fn is_valid_idn_hostname(_: &str) -> bool {
            true
        }

        /// Per-call memoization cache for URI/IRI `format` checks in generated validators.
        pub type Cache = std::collections::HashMap<Box<str>, bool, ahash::RandomState>;

        /// Validate an email format using configured [`crate::EmailOptions`].
        #[must_use]
        pub fn is_valid_email_with_options(
            value: &str,
            options: Option<&crate::EmailOptions>,
        ) -> bool {
            crate::keywords::format::is_valid_email(value, options.map(|opts| &opts.inner))
        }

        /// Validate an IDN email format using configured [`crate::EmailOptions`].
        #[cfg(feature = "idna")]
        #[must_use]
        pub fn is_valid_idn_email_with_options(
            value: &str,
            options: Option<&crate::EmailOptions>,
        ) -> bool {
            crate::keywords::format::is_valid_idn_email(value, options.map(|opts| &opts.inner))
        }

        // Without `idna` the format is unknown, and an unknown format admits every instance.
        #[cfg(not(feature = "idna"))]
        #[must_use]
        pub fn is_valid_idn_email_with_options(_: &str, _: Option<&crate::EmailOptions>) -> bool {
            true
        }
    }
    pub mod content {
        pub use crate::{
            content_encoding::{
                from_base16, from_base32, from_base32hex, from_base64, from_base64url, is_base16,
                is_base32, is_base32hex, is_base64, is_base64url,
            },
            content_media_type::is_json,
        };
    }
    pub mod numeric {
        /// Compare `value` >= `limit` using runtime numeric semantics.
        pub fn ge<T>(value: &json_codec::Number, limit: T) -> bool
        where
            T: Copy + ::numeric::traits::ToPrimitive,
            u64: num_cmp::NumCmp<T>,
            i64: num_cmp::NumCmp<T>,
            f64: num_cmp::NumCmp<T>,
        {
            crate::numeric::ge(value, limit)
        }

        /// Compare `value` <= `limit` using runtime numeric semantics.
        pub fn le<T>(value: &json_codec::Number, limit: T) -> bool
        where
            T: Copy + ::numeric::traits::ToPrimitive,
            u64: num_cmp::NumCmp<T>,
            i64: num_cmp::NumCmp<T>,
            f64: num_cmp::NumCmp<T>,
        {
            crate::numeric::le(value, limit)
        }

        pub fn eq<T>(value: &json_codec::Number, limit: T) -> bool
        where
            T: Copy + ::numeric::traits::ToPrimitive,
            u64: num_cmp::NumCmp<T>,
            i64: num_cmp::NumCmp<T>,
            f64: num_cmp::NumCmp<T>,
        {
            crate::numeric::eq(value, limit)
        }

        /// Compare `value` > `limit` using runtime numeric semantics.
        pub fn gt<T>(value: &json_codec::Number, limit: T) -> bool
        where
            T: Copy + ::numeric::traits::ToPrimitive,
            u64: num_cmp::NumCmp<T>,
            i64: num_cmp::NumCmp<T>,
            f64: num_cmp::NumCmp<T>,
        {
            crate::numeric::gt(value, limit)
        }

        /// Compare `value` < `limit` using runtime numeric semantics.
        pub fn lt<T>(value: &json_codec::Number, limit: T) -> bool
        where
            T: Copy + ::numeric::traits::ToPrimitive,
            u64: num_cmp::NumCmp<T>,
            i64: num_cmp::NumCmp<T>,
            f64: num_cmp::NumCmp<T>,
        {
            crate::numeric::lt(value, limit)
        }

        /// Check `multipleOf` with integer divisors using runtime numeric semantics.
        #[must_use]
        pub fn is_multiple_of_integer(value: &json_codec::Number, multiple: f64) -> bool {
            crate::numeric::is_multiple_of_integer(value, multiple)
        }

        /// Check `multipleOf` with fractional divisors using runtime numeric semantics.
        #[must_use]
        pub fn is_multiple_of_float(value: &json_codec::Number, multiple: f64) -> bool {
            crate::numeric::is_multiple_of_float(value, multiple)
        }

        /// Check numeric bounds with a compiled descriptor for arbitrary-precision schemas.
        #[cfg(feature = "arbitrary-precision")]
        #[must_use]
        pub fn check_compiled_bound(
            value: &json_codec::Number,
            op: u8,
            limit_literal: &'static str,
        ) -> bool {
            use jsonschema_value::numeric_check::{
                check_bound, compile_bound, BoundOp, CompiledBound,
            };

            #[inline]
            fn literal_key(literal: &'static str) -> (usize, usize) {
                (literal.as_ptr() as usize, literal.len())
            }

            type BoundCache =
                std::cell::RefCell<ahash::AHashMap<((usize, usize), u8), CompiledBound>>;
            std::thread_local! {
                static CACHE: BoundCache = std::cell::RefCell::new(ahash::AHashMap::default());
            }

            let op_tag = op;
            let Some(op) = BoundOp::from_u8(op_tag) else {
                unreachable!("codegen emits only bound op tags 0..=3")
            };
            let key = (literal_key(limit_literal), op_tag);

            CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let compiled = cache.entry(key).or_insert_with(|| {
                    let limit = json_codec::from_str::<json_codec::Number>(limit_literal)
                        .expect("Codegen emitted an invalid numeric literal");
                    compile_bound(op, &limit)
                });
                check_bound(compiled, value)
            })
        }

        /// Check `multipleOf` with a compiled descriptor for arbitrary-precision schemas.
        #[cfg(feature = "arbitrary-precision")]
        #[must_use]
        pub fn check_compiled_multiple_of(
            value: &json_codec::Number,
            limit_literal: &'static str,
        ) -> bool {
            use jsonschema_value::numeric_check::{
                check_multiple_of, compile_multiple_of, CompiledMultipleOf,
            };

            #[inline]
            fn literal_key(literal: &'static str) -> (usize, usize) {
                (literal.as_ptr() as usize, literal.len())
            }

            std::thread_local! {
                static CACHE: std::cell::RefCell<ahash::AHashMap<(usize, usize), CompiledMultipleOf>> =
                    std::cell::RefCell::new(ahash::AHashMap::default());
            }
            let key = literal_key(limit_literal);

            CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let compiled = cache.entry(key).or_insert_with(|| {
                    let limit = json_codec::from_str::<json_codec::Number>(limit_literal)
                        .expect("Codegen emitted an invalid numeric literal");
                    compile_multiple_of(&limit)
                });
                check_multiple_of(compiled, value)
            })
        }
    }

    /// Per-keyword error constructors for generated code; evaluation path always equals schema path.
    pub mod error {
        use json_codec::Value;

        use crate::{
            paths::Location,
            types::{JsonType, JsonTypeSet},
            validator::LazyEvaluationPath,
            ErrorIterator, LazyInstance, ValidationError,
        };

        /// Wrap errors collected by generated `collect_errors` code into an [`ErrorIterator`].
        #[inline]
        #[must_use]
        pub fn iterator_from(errors: Vec<ValidationError<'_>>) -> ErrorIterator<'_> {
            ErrorIterator::from_iterator(errors.into_iter())
        }

        /// `contentEncoding` keyword violation.
        #[inline]
        pub fn content_encoding<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            encoding: &str,
        ) -> ValidationError<'i> {
            ValidationError::content_encoding(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                encoding,
            )
        }

        /// `contentMediaType` keyword violation.
        #[inline]
        pub fn content_media_type<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            media_type: &str,
        ) -> ValidationError<'i> {
            ValidationError::content_media_type(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                media_type,
            )
        }

        /// `minLength` keyword violation: string is shorter than `limit` characters.
        #[inline]
        pub fn min_length<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: u64,
        ) -> ValidationError<'i> {
            ValidationError::min_length(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `maxLength` keyword violation: string exceeds `limit` characters.
        #[inline]
        pub fn max_length<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: u64,
        ) -> ValidationError<'i> {
            ValidationError::max_length(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `pattern` keyword violation.
        #[inline]
        pub fn pattern<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            pattern: &str,
        ) -> ValidationError<'i> {
            ValidationError::pattern(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                pattern.to_string(),
            )
        }

        /// `format` keyword violation.
        #[inline]
        pub fn format<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            format: &str,
        ) -> ValidationError<'i> {
            ValidationError::format(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                format,
            )
        }

        /// `type` keyword violation: instance is not one of the expected JSON types.
        ///
        /// Use [`single_type`] when only one type is expected, [`multiple_types`] for union types.
        #[inline]
        pub fn single_type<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            ty: JsonType,
        ) -> ValidationError<'i> {
            ValidationError::single_type_error(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                ty,
            )
        }

        /// `type` keyword violation for a union of expected types.
        #[inline]
        pub fn multiple_types<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            types: JsonTypeSet,
        ) -> ValidationError<'i> {
            ValidationError::multiple_type_error(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                types,
            )
        }

        /// `minimum` keyword violation: value is less than `limit`.
        #[inline]
        pub fn minimum<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: Value,
        ) -> ValidationError<'i> {
            ValidationError::minimum(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `maximum` keyword violation: value exceeds `limit`.
        #[inline]
        pub fn maximum<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: Value,
        ) -> ValidationError<'i> {
            ValidationError::maximum(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `exclusiveMinimum` keyword violation: value is not strictly greater than `limit`.
        #[inline]
        pub fn exclusive_minimum<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: Value,
        ) -> ValidationError<'i> {
            ValidationError::exclusive_minimum(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `exclusiveMaximum` keyword violation: value is not strictly less than `limit`.
        #[inline]
        pub fn exclusive_maximum<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: Value,
        ) -> ValidationError<'i> {
            ValidationError::exclusive_maximum(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `multipleOf` keyword violation (non-arbitrary-precision).
        #[cfg(not(feature = "arbitrary-precision"))]
        #[inline]
        pub fn multiple_of<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            multiple_of: f64,
        ) -> ValidationError<'i> {
            ValidationError::multiple_of(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                multiple_of,
            )
        }

        /// `multipleOf` keyword violation (arbitrary-precision).
        #[cfg(feature = "arbitrary-precision")]
        #[inline]
        pub fn multiple_of<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            multiple_of: Value,
        ) -> ValidationError<'i> {
            ValidationError::multiple_of(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                multiple_of,
            )
        }

        /// `minItems` keyword violation: array has fewer items than `limit`.
        #[inline]
        pub fn min_items<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: u64,
        ) -> ValidationError<'i> {
            ValidationError::min_items(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `maxItems` keyword violation: array exceeds `limit` items.
        #[inline]
        pub fn max_items<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: u64,
        ) -> ValidationError<'i> {
            ValidationError::max_items(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `additionalItems` keyword violation (Draft 4 / 2019-09 tuple form).
        #[inline]
        pub fn additional_items<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: usize,
        ) -> ValidationError<'i> {
            ValidationError::additional_items(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `uniqueItems` keyword violation: array contains duplicate elements.
        #[inline]
        pub fn unique_items<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
        ) -> ValidationError<'i> {
            ValidationError::unique_items(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
            )
        }

        /// `contains` keyword violation: no array item matched the `contains` schema.
        #[inline]
        pub fn contains<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
        ) -> ValidationError<'i> {
            ValidationError::contains(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
            )
        }

        /// `minProperties` keyword violation: object has fewer properties than `limit`.
        #[inline]
        pub fn min_properties<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: u64,
        ) -> ValidationError<'i> {
            ValidationError::min_properties(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `maxProperties` keyword violation: object exceeds `limit` properties.
        #[inline]
        pub fn max_properties<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            limit: u64,
        ) -> ValidationError<'i> {
            ValidationError::max_properties(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                limit,
            )
        }

        /// `required` keyword violation: object is missing `property`.
        #[inline]
        pub fn required<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            property: &str,
        ) -> ValidationError<'i> {
            ValidationError::required(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                Value::String(property.to_owned()),
            )
        }

        /// `additionalProperties` keyword violation (the `false` schema form).
        #[inline]
        pub fn additional_properties<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            unexpected: Vec<String>,
        ) -> ValidationError<'i> {
            ValidationError::additional_properties(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                unexpected,
            )
        }

        /// `const` keyword violation: instance does not equal the expected constant.
        #[inline]
        pub fn constant<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            expected: Value,
        ) -> ValidationError<'i> {
            ValidationError::new(
                instance.into(),
                crate::error::ValidationErrorKind::Constant {
                    expected_value: expected,
                },
                instance_path,
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
            )
        }

        /// `enum` keyword violation: instance does not match any of the allowed values.
        #[inline]
        pub fn enumeration<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            options: &Value,
        ) -> ValidationError<'i> {
            ValidationError::enumeration(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                options,
            )
        }

        /// `false` schema violation: nothing is valid against a `false` schema.
        #[inline]
        pub fn false_schema<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
        ) -> ValidationError<'i> {
            ValidationError::false_schema(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
            )
        }

        /// `not` keyword violation: instance is valid against the negated schema.
        #[inline]
        pub fn not<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            schema: Value,
        ) -> ValidationError<'i> {
            ValidationError::not(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                schema,
            )
        }

        /// `anyOf` keyword violation: instance is not valid under any of the listed schemas.
        #[inline]
        pub fn any_of<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            context: Vec<Vec<ValidationError<'i>>>,
        ) -> ValidationError<'i> {
            ValidationError::any_of(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                context,
            )
        }

        /// `oneOf` keyword violation: instance is not valid under any of the listed schemas.
        #[inline]
        pub fn one_of_not_valid<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            context: Vec<Vec<ValidationError<'i>>>,
        ) -> ValidationError<'i> {
            ValidationError::one_of_not_valid(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                context,
            )
        }

        /// `oneOf` keyword violation: instance is valid under more than one of the listed schemas.
        #[inline]
        pub fn one_of_multiple_valid<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            context: Vec<Vec<ValidationError<'i>>>,
        ) -> ValidationError<'i> {
            ValidationError::one_of_multiple_valid(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                context,
            )
        }

        /// `unevaluatedProperties` keyword violation.
        #[inline]
        pub fn unevaluated_properties<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            unexpected: Vec<String>,
        ) -> ValidationError<'i> {
            ValidationError::unevaluated_properties(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                unexpected,
            )
        }

        /// `unevaluatedItems` keyword violation.
        #[inline]
        pub fn unevaluated_items<'i>(
            schema_path: &str,
            instance_path: Location,
            instance: impl Into<LazyInstance<'i>>,
            unexpected: Vec<String>,
        ) -> ValidationError<'i> {
            ValidationError::unevaluated_items(
                Location::from_escaped(schema_path),
                LazyEvaluationPath::SameAsSchemaPath,
                instance_path,
                instance.into(),
                unexpected,
            )
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/lib/tests_util_4365.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/lib/tests_4700.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/lib/all_async_tests_4371.rs"));
