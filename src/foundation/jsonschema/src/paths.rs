//! Facilities for working with paths within schemas or validated instances.
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    fmt,
    sync::{Arc, OnceLock},
};

use referencing::{
    unescape_segment, write_escaped_str, write_index, JsonPointerNode, JsonPointerSegment,
};

use crate::keywords::Keyword;

/// A location segment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocationSegment<'a> {
    /// Property name within a JSON object.
    Property(Cow<'a, str>),
    /// JSON Schema keyword.
    Index(usize),
}

impl fmt::Display for LocationSegment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocationSegment::Property(property) => f.write_str(property),
            LocationSegment::Index(idx) => f.write_str(itoa::Buffer::new().format(*idx)),
        }
    }
}

/// A lazily constructed location within a JSON instance.
///
/// [`LazyLocation`] builds a path incrementally during JSON Schema validation without allocating
/// memory until required by storing each segment on the stack.
pub type LazyLocation<'a, 'b> = JsonPointerNode<'a, 'b>;

/// Cached empty location - very common for root-level errors.
static EMPTY_LOCATION: OnceLock<Location> = OnceLock::new();

/// Cached single-index paths for indices 0-15.
/// Cloning these is just an atomic increment (Arc clone).
static CACHED_INDEX_PATHS: OnceLock<[Location; 16]> = OnceLock::new();

thread_local! {
    static LOCATION_BUFFER: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Build an `Arc<str>` in a single allocation.
///
/// `Arc::from(String)` would allocate a second time and copy, so format into a reused
/// buffer instead. `write` must not build a location itself - that panics on the borrow.
#[inline]
pub(crate) fn build_arc_str(capacity: usize, write: impl FnOnce(&mut String)) -> Arc<str> {
    LOCATION_BUFFER.with_borrow_mut(|buffer| {
        buffer.clear();
        buffer.reserve(capacity);
        write(buffer);
        Arc::from(buffer.as_str())
    })
}

#[inline]
fn build_location(capacity: usize, write: impl FnOnce(&mut String)) -> Location {
    Location(build_arc_str(capacity, write))
}

#[inline]
fn get_cached_index_paths() -> &'static [Location; 16] {
    CACHED_INDEX_PATHS.get_or_init(|| {
        std::array::from_fn(|i| {
            let mut itoa_buf = itoa::Buffer::new();
            let s = itoa_buf.format(i);
            let mut buffer = String::with_capacity(1 + s.len());
            buffer.push('/');
            buffer.push_str(s);
            Location(Arc::from(buffer))
        })
    })
}

