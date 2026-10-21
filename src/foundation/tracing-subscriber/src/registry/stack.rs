use alloc::vec::Vec;
pub(crate) use tracing_core::span::Id;

#[derive(Debug)]
struct ContextId {
    id: Id,
    duplicate: bool,
}

/// `SpanStack` tracks what spans are currently executing on a thread-local basis.
///
/// A "separate current span" for each thread is a semantic choice, as each span
/// can be executing in a different thread.
#[derive(Debug, Default)]
pub(crate) struct SpanStack {
    stack: Vec<ContextId>,
}

impl SpanStack {
    #[inline]
    pub(super) fn push(&mut self, id: Id) {
        let duplicate = self.stack.iter().any(|i| i.id == id);
        self.stack.push(ContextId { id, duplicate });
    }

    #[inline]
    pub(super) fn pop(&mut self, expected_id: &Id) -> bool {
        if let Some((idx, _)) = self
            .stack
            .iter()
            .enumerate()
            .rev()
            .find(|(_, ctx_id)| ctx_id.id == *expected_id)
        {
            let ContextId { id: _, duplicate } = self.stack.remove(idx);
            return !duplicate;
        }
        false
    }

    #[inline]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Id> {
        self.stack
            .iter()
            .rev()
            .filter_map(|ContextId { id, duplicate }| if !*duplicate { Some(id) } else { None })
    }

    #[inline]
    pub(crate) fn current(&self) -> Option<&Id> {
        self.iter().next()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/tracing-subscriber/unit/src/registry/stack/tests_55.rs"));
