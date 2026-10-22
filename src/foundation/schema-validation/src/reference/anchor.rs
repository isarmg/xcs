//! Anchors identify sub-schemas within a document by name.
//!
//! JSON Schema defines two anchor flavors:
//! - [`Anchor::Default`]: a plain anchor (`$anchor`), resolved against the current base URI.
//! - [`Anchor::Dynamic`]: a dynamic anchor (`$dynamicAnchor`), which re-anchors to the
//!   outermost matching dynamic anchor found in the dynamic scope during resolution.
//!
//! [`AnchorIter`] avoids a heap allocation for the common case of 0–2 anchors per schema object.

use json_codec::Value;

use crate::reference::{Draft, Error, Resolved, Resolver, ResourceRef};

/// An anchor within a resource.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Anchor<'a> {
    Default {
        name: &'a str,
        resource: ResourceRef<'a>,
    },
    Dynamic {
        name: &'a str,
        resource: ResourceRef<'a>,
    },
}

impl<'a> Anchor<'a> {
    /// Anchor's name.
    #[inline]
    pub(crate) fn name(&self) -> &'a str {
        match self {
            Anchor::Default { name, .. } | Anchor::Dynamic { name, .. } => name,
        }
    }
}

impl<'r> Anchor<'r> {
    /// Get the resource for this anchor.
    pub(crate) fn resolve(&self, resolver: Resolver<'r>) -> Result<Resolved<'r>, Error> {
        match self {
            Anchor::Default { resource, .. } => Ok(Resolved::new(
                resource.contents(),
                resolver,
                resource.draft(),
            )),
            Anchor::Dynamic { name, resource } => {
                let scope = resolver.dynamic_scope();
                let mut last = *resource;
                let mut found_in = None;
                for uri in &scope {
                    match resolver.lookup_anchor(uri, name) {
                        Ok(anchor) => {
                            if let Anchor::Dynamic { resource, .. } = anchor {
                                last = resource;
                                found_in = Some(uri);
                            }
                        }
                        Err(Error::NoSuchAnchor { .. }) => {}
                        Err(err) => return Err(err),
                    }
                }
                // The anchor's own document is the base for references inside it
                let resolver = match found_in {
                    Some(uri) => resolver.at_uri(uri)?,
                    None => resolver,
                };
                Ok(Resolved::new(
                    last.contents(),
                    resolver.in_subresource(last)?,
                    last.draft(),
                ))
            }
        }
    }
}

/// An iterator over 0, 1, or 2 anchors — avoids a [`Vec`] allocation for the common case.
pub(crate) enum AnchorIter<'a> {
    Empty,
    One(Anchor<'a>),
    Two(Anchor<'a>, Anchor<'a>),
}

impl<'a> Iterator for AnchorIter<'a> {
    type Item = Anchor<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match std::mem::replace(self, AnchorIter::Empty) {
            AnchorIter::Empty => None,
            AnchorIter::One(anchor) => Some(anchor),
            AnchorIter::Two(first, second) => {
                *self = AnchorIter::One(second);
                Some(first)
            }
        }
    }
}

pub(crate) fn anchor(draft: Draft, contents: &Value) -> AnchorIter<'_> {
    let Some(schema) = contents.as_object() else {
        return AnchorIter::Empty;
    };

    // First check for top-level anchors
    let default_anchor =
        schema
            .get("$anchor")
            .and_then(Value::as_str)
            .map(|name| Anchor::Default {
                name,
                resource: ResourceRef::new(contents, draft),
            });

    let dynamic_anchor = schema
        .get("$dynamicAnchor")
        .and_then(Value::as_str)
        .map(|name| Anchor::Dynamic {
            name,
            resource: ResourceRef::new(contents, draft),
        });

    match (default_anchor, dynamic_anchor) {
        (Some(default), Some(dynamic)) => AnchorIter::Two(default, dynamic),
        (Some(default), None) => AnchorIter::One(default),
        (None, Some(dynamic)) => AnchorIter::One(dynamic),
        (None, None) => AnchorIter::Empty,
    }
}

pub(crate) fn anchor_2019(draft: Draft, contents: &Value) -> AnchorIter<'_> {
    match contents
        .as_object()
        .and_then(|schema| schema.get("$anchor"))
        .and_then(Value::as_str)
    {
        Some(name) => AnchorIter::One(Anchor::Default {
            name,
            resource: ResourceRef::new(contents, draft),
        }),
        None => AnchorIter::Empty,
    }
}

pub(crate) fn legacy_anchor_in_dollar_id(draft: Draft, contents: &Value) -> AnchorIter<'_> {
    match contents
        .as_object()
        .and_then(|schema| schema.get("$id"))
        .and_then(Value::as_str)
        .and_then(|id| id.strip_prefix('#'))
    {
        Some(id) => AnchorIter::One(Anchor::Default {
            name: id,
            resource: ResourceRef::new(contents, draft),
        }),
        None => AnchorIter::Empty,
    }
}

pub(crate) fn legacy_anchor_in_id(draft: Draft, contents: &Value) -> AnchorIter<'_> {
    match contents
        .as_object()
        .and_then(|schema| schema.get("id"))
        .and_then(Value::as_str)
        .and_then(|id| id.strip_prefix('#'))
    {
        Some(id) => AnchorIter::One(Anchor::Default {
            name: id,
            resource: ResourceRef::new(contents, draft),
        }),
        None => AnchorIter::Empty,
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/referencing/unit/src/anchor/tests_174.rs"
));