impl<'a> From<&'a LazyLocation<'_, '_>> for Location {
    fn from(value: &'a LazyLocation<'_, '_>) -> Self {
        const STACK_CAPACITY: usize = 16;

        // Fast path: empty location
        if value.parent().is_none() {
            return Location::new();
        }

        // Fast path: single index segment (very common for array validation)
        // Use cached locations for indices 0-15 to avoid allocation
        if let Some(parent) = value.parent() {
            if parent.parent().is_none() {
                if let JsonPointerSegment::Index(idx) = value.segment() {
                    if *idx < 16 {
                        return get_cached_index_paths()[*idx].clone();
                    }
                    // Single index > 15: compute directly
                    let mut idx_buffer = itoa::Buffer::new();
                    let idx = idx_buffer.format(*idx);
                    return build_location(1 + idx.len(), |buffer| {
                        buffer.push('/');
                        buffer.push_str(idx);
                    });
                }
            }
        }

        // General path: multi-segment

        // First pass: count segments and calculate string capacity
        let mut capacity = 0;
        let mut string_capacity = 0;
        let mut head = value;

        while let Some(next) = head.parent() {
            capacity += 1;
            string_capacity += match head.segment() {
                JsonPointerSegment::Key(property) => property.len() + 1,
                JsonPointerSegment::Index(idx) => idx.checked_ilog10().unwrap_or(0) as usize + 2,
            };
            head = next;
        }

        build_location(string_capacity, |buffer| {
            if capacity <= STACK_CAPACITY {
                // Stack-allocated storage with references - no cloning needed
                let mut stack_segments: [Option<&JsonPointerSegment<'_>>; STACK_CAPACITY] =
                    [None; STACK_CAPACITY];
                let mut idx = 0;
                head = value;

                if head.parent().is_some() {
                    stack_segments[idx] = Some(head.segment());
                    idx += 1;
                }

                while let Some(next) = head.parent() {
                    head = next;
                    if head.parent().is_some() {
                        stack_segments[idx] = Some(head.segment());
                        idx += 1;
                    }
                }

                // Format in reverse order
                for segment in stack_segments[..idx].iter().rev().flatten() {
                    buffer.push('/');
                    match segment {
                        JsonPointerSegment::Key(property) => {
                            write_escaped_str(buffer, property);
                        }
                        JsonPointerSegment::Index(idx) => {
                            write_index(buffer, *idx);
                        }
                    }
                }
            } else {
                // Heap-allocated fallback for deep paths (>16 segments)
                let mut segments: Vec<&JsonPointerSegment<'_>> = Vec::with_capacity(capacity);
                head = value;

                if head.parent().is_some() {
                    segments.push(head.segment());
                }

                while let Some(next) = head.parent() {
                    head = next;
                    if head.parent().is_some() {
                        segments.push(head.segment());
                    }
                }

                for segment in segments.iter().rev() {
                    buffer.push('/');
                    match segment {
                        JsonPointerSegment::Key(property) => {
                            write_escaped_str(buffer, property);
                        }
                        JsonPointerSegment::Index(idx) => {
                            write_index(buffer, *idx);
                        }
                    }
                }
            }
        })
    }
}

/// Tracks `$ref` traversals during validation for evaluation path computation.
///
/// This is a stack-allocated linked list that gets pushed when crossing `$ref` boundaries.
/// Each entry stores the **suffix** of the `$ref` location (path relative to its resource base),
/// which is precomputed at compile time.
///
/// Use `Option<&RefTracker>` throughout the validation code:
/// - `None` means no `$ref` has been traversed yet
/// - `Some(&tracker)` means at least one `$ref` has been crossed
///
/// # Example: Single $ref
///
/// ```text
/// Schema:
/// {
///   "properties": {
///     "user": { "$ref": "#/$defs/Person" }
///   },
///   "$defs": {
///     "Person": { "type": "object" }
///   }
/// }
///
/// Instance: { "user": "not-an-object" }
///
/// At the "type" validator:
///   tracker.prefix() = /properties/user/$ref
///   validator.suffix = /type
///   tracker  = /properties/user/$ref/type
/// ```
///
/// # Example: Nested $refs
///
/// ```text
/// Schema:
/// {
///   "properties": {
///     "order": { "$ref": "#/$defs/Order" }
///   },
///   "$defs": {
///     "Order": {
///       "properties": {
///         "item": { "$ref": "#/$defs/Item" }
///       }
///     },
///     "Item": { "type": "string" }
///   }
/// }
///
/// Instance: { "order": { "item": 123 } }
///
/// At the "type" validator:
///   tracker has two entries:
///     1. suffix = /properties/order/$ref (from root resource)
///     2. suffix = /properties/item/$ref  (from $defs/Order resource)
///   tracker.prefix() = /properties/order/$ref/properties/item/$ref
///   validator.suffix = /type
///   tracker  = /properties/order/$ref/properties/item/$ref/type
/// ```
#[derive(Debug)]
pub(crate) struct RefTracker<'a> {
    /// Path of the `$ref` keyword relative to its resource base.
    /// E.g., `/properties/user/$ref` (not the full canonical path).
    suffix: &'a Location,
    /// The resource base of the `$ref` target.
    /// Used to compute validator suffixes at runtime.
    /// E.g., `/$defs/Person` when `$ref` points to `#/$defs/Person`.
    target_base: &'a Location,
    /// Parent tracker for nested `$ref`s. `None` for the first `$ref` in the chain.
    parent: Option<&'a RefTracker<'a>>,
    /// Cached joined prefix (computed once on first access).
    cached_prefix: std::sync::OnceLock<Location>,
    /// Identifier of the canonical prefix in the current evaluation cache. Zero means uncached.
    cached_prefix_identifier: Cell<usize>,
}

