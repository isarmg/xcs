//! `oneOf` / `anyOf` branches that each require one property and fix it to their own string
//! `const`. Against an object only the branch holding the instance's value can pass, so that
//! branch alone decides.
use crate::reference::{Draft, Resolver, Vocabulary};
use ahash::{AHashMap, AHashSet};
use json_codec::{Map, Value};

use crate::validation::{compiler, Json, Node, Object};

// Deep enough for real `$ref` chains; a cycle stops here.
const MAX_DEPTH: usize = 16;
const MIN_BRANCHES: usize = 3;

pub(crate) struct Discriminator<F: Json> {
    key: F::PreparedKey,
    branches: AHashMap<String, usize>,
}

impl<F: Json> Discriminator<F> {
    pub(crate) fn compile(ctx: &compiler::Context<F>, items: &[Value]) -> Option<Self> {
        // With two branches a dispatch saves at most one branch, about what the analysis costs.
        if items.len() < MIN_BRANCHES
            || !ctx.has_vocabulary(&Vocabulary::Validation)
            || !ctx.has_vocabulary(&Vocabulary::Applicator)
            || ["required", "properties", "const", "enum", "allOf", "$ref"]
                .iter()
                .any(|keyword| ctx.is_keyword_overridden(keyword))
        {
            return None;
        }
        let mut branches: Vec<AHashMap<&str, &str>> = Vec::with_capacity(items.len());
        for item in items {
            let mut pins = Pins::default();
            pins.collect(ctx.resolver(), ctx.draft(), item, 0);
            // Only keys the first branch pins can pin every branch.
            let wanted = branches.first();
            let values = pins.required_values(wanted);
            if values.is_empty() {
                return None;
            }
            branches.push(values);
        }
        let mut keys: Vec<&str> = branches.first()?.keys().copied().collect();
        keys.sort_unstable();
        'keys: for key in keys {
            let mut map = AHashMap::with_capacity(items.len());
            for (idx, values) in branches.iter().enumerate() {
                let Some(value) = values.get(key) else {
                    continue 'keys;
                };
                if map.insert((*value).to_string(), idx).is_some() {
                    continue 'keys;
                }
            }
            return Some(Discriminator {
                key: F::prepare_key(key),
                branches: map,
            });
        }
        None
    }

    #[inline]
    pub(crate) fn candidates(&self, instance: &F::Node<'_>) -> Candidates {
        // `required` and `properties` do not constrain anything but an object.
        let Some(object) = instance.as_object() else {
            return Candidates::All;
        };
        object
            .get(&self.key)
            .and_then(|value| value.as_string())
            .and_then(|value| self.branches.get(value.as_ref()).copied())
            .map_or(Candidates::None, Candidates::One)
    }
}

/// Narrows the branches of `oneOf` / `anyOf` that can accept an instance.
pub(crate) trait Dispatch<F: Json>: Send + Sync {
    fn candidates(&self, instance: &F::Node<'_>) -> Candidates;
}

/// Every branch stays a candidate; compiles down to the plain loop.
pub(crate) struct NoDispatch;

impl<F: Json> Dispatch<F> for NoDispatch {
    #[inline]
    fn candidates(&self, _: &F::Node<'_>) -> Candidates {
        Candidates::All
    }
}

impl<F: Json> Dispatch<F> for Discriminator<F> {
    #[inline]
    fn candidates(&self, instance: &F::Node<'_>) -> Candidates {
        Discriminator::candidates(self, instance)
    }
}

/// Which branches can accept an instance.
pub(crate) enum Candidates {
    All,
    One(usize),
    None,
}

/// What a branch provably requires: only keywords that certainly apply contribute, so anything
/// left unexamined can only constrain the branch further.
#[derive(Default)]
struct Pins<'a> {
    required: AHashSet<&'a str>,
    properties: Vec<(&'a Map<String, Value>, Resolver<'a>, Draft)>,
}

impl<'a> Pins<'a> {
    fn collect(&mut self, resolver: &Resolver<'a>, draft: Draft, schema: &'a Value, depth: usize) {
        let Value::Object(map) = schema else {
            return;
        };
        if depth > MAX_DEPTH {
            return;
        }
        if let Some(Value::String(reference)) = map.get("$ref") {
            if let Some((contents, resolver)) = resolve(resolver, draft, reference) {
                self.collect(&resolver, draft, contents, depth + 1);
            }
            // Drafts 4 to 7 ignore everything next to `$ref`.
            if matches!(draft, Draft::Draft4 | Draft::Draft6 | Draft::Draft7) {
                return;
            }
        }
        if let Some(Value::Array(required)) = map.get("required") {
            self.required
                .extend(required.iter().filter_map(Value::as_str));
        }
        if let Some(Value::Object(properties)) = map.get("properties") {
            self.properties.push((properties, resolver.clone(), draft));
        }
        if let Some(Value::Array(all_of)) = map.get("allOf") {
            for item in all_of {
                self.collect(resolver, draft, item, depth + 1);
            }
        }
    }

    /// Required names fixed to one string, limited to the keys of `wanted` when given.
    fn required_values(self, wanted: Option<&AHashMap<&str, &str>>) -> AHashMap<&'a str, &'a str> {
        let mut values = AHashMap::new();
        for name in self.required {
            if wanted.is_some_and(|wanted| !wanted.contains_key(name)) {
                continue;
            }
            for (properties, resolver, draft) in &self.properties {
                if let Some(value) = Map::get(properties, name)
                    .and_then(|subschema| pinned_value(resolver, *draft, subschema, 0))
                {
                    values.insert(name, value);
                    break;
                }
            }
        }
        values
    }
}

/// The one string a subschema accepts, from `const` or a one-item `enum`.
fn pinned_value<'a>(
    resolver: &Resolver<'a>,
    draft: Draft,
    schema: &'a Value,
    depth: usize,
) -> Option<&'a str> {
    let Value::Object(map) = schema else {
        return None;
    };
    if depth > MAX_DEPTH {
        return None;
    }
    if let Some(Value::String(reference)) = map.get("$ref") {
        if let Some((contents, resolver)) = resolve(resolver, draft, reference) {
            if let Some(value) = pinned_value(&resolver, draft, contents, depth + 1) {
                return Some(value);
            }
        }
        if matches!(draft, Draft::Draft4 | Draft::Draft6 | Draft::Draft7) {
            return None;
        }
    }
    // Draft 4 has no `const`.
    let constant = map.get("const").filter(|_| draft != Draft::Draft4);
    match (constant, map.get("enum")) {
        (Some(Value::String(value)), _) => Some(value),
        (_, Some(Value::Array(values))) => match values.as_slice() {
            [Value::String(value)] => Some(value),
            _ => None,
        },
        _ => None,
    }
}

// A target under another draft reads keywords differently, so it contributes nothing.
fn resolve<'a>(
    resolver: &Resolver<'a>,
    draft: Draft,
    reference: &str,
) -> Option<(&'a Value, Resolver<'a>)> {
    let (contents, resolver, target_draft) = resolver.lookup(reference).ok()?.into_inner();
    (target_draft == draft).then_some((contents, resolver))
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/keywords/discriminator/tests_210.rs"
));
