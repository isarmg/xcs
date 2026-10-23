pub(crate) mod additional_items;
pub(crate) mod additional_properties;
pub(crate) mod all_of;
pub(crate) mod any_of;
pub(crate) mod boolean;
pub(crate) mod const_;
pub(crate) mod contains;
pub(crate) mod content;
pub(crate) mod custom;
pub(crate) mod dependencies;
pub(crate) mod discriminator;
pub(crate) mod enum_;
pub(crate) mod format;
pub(crate) mod helpers;
pub(crate) mod if_;
pub(crate) mod items;
pub(crate) mod legacy;
pub(crate) mod max_items;
pub(crate) mod max_length;
pub(crate) mod max_properties;
pub(crate) mod min_items;
pub(crate) mod min_length;
pub(crate) mod min_properties;
pub(crate) mod minmax;
pub(crate) mod multiple_of;
pub(crate) mod not;
pub(crate) mod one_of;
pub(crate) mod pattern;
pub(crate) mod pattern_properties;
pub(crate) mod prefix_items;
pub(crate) mod properties;
pub(crate) mod property_names;
pub(crate) mod ref_;
pub(crate) mod required;
pub(crate) mod rfc3986;
mod uuid_format;
pub(crate) mod type_;
pub(crate) mod unevaluated_items;
pub(crate) mod unevaluated_properties;
pub(crate) mod unique_items;
use core::fmt;

use crate::reference::{Draft, Vocabulary};
use json_codec::{Map, Value};

use crate::validation::{compiler, error, validator::Validate, Json, JsonValueBackend};

/// Whether a validator specialized for one type also accepts `null`
pub(crate) trait Nullability: Send + Sync + fmt::Debug + 'static {
    const ACCEPTS_NULL: bool;
}

#[derive(Debug)]
pub(crate) struct Nullable;

#[derive(Debug)]
pub(crate) struct NotNullable;

impl Nullability for Nullable {
    const ACCEPTS_NULL: bool = true;
}

impl Nullability for NotNullable {
    const ACCEPTS_NULL: bool = false;
}

pub(crate) type CompilationResult<'a, F = JsonValueBackend> =
    Result<BoxedValidator<F>, error::ValidationError<'a>>;
pub(crate) type BoxedValidator<F = JsonValueBackend> = Box<dyn Validate<F>>;

type CompileFunc<'a, F> = fn(
    &'a compiler::Context<F>,
    &'a Map<String, Value>,
    &'a Value,
) -> Option<CompilationResult<'a, F>>;

#[derive(Debug, Clone)]
pub(crate) enum Keyword {
    Builtin(BuiltinKeyword),
    Custom(Box<str>),
}

#[derive(Debug, Clone)]
pub(crate) enum BuiltinKeyword {
    Ref,
    AdditionalItems,
    AdditionalProperties,
    AllOf,
    AnyOf,
    Dependencies,
    Enum,
    Format,
    Items,
    MaxItems,
    MaxLength,
    MaxProperties,
    MinItems,
    MinLength,
    MinProperties,
    MultipleOf,
    Not,
    OneOf,
    Pattern,
    PatternProperties,
    Properties,
    Required,
    UniqueItems,
    Maximum,
    Minimum,
    Type,
    Const,
    Contains,
    ExclusiveMaximum,
    ExclusiveMinimum,
    PropertyNames,
    ContentMediaType,
    ContentEncoding,
    ContentSchema,
    If,
    RecursiveRef,
    DependentRequired,
    DependentSchemas,
    PrefixItems,
    UnevaluatedItems,
    UnevaluatedProperties,
    DynamicRef,
}