impl<'a> RefTracker<'a> {
    /// Create a new tracker for a `$ref` traversal.
    ///
    /// # Arguments
    /// - `suffix`: The `$ref` keyword's path relative to its resource base
    ///   (precomputed at compile time via `ctx.suffix()`)
    /// - `target_base`: The resource base of the `$ref` target
    ///   (e.g., `/$defs/Person` when `$ref` points to `#/$defs/Person`)
    /// - `parent`: The parent tracker, or `None` if this is the first `$ref`
    #[inline]
    #[must_use]
    pub(crate) fn new(
        suffix: &'a Location,
        target_base: &'a Location,
        parent: Option<&'a RefTracker<'a>>,
    ) -> Self {
        RefTracker {
            suffix,
            target_base,
            parent,
            cached_prefix: std::sync::OnceLock::new(),
            cached_prefix_identifier: Cell::new(0),
        }
    }

    /// Get the joined prefix of all `$ref` suffixes.
    ///
    /// Computed once on first access, then cached.
    #[inline]
    pub(crate) fn prefix(&self) -> &Location {
        if let Some(prefix) = self.cached_prefix.get() {
            return prefix;
        }
        // Fill uncached ancestors root-first, so a long `$ref` chain joins in a loop, not a recursion.
        let mut uncached = Vec::new();
        let mut ancestor = self.parent;
        while let Some(tracker) = ancestor {
            if tracker.cached_prefix.get().is_some() {
                break;
            }
            uncached.push(tracker);
            ancestor = tracker.parent;
        }
        for tracker in uncached.into_iter().rev() {
            tracker
                .cached_prefix
                .get_or_init(|| tracker.compute_prefix());
        }
        self.cached_prefix.get_or_init(|| self.compute_prefix())
    }

    /// Compute the suffix for a canonical location by stripping the `target_base`.
    ///
    /// E.g., for location `/$defs/Person/type` with `target_base` `/$defs/Person`,
    /// returns `/type`.
    #[inline]
    pub(crate) fn compute_suffix(&self, location: &Location) -> Location {
        let suffix = location
            .as_str()
            .strip_prefix(self.target_base.as_str())
            .unwrap_or(location.as_str());
        Location::from_escaped(suffix)
    }

    /// Compute the evaluation path for a validator with the given canonical location.
    ///
    /// This is: `prefix + (location - target_base)`
    #[inline]
    pub(crate) fn evaluation_path_with_prefix(
        &self,
        prefix: &Location,
        location: &Location,
    ) -> Location {
        let suffix = location
            .as_str()
            .strip_prefix(self.target_base.as_str())
            .unwrap_or(location.as_str());
        prefix.join_raw_suffix(suffix)
    }

    #[inline]
    pub(crate) fn parent(&self) -> Option<&RefTracker<'a>> {
        self.parent
    }

    #[inline]
    pub(crate) fn suffix(&self) -> &Location {
        self.suffix
    }

    #[inline]
    pub(crate) fn cached_prefix_identifier(&self) -> Option<usize> {
        match self.cached_prefix_identifier.get() {
            0 => None,
            identifier => Some(identifier),
        }
    }

    #[inline]
    pub(crate) fn cache_prefix_identifier(&self, identifier: usize) {
        debug_assert_ne!(identifier, 0);
        self.cached_prefix_identifier.set(identifier);
    }

    fn compute_prefix(&self) -> Location {
        match self.parent {
            None => self.suffix.clone(),
            Some(parent) => parent.prefix().join_raw_suffix(self.suffix.as_str()),
        }
    }

    /// Capture evaluation path state at error creation time.
    ///
    /// Returns a lazy evaluation path that defers computation until needed.
    #[inline]
    pub(crate) fn capture(&self, location: &Location) -> LazyEvaluationPath {
        LazyEvaluationPath::Deferred {
            prefix: self.prefix().clone(),
            suffix: self.compute_suffix(location),
            cached: OnceLock::new(),
        }
    }
}

