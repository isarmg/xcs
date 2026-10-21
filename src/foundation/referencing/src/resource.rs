use std::borrow::Cow;

use json_codec::Value;

use crate::{Draft, Error, Resolved, Resolver, Segments};

/// An owned document with a concrete interpretation under a JSON Schema specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    contents: Value,
    draft: Draft,
}

impl Resource {
    #[inline]
    pub(crate) fn new(contents: Value, draft: Draft) -> Self {
        Self { contents, draft }
    }
    #[inline]
    pub(crate) fn into_inner(self) -> (Draft, Value) {
        (self.draft, self.contents)
    }
    /// Resource contents.
    #[must_use]
    #[inline]
    pub fn contents(&self) -> &Value {
        &self.contents
    }
    /// JSON Schema draft under which this contents is interpreted.
    #[must_use]
    #[inline]
    pub fn draft(&self) -> Draft {
        self.draft
    }
    /// Create a resource with automatically detecting specification which applies to the contents.
    ///
    /// Unknown `$schema` values are treated as `Draft::Unknown`.
    #[must_use]
    #[inline]
    pub fn from_contents(contents: Value) -> Resource {
        Draft::default().detect(&contents).create_resource(contents)
    }
}

/// A borrowed document with a concrete interpretation under a JSON Schema specification.
#[derive(Debug, Clone, Copy)]
pub struct ResourceRef<'a> {
    contents: &'a Value,
    draft: Draft,
}

impl<'a> ResourceRef<'a> {
    #[must_use]
    #[inline]
    pub fn new(contents: &'a Value, draft: Draft) -> Self {
        Self { contents, draft }
    }
    #[must_use]
    #[inline]
    pub fn contents(&self) -> &'a Value {
        self.contents
    }
    #[must_use]
    #[inline]
    pub fn draft(&self) -> Draft {
        self.draft
    }

    /// Create a resource-ref with automatically detecting specification which applies to the contents.
    ///
    /// Unknown `$schema` values are treated as `Draft::Unknown`.
    #[must_use]
    #[inline]
    pub fn from_contents(contents: &'a Value) -> Self {
        let draft = Draft::default().detect(contents);
        Self::new(contents, draft)
    }

    #[must_use]
    #[inline]
    pub fn id(&self) -> Option<&str> {
        self.draft
            .id_of(self.contents)
            .map(|id| id.trim_end_matches('#'))
    }
}

impl<'r> ResourceRef<'r> {
    pub(crate) fn pointer(
        self,
        pointer: &str,
        mut resolver: Resolver<'r>,
    ) -> Result<Resolved<'r>, Error> {
        // INVARIANT: Pointer always starts with `/`
        let mut contents = self.contents;
        let mut segments = Segments::new();
        let original_pointer = pointer;
        let pointer = percent_encoding::percent_decode_str(&pointer[1..])
            .decode_utf8()
            .map_err(|err| Error::invalid_percent_encoding(original_pointer, err))?;
        for segment in pointer.split('/') {
            if let Some(array) = contents.as_array() {
                let idx = segment
                    .parse::<usize>()
                    .map_err(|err| Error::invalid_array_index(original_pointer, segment, err))?;
                if let Some(next) = array.get(idx) {
                    contents = next;
                } else {
                    return Err(Error::pointer_to_nowhere(original_pointer));
                }
                segments.push(idx);
            } else {
                let segment = unescape_segment(segment);
                if let Some(next) = contents.get(segment.as_ref()) {
                    contents = next;
                } else {
                    return Err(Error::pointer_to_nowhere(original_pointer));
                }
                segments.push(segment);
            }
            let last = &resolver;
            let new_resolver = self.draft.maybe_in_subresource(
                &segments,
                &resolver,
                ResourceRef::new(contents, self.draft),
            )?;
            if new_resolver != *last {
                segments = Segments::new();
            }
            resolver = new_resolver;
        }
        Ok(Resolved::new(contents, resolver, self.draft))
    }
}

/// Unescape JSON Pointer path segments by converting `~1` to `/` and `~0` to `~`.
#[must_use]
pub fn unescape_segment(mut segment: &str) -> Cow<'_, str> {
    // Naively, checking for `~` and then replacing implies two passes
    // over the input buffer. First, search in the first `contains('~')` call
    // and then replacing `~1` & `~0` at once in a single pass.
    //
    // This implementation is ~3x faster than the naive one.
    //
    // **NOTE**: Heavily inspired by the implementation in `boon`:
    // `https://github.com/santhosh-tekuri/boon/blob/fb09df2db19be75c32c0970b4bdedf1655f5f943/src/util.rs#L31`
    let Some(mut tilde_idx) = segment.find('~') else {
        return Cow::Borrowed(segment);
    };

    let mut buffer = String::with_capacity(segment.len());
    loop {
        let (before, after) = segment.split_at(tilde_idx);
        buffer.push_str(before);
        segment = &after[1..];
        let next_char_size = match segment.chars().next() {
            Some('1') => {
                buffer.push('/');
                1
            }
            Some('0') => {
                buffer.push('~');
                1
            }
            Some(next) => {
                buffer.push('~');
                buffer.push(next);
                next.len_utf8()
            }
            None => {
                buffer.push('~');
                break;
            }
        };
        segment = &segment[next_char_size..];
        let Some(next_tilde_idx) = segment.find('~') else {
            buffer.push_str(segment);
            break;
        };
        tilde_idx = next_tilde_idx;
    }
    Cow::Owned(buffer)
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/referencing/unit/src/resource/tests_185.rs"));