impl BuiltinKeyword {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Ref => "$ref",
            Self::AdditionalItems => "additionalItems",
            Self::AdditionalProperties => "additionalProperties",
            Self::AllOf => "allOf",
            Self::AnyOf => "anyOf",
            Self::Dependencies => "dependencies",
            Self::Enum => "enum",
            Self::Format => "format",
            Self::Items => "items",
            Self::MaxItems => "maxItems",
            Self::MaxLength => "maxLength",
            Self::MaxProperties => "maxProperties",
            Self::MinItems => "minItems",
            Self::MinLength => "minLength",
            Self::MinProperties => "minProperties",
            Self::MultipleOf => "multipleOf",
            Self::Not => "not",
            Self::OneOf => "oneOf",
            Self::Pattern => "pattern",
            Self::PatternProperties => "patternProperties",
            Self::Properties => "properties",
            Self::Required => "required",
            Self::UniqueItems => "uniqueItems",
            Self::Maximum => "maximum",
            Self::Minimum => "minimum",
            Self::Type => "type",
            Self::Const => "const",
            Self::Contains => "contains",
            Self::ExclusiveMaximum => "exclusiveMaximum",
            Self::ExclusiveMinimum => "exclusiveMinimum",
            Self::PropertyNames => "propertyNames",
            Self::ContentMediaType => "contentMediaType",
            Self::ContentEncoding => "contentEncoding",
            Self::ContentSchema => "contentSchema",
            Self::If => "if",
            Self::RecursiveRef => "$recursiveRef",
            Self::DependentRequired => "dependentRequired",
            Self::DependentSchemas => "dependentSchemas",
            Self::PrefixItems => "prefixItems",
            Self::UnevaluatedItems => "unevaluatedItems",
            Self::UnevaluatedProperties => "unevaluatedProperties",
            Self::DynamicRef => "$dynamicRef",
        }
    }
}

impl Keyword {
    pub(crate) fn custom(name: impl Into<String>) -> Self {
        Keyword::Custom(name.into().into_boxed_str())
    }
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Builtin(d) => d.as_str(),
            Self::Custom(s) => s,
        }
    }
}