/// Compute the evaluation path, using the tracker if present.
#[inline]
pub(crate) fn evaluation_path(
    tracker: Option<&RefTracker<'_>>,
    location: &Location,
    ctx: &mut crate::validator::ValidationContext,
) -> Location {
    match tracker {
        None => location.clone(),
        Some(t) => ctx.evaluation_path(t, location),
    }
}

/// Capture evaluation path state at error creation time.
#[inline]
pub(crate) fn capture_evaluation_path(
    tracker: Option<&RefTracker<'_>>,
    location: &Location,
) -> LazyEvaluationPath {
    match tracker {
        None => LazyEvaluationPath::SameAsSchemaPath,
        Some(t) => t.capture(location),
    }
}

/// Lazily-computed evaluation path stored in validation errors.
///
/// # Why lazy?
///
/// Validators like `anyOf` collect errors from all branches but discard them
/// if any branch succeeds. Eagerly computing evaluation paths for discarded
/// errors wastes CPU cycles. We defer the string join until the error is
/// actually displayed.
///
/// # Example 1: No $ref traversal
///
/// ```text
/// Schema: { "properties": { "name": { "type": "string" } } }
/// Instance: { "name": 123 }
///
/// schema_path = tracker = /properties/name/type
///
/// We store: SameAsSchemaPath (zero extra allocation)
/// ```
///
/// # Example 2: Single $ref
///
/// ```text
/// Schema:
/// {
///   "properties": {
///     "user": { "$ref": "#/$defs/Person" }
///   },
///   "$defs": {
///     "Person": { "type": "object" }
///   }
/// }
/// Instance: { "user": "not-an-object" }
///
/// schema_path     = /$defs/Person/type  (canonical location, no $ref)
/// tracker = /properties/user/$ref/type (actual traversal path)
///
/// We store:
///   - prefix: /properties/user/$ref  (from RefTracker)
///   - suffix: /type                  (precomputed at compile time)
///
/// On access: tracker = prefix + suffix
/// ```
///
/// # Example 3: Nested $refs
///
/// ```text
/// Schema:
/// {
///   "properties": {
///     "order": { "$ref": "#/$defs/Order" }
///   },
///   "$defs": {
///     "Order": {
///       "properties": {
///         "item": { "$ref": "#/$defs/Item" }
///       }
///     },
///     "Item": {
///       "properties": {
///         "price": { "type": "number" }
///       }
///     }
///   }
/// }
/// Instance: { "order": { "item": { "price": "free" } } }
///
/// schema_path     = /$defs/Item/properties/price/type
/// tracker = /properties/order/$ref/properties/item/$ref/properties/price/type
///
/// We store:
///   - prefix: /properties/order/$ref/properties/item/$ref  (from RefTracker chain)
///   - suffix: /properties/price/type                       (computed at error creation)
/// ```
#[derive(Debug)]
pub(crate) enum LazyEvaluationPath {
    /// No `$ref` traversal - `tracker` equals `schema_path`.
    /// Zero extra storage; we reference `ValidationError::schema_path` directly.
    SameAsSchemaPath,

    /// Already computed evaluation path (e.g., from `into_owned`).
    Computed(Location),

    /// `$ref` was traversed - lazily compute `prefix + suffix`.
    Deferred {
        /// Chain of `$ref` locations (e.g., `/properties/user/$ref`)
        prefix: Location,
        /// Path relative to innermost `$ref` target (e.g., `/type`)
        suffix: Location,
        /// Cached result of `prefix.join_raw_suffix(&suffix)`
        cached: OnceLock<Location>,
    },
}

impl Clone for LazyEvaluationPath {
    fn clone(&self) -> Self {
        match self {
            LazyEvaluationPath::SameAsSchemaPath => LazyEvaluationPath::SameAsSchemaPath,
            LazyEvaluationPath::Computed(loc) => LazyEvaluationPath::Computed(loc.clone()),
            LazyEvaluationPath::Deferred {
                prefix,
                suffix,
                cached,
            } => {
                let new_cached = OnceLock::new();
                if let Some(val) = cached.get() {
                    let _ = new_cached.set(val.clone());
                }
                LazyEvaluationPath::Deferred {
                    prefix: prefix.clone(),
                    suffix: suffix.clone(),
                    cached: new_cached,
                }
            }
        }
    }
}

impl From<Location> for LazyEvaluationPath {
    /// Convert a computed Location to `LazyEvaluationPath`.
    ///
    /// Used when an evaluation path has been computed (e.g., via `into_owned`) and needs
    /// to be preserved when creating a new error.
    #[inline]
    fn from(location: Location) -> Self {
        LazyEvaluationPath::Computed(location)
    }
}

impl LazyEvaluationPath {
    /// Resolve the evaluation path, computing it if necessary.
    ///
    /// For `SameAsSchemaPath`, caller must pass the `schema_path` reference.
    #[inline]
    #[must_use]
    pub(crate) fn resolve<'a>(&'a self, schema_path: &'a Location) -> &'a Location {
        match self {
            LazyEvaluationPath::SameAsSchemaPath => schema_path,
            LazyEvaluationPath::Computed(loc) => loc,
            LazyEvaluationPath::Deferred {
                prefix,
                suffix,
                cached,
            } => cached.get_or_init(|| prefix.join_raw_suffix(suffix.as_str())),
        }
    }

    /// Consume self and return the owned evaluation path.
    #[inline]
    #[must_use]
    pub(crate) fn into_owned(self, schema_path: Location) -> Location {
        match self {
            LazyEvaluationPath::SameAsSchemaPath => schema_path,
            LazyEvaluationPath::Computed(loc) => loc,
            LazyEvaluationPath::Deferred {
                prefix,
                suffix,
                cached,
            } => cached
                .into_inner()
                .unwrap_or_else(|| prefix.join_raw_suffix(suffix.as_str())),
        }
    }
}

impl<'a> From<&'a Keyword> for LocationSegment<'a> {
    fn from(value: &'a Keyword) -> Self {
        match value {
            Keyword::Builtin(k) => LocationSegment::Property(k.as_str().into()),
            Keyword::Custom(s) => LocationSegment::Property(Cow::Borrowed(s)),
        }
    }
}

impl<'a> From<&'a str> for LocationSegment<'a> {
    #[inline]
    fn from(value: &'a str) -> LocationSegment<'a> {
        LocationSegment::Property(Cow::Borrowed(value))
    }
}