impl From<BuiltinKeyword> for Keyword {
    fn from(value: BuiltinKeyword) -> Self {
        Keyword::Builtin(value)
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Returns the execution priority for a keyword (lower = execute first).
/// This enables "fail fast" by running cheap validators before expensive ones.
///
/// Priority groups:
/// - 1-9: Type checks and simple assertions (very cheap)
/// - 10-19: Value constraints and simple validators
/// - 20-29: String/array/object size constraints
/// - 30-39: Property/item validators
/// - 40-49: Composition validators (allOf, anyOf, oneOf)
/// - 50+: Reference validators (most expensive due to indirection)
pub(crate) fn keyword_priority(keyword: &Keyword) -> u8 {
    match keyword {
        // Type checks are extremely cheap (single discriminant comparison)
        Keyword::Builtin(BuiltinKeyword::Type) => 1,

        // Const/enum are cheap (direct value comparison)
        Keyword::Builtin(BuiltinKeyword::Const) => 5,
        Keyword::Builtin(BuiltinKeyword::Enum) => 6,

        // Simple numeric assertions
        Keyword::Builtin(BuiltinKeyword::Minimum) => 10,
        Keyword::Builtin(BuiltinKeyword::Maximum) => 11,
        Keyword::Builtin(BuiltinKeyword::ExclusiveMinimum) => 12,
        Keyword::Builtin(BuiltinKeyword::ExclusiveMaximum) => 13,
        Keyword::Builtin(BuiltinKeyword::MultipleOf) => 14,

        // Size constraints (also cheap)
        Keyword::Builtin(BuiltinKeyword::MinLength) => 20,
        Keyword::Builtin(BuiltinKeyword::MaxLength) => 21,
        Keyword::Builtin(BuiltinKeyword::MinItems) => 22,
        Keyword::Builtin(BuiltinKeyword::MaxItems) => 23,
        Keyword::Builtin(BuiltinKeyword::MinProperties) => 24,
        Keyword::Builtin(BuiltinKeyword::MaxProperties) => 25,

        // Required is relatively cheap (just checks key existence)
        Keyword::Builtin(BuiltinKeyword::Required) => 26,
        Keyword::Builtin(BuiltinKeyword::DependentRequired) => 27,

        // Pattern/format validation (moderate - involves string processing)
        Keyword::Builtin(BuiltinKeyword::Pattern) => 30,
        Keyword::Builtin(BuiltinKeyword::Format) => 31,
        Keyword::Builtin(BuiltinKeyword::ContentEncoding) => 32,
        Keyword::Builtin(BuiltinKeyword::ContentMediaType) => 33,
        Keyword::Builtin(BuiltinKeyword::ContentSchema) => 34,

        // Unique items (potentially O(n²) for arrays)
        Keyword::Builtin(BuiltinKeyword::UniqueItems) => 35,

        // Property/item validators (iterate over children)
        Keyword::Builtin(BuiltinKeyword::Properties) => 40,
        Keyword::Builtin(BuiltinKeyword::PatternProperties) => 41,
        Keyword::Builtin(BuiltinKeyword::AdditionalProperties) => 42,
        Keyword::Builtin(BuiltinKeyword::PropertyNames) => 43,
        Keyword::Builtin(BuiltinKeyword::Items) => 44,
        Keyword::Builtin(BuiltinKeyword::PrefixItems) => 45,
        Keyword::Builtin(BuiltinKeyword::AdditionalItems) => 46,
        Keyword::Builtin(BuiltinKeyword::Contains) => 47,
        Keyword::Builtin(BuiltinKeyword::Dependencies) => 48,
        Keyword::Builtin(BuiltinKeyword::DependentSchemas) => 49,

        // Composition validators (validate against multiple subschemas)
        Keyword::Builtin(BuiltinKeyword::AllOf) => 50,
        Keyword::Builtin(BuiltinKeyword::AnyOf) => 51,
        Keyword::Builtin(BuiltinKeyword::OneOf) => 52,
        Keyword::Builtin(BuiltinKeyword::Not) => 53,
        Keyword::Builtin(BuiltinKeyword::If) => 54,

        // Unevaluated validators (depend on other validators' results)
        Keyword::Builtin(BuiltinKeyword::UnevaluatedProperties) => 60,
        Keyword::Builtin(BuiltinKeyword::UnevaluatedItems) => 61,

        // Reference validators (most expensive - involve indirection)
        Keyword::Builtin(BuiltinKeyword::Ref) => 70,
        Keyword::Builtin(BuiltinKeyword::RecursiveRef) => 71,
        Keyword::Builtin(BuiltinKeyword::DynamicRef) => 72,

        // Custom keywords get default priority
        Keyword::Custom(_) => 80,
    }
}

/// Whether the keyword asserts on the instance itself rather than applying subschemas.
///
/// Only such a keyword may be checked with `is_valid` before `validate`: re-running it on
/// failure repeats one cheap check, while re-running an applicator would walk its subtree twice.
pub(crate) fn keyword_is_leaf(keyword: &Keyword) -> bool {
    match keyword {
        Keyword::Builtin(BuiltinKeyword::ContentSchema) | Keyword::Custom(_) => false,
        Keyword::Builtin(_) => keyword_priority(keyword) < 40,
    }
}

pub(crate) fn get_for_draft<'a, F: Json>(
    ctx: &compiler::Context<'_, F>,
    keyword: &'a str,
) -> Option<(Keyword, CompileFunc<'a, F>)> {
    match (ctx.draft(), keyword) {
        // Keywords common to all drafts
        (_, "$ref") => Some((BuiltinKeyword::Ref.into(), ref_::compile_ref)),
        (_, "additionalItems") if ctx.has_vocabulary(&Vocabulary::Applicator) => Some((
            BuiltinKeyword::AdditionalItems.into(),
            additional_items::compile,
        )),
        (_, "additionalProperties") if ctx.has_vocabulary(&Vocabulary::Applicator) => Some((
            BuiltinKeyword::AdditionalProperties.into(),
            additional_properties::compile,
        )),
        (_, "allOf") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::AllOf.into(), all_of::compile))
        }
        (_, "anyOf") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::AnyOf.into(), any_of::compile))
        }
        (_, "dependencies") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::Dependencies.into(), dependencies::compile))
        }
        (_, "enum") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Enum.into(), enum_::compile))
        }
        (Draft::Draft201909, "format") if ctx.has_vocabulary(&Vocabulary::Format) => {
            Some((BuiltinKeyword::Format.into(), format::compile))
        }
        (Draft::Draft202012 | Draft::Unknown, "format")
            if ctx.has_vocabulary(&Vocabulary::FormatAnnotation) =>
        {
            Some((BuiltinKeyword::Format.into(), format::compile))
        }
        (_, "format") => Some((BuiltinKeyword::Format.into(), format::compile)),
        (_, "items") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::Items.into(), items::compile))
        }
        (_, "maxItems") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::MaxItems.into(), max_items::compile))
        }
        (_, "maxLength") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::MaxLength.into(), max_length::compile))
        }
        (_, "maxProperties") if ctx.has_vocabulary(&Vocabulary::Validation) => Some((
            BuiltinKeyword::MaxProperties.into(),
            max_properties::compile,
        )),
        (_, "minItems") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::MinItems.into(), min_items::compile))
        }
        (_, "minLength") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::MinLength.into(), min_length::compile))
        }
        (_, "minProperties") if ctx.has_vocabulary(&Vocabulary::Validation) => Some((
            BuiltinKeyword::MinProperties.into(),
            min_properties::compile,
        )),
        (_, "multipleOf") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::MultipleOf.into(), multiple_of::compile))
        }
        (_, "not") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::Not.into(), not::compile))
        }
        (_, "oneOf") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::OneOf.into(), one_of::compile))
        }
        (_, "pattern") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Pattern.into(), pattern::compile))
        }
        (_, "patternProperties") if ctx.has_vocabulary(&Vocabulary::Applicator) => Some((
            BuiltinKeyword::PatternProperties.into(),
            pattern_properties::compile,
        )),
        (_, "properties") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::Properties.into(), properties::compile))
        }
        (_, "required") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Required.into(), required::compile))
        }
        (_, "uniqueItems") if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::UniqueItems.into(), unique_items::compile))
        }
        // Draft 4 specific
        (Draft::Draft4, "maximum") => Some((
            BuiltinKeyword::Maximum.into(),
            legacy::maximum_draft_4::compile,
        )),
        (Draft::Draft4, "minimum") => Some((
            BuiltinKeyword::Minimum.into(),
            legacy::minimum_draft_4::compile,
        )),
        (Draft::Draft4, "type") => {
            Some((BuiltinKeyword::Type.into(), legacy::type_draft_4::compile))
        }
        // Draft 6 and later
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "const",
        ) if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Const.into(), const_::compile))
        }
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "contains",
        ) if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((BuiltinKeyword::Contains.into(), contains::compile))
        }
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "exclusiveMaximum",
        ) if ctx.has_vocabulary(&Vocabulary::Validation) => Some((
            BuiltinKeyword::ExclusiveMaximum.into(),
            minmax::compile_exclusive_maximum,
        )),
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "exclusiveMinimum",
        ) if ctx.has_vocabulary(&Vocabulary::Validation) => Some((
            BuiltinKeyword::ExclusiveMinimum.into(),
            minmax::compile_exclusive_minimum,
        )),
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "maximum",
        ) if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Maximum.into(), minmax::compile_maximum))
        }
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "minimum",
        ) if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Minimum.into(), minmax::compile_minimum))
        }
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "propertyNames",
        ) if ctx.has_vocabulary(&Vocabulary::Applicator) => Some((
            BuiltinKeyword::PropertyNames.into(),
            property_names::compile,
        )),
        (
            Draft::Draft6
            | Draft::Draft7
            | Draft::Draft201909
            | Draft::Draft202012
            | Draft::Unknown,
            "type",
        ) if ctx.has_vocabulary(&Vocabulary::Validation) => {
            Some((BuiltinKeyword::Type.into(), type_::compile))
        }
        (Draft::Draft6 | Draft::Draft7, "contentMediaType") => Some((
            BuiltinKeyword::ContentMediaType.into(),
            content::compile_media_type,
        )),
        (Draft::Draft6 | Draft::Draft7, "contentEncoding") => Some((
            BuiltinKeyword::ContentEncoding.into(),
            content::compile_content_encoding,
        )),
        (Draft::Draft201909 | Draft::Draft202012 | Draft::Unknown, "contentMediaType")
            if ctx.has_vocabulary(&Vocabulary::Content) =>
        {
            Some((
                BuiltinKeyword::ContentMediaType.into(),
                content::compile_media_type_annotation,
            ))
        }
        (Draft::Draft201909 | Draft::Draft202012 | Draft::Unknown, "contentEncoding")
            if ctx.has_vocabulary(&Vocabulary::Content) =>
        {
            Some((
                BuiltinKeyword::ContentEncoding.into(),
                content::compile_content_encoding_annotation,
            ))
        }
        (Draft::Draft201909 | Draft::Draft202012 | Draft::Unknown, "contentSchema")
            if ctx.has_vocabulary(&Vocabulary::Content) =>
        {
            Some((
                BuiltinKeyword::ContentSchema.into(),
                content::compile_content_schema_annotation,
            ))
        }
        (Draft::Draft7 | Draft::Draft201909 | Draft::Draft202012 | Draft::Unknown, "if")
            if ctx.has_vocabulary(&Vocabulary::Applicator) =>
        {
            Some((BuiltinKeyword::If.into(), if_::compile))
        }
        // Draft 2019-09 specific
        (Draft::Draft201909, "$recursiveRef") => Some((
            BuiltinKeyword::RecursiveRef.into(),
            ref_::compile_recursive_ref,
        )),
        (Draft::Draft201909 | Draft::Draft202012 | Draft::Unknown, "dependentRequired")
            if ctx.has_vocabulary(&Vocabulary::Validation) =>
        {
            Some((
                BuiltinKeyword::DependentRequired.into(),
                dependencies::compile_dependent_required,
            ))
        }
        (Draft::Draft201909 | Draft::Draft202012 | Draft::Unknown, "dependentSchemas")
            if ctx.has_vocabulary(&Vocabulary::Applicator) =>
        {
            Some((
                BuiltinKeyword::DependentSchemas.into(),
                dependencies::compile_dependent_schemas,
            ))
        }
        (Draft::Draft201909, "unevaluatedItems") if ctx.has_vocabulary(&Vocabulary::Applicator) => {
            Some((
                BuiltinKeyword::UnevaluatedItems.into(),
                unevaluated_items::compile,
            ))
        }
        (Draft::Draft202012 | Draft::Unknown, "unevaluatedItems")
            if ctx.has_vocabulary(&Vocabulary::Unevaluated) =>
        {
            Some((
                BuiltinKeyword::UnevaluatedItems.into(),
                unevaluated_items::compile,
            ))
        }
        (Draft::Draft201909, "unevaluatedProperties")
            if ctx.has_vocabulary(&Vocabulary::Applicator) =>
        {
            Some((
                BuiltinKeyword::UnevaluatedProperties.into(),
                unevaluated_properties::compile,
            ))
        }
        (Draft::Draft202012 | Draft::Unknown, "unevaluatedProperties")
            if ctx.has_vocabulary(&Vocabulary::Unevaluated) =>
        {
            Some((
                BuiltinKeyword::UnevaluatedProperties.into(),
                unevaluated_properties::compile,
            ))
        }
        // Draft 2020-12 specific
        (Draft::Draft202012 | Draft::Unknown, "prefixItems")
            if ctx.has_vocabulary(&Vocabulary::Applicator) =>
        {
            Some((BuiltinKeyword::PrefixItems.into(), prefix_items::compile))
        }
        (Draft::Draft202012 | Draft::Unknown, "$dynamicRef") => {
            Some((BuiltinKeyword::DynamicRef.into(), ref_::compile_dynamic_ref))
        }
        // Unknown or not-yet-implemented keyword
        _ => None,
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/mod/tests_575.rs"
));