impl<'a> From<&'a String> for LocationSegment<'a> {
    #[inline]
    fn from(value: &'a String) -> LocationSegment<'a> {
        LocationSegment::Property(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, str>> for LocationSegment<'a> {
    #[inline]
    fn from(value: Cow<'a, str>) -> LocationSegment<'a> {
        LocationSegment::Property(value)
    }
}

impl From<usize> for LocationSegment<'_> {
    #[inline]
    fn from(value: usize) -> Self {
        LocationSegment::Index(value)
    }
}

impl<'a> From<LocationSegment<'a>> for JsonPointerSegment<'a> {
    fn from(value: LocationSegment<'a>) -> Self {
        match value {
            LocationSegment::Property(property) => JsonPointerSegment::Key(property),
            LocationSegment::Index(idx) => JsonPointerSegment::Index(idx),
        }
    }
}

/// A cheap to clone JSON pointer that represents location with a JSON value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Location(Arc<str>);

impl serde::Serialize for Location {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl Location {
    /// Create a new, empty `Location`.
    ///
    /// Returns a cached instance to avoid allocation.
    #[must_use]
    pub fn new() -> Self {
        EMPTY_LOCATION.get_or_init(|| Self(Arc::from(""))).clone()
    }

    pub(crate) fn from_escaped(escaped: &str) -> Self {
        Self(Arc::from(escaped))
    }

    /// Append a raw JSON pointer suffix (already escaped).
    /// This is more efficient than multiple `join` calls when the suffix
    /// is already a valid JSON pointer path.
    #[must_use]
    pub(crate) fn join_raw_suffix(&self, suffix: &str) -> Self {
        if suffix.is_empty() {
            return self.clone();
        }
        let parent = &self.0;
        build_location(parent.len() + suffix.len(), |buffer| {
            buffer.push_str(parent);
            buffer.push_str(suffix);
        })
    }

    /// Append a segment taken straight from a [`LazyLocation`] frame.
    #[must_use]
    pub(crate) fn join_pointer_segment(&self, segment: &JsonPointerSegment<'_>) -> Self {
        let parent = &self.0;
        match segment {
            JsonPointerSegment::Key(property) => {
                build_location(parent.len() + property.len() + 1, |buffer| {
                    buffer.push_str(parent);
                    buffer.push('/');
                    write_escaped_str(buffer, property);
                })
            }
            JsonPointerSegment::Index(index) => build_location(parent.len() + 2, |buffer| {
                buffer.push_str(parent);
                buffer.push('/');
                write_index(buffer, *index);
            }),
        }
    }

    #[must_use]
    pub fn join<'a>(&self, segment: impl Into<LocationSegment<'a>>) -> Self {
        let parent = &self.0;
        match segment.into() {
            LocationSegment::Property(property) => {
                build_location(parent.len() + property.len() + 1, |buffer| {
                    buffer.push_str(parent);
                    buffer.push('/');
                    write_escaped_str(buffer, &property);
                })
            }
            LocationSegment::Index(idx) => {
                let mut idx_buffer = itoa::Buffer::new();
                let idx = idx_buffer.format(idx);
                build_location(parent.len() + idx.len() + 1, |buffer| {
                    buffer.push_str(parent);
                    buffer.push('/');
                    buffer.push_str(idx);
                })
            }
        }
    }
    /// Address of the backing string, stable while the owner is alive.
    #[must_use]
    pub(crate) fn as_ptr(&self) -> usize {
        Arc::as_ptr(&self.0).cast::<u8>() as usize
    }

    /// Get a clone of the inner `Arc<str>` representing the location.
    #[must_use]
    pub(crate) fn as_arc(&self) -> Arc<str> {
        Arc::clone(&self.0)
    }

    /// Get a string slice representing the location.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Whether this location points at the root of the document.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Get a byte slice representing the location.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[must_use]
    pub fn iter(&self) -> std::vec::IntoIter<LocationSegment<'_>> {
        <&Self as IntoIterator>::into_iter(self)
    }
    /// Iterate the segments without materializing them.
    pub fn segments(&self) -> impl Iterator<Item = LocationSegment<'_>> + Clone {
        self.as_str().split('/').filter(|p| !p.is_empty()).map(|p| {
            p.parse::<usize>().map_or_else(
                |_| LocationSegment::Property(unescape_segment(p)),
                LocationSegment::Index,
            )
        })
    }
}

impl Default for Location {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> IntoIterator for &'a Location {
    type Item = LocationSegment<'a>;
    type IntoIter = std::vec::IntoIter<LocationSegment<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments().collect::<Vec<_>>().into_iter()
    }
}

impl<'a> FromIterator<LocationSegment<'a>> for Location {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = LocationSegment<'a>>,
    {
        fn inner<'a, 'b, 'c, I>(path_iter: &mut I, location: &'b LazyLocation<'b, 'a>) -> Location
        where
            I: Iterator<Item = LocationSegment<'c>>,
        {
            let Some(path) = path_iter.next() else {
                return location.into();
            };
            let location = location.push(path);
            inner(path_iter, &location)
        }

        let loc = LazyLocation::default();
        inner(&mut iter.into_iter(), &loc)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/paths/tests_796.rs"));
