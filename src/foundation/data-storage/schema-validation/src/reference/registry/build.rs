//! BFS pipeline that processes pending resources into the prepared index.
//!
//! Entry points:
//! - [`index_resources`]: processes pending resources and returns a prepared index.
//! - [`build_index_from_stored`]: builds an index from pre-stored documents
//!   (used by the static [`super::SPECIFICATIONS`] registry).
//!
//! [`StoredResource`] holds borrowed (externally-owned, zero-copy), owned (retrieved),
//! or shared ([`Arc<Value>`]) documents uniformly.

use std::{collections::VecDeque, num::NonZeroUsize, sync::Arc};

use ahash::{AHashMap, AHashSet};
use fluent_uri::Uri;
use json_codec::Value;

use crate::reference::{
    cache::UriCache,
    meta::{metas_for_draft, META_SCHEMAS_ALL},
    path::JsonPointerSegment,
    pointer::{pointer, ParsedPointer, ParsedPointerSegment},
    uri, Draft, Error, JsonPointerNode, Retrieve,
};

use super::{index::Index, input::PendingResource};

#[derive(Debug)]
enum StoredValue<'a> {
    Borrowed(&'a Value),
    Owned(Value),
    Shared(Arc<Value>),
}

/// A schema document stored in the registry: borrowed from the caller, owned, or shared.
#[derive(Debug)]
pub(super) struct StoredResource<'a> {
    value: StoredValue<'a>,
    draft: Draft,
}

impl<'a> StoredResource<'a> {
    #[inline]
    pub(super) fn owned(value: Value, draft: Draft) -> Self {
        Self {
            value: StoredValue::Owned(value),
            draft,
        }
    }

    #[inline]
    pub(super) fn borrowed(value: &'a Value, draft: Draft) -> Self {
        Self {
            value: StoredValue::Borrowed(value),
            draft,
        }
    }

    #[inline]
    pub(super) fn shared(value: Arc<Value>, draft: Draft) -> Self {
        Self {
            value: StoredValue::Shared(value),
            draft,
        }
    }

    #[inline]
    pub(super) fn contents(&self) -> &Value {
        match &self.value {
            StoredValue::Borrowed(value) => value,
            StoredValue::Owned(value) => value,
            StoredValue::Shared(value) => value,
        }
    }

    #[inline]
    pub(super) fn borrowed_contents(&self) -> Option<&'a Value> {
        match &self.value {
            StoredValue::Borrowed(value) => Some(value),
            StoredValue::Owned(_) | StoredValue::Shared(_) => None,
        }
    }

    #[inline]
    pub(super) fn draft(&self) -> Draft {
        self.draft
    }
}

pub(super) type ResourceStore<'a> = AHashMap<Arc<Uri<String>>, Arc<StoredResource<'a>>>;
pub(super) type KnownResources = AHashSet<Uri<String>>;
type VisitedLocalRefs<'a> = AHashSet<(NonZeroUsize, &'a str)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ReferenceKind {
    DollarRef,
    DollarSchema,
}

/// An entry in the processing queue.
///
/// `pointer_path` is a JSON Pointer relative to the document root (`""` means root).
/// Local `$ref`s are always resolved against the document root.
struct CrawlTask {
    base_uri: Arc<Uri<String>>,
    document_root_uri: Arc<Uri<String>>,
    pointer_path: String,
    draft: Draft,
}

/// A deferred local `$ref` target.
///
/// Like [`CrawlTask`] but carries the pre-resolved value address (`schema_ptr`) obtained
/// for free during the `pointer()` call at push time. Used in [`process_deferred_refs`] to
/// skip already-visited targets without a second `pointer()` traversal.
struct PendingLocalRef {
    document_root_uri: Arc<Uri<String>>,
    pointer: String,
    schema_ptr: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisitedSchemaContext {
    schema_ptr: NonZeroUsize,
    base_ptr: NonZeroUsize,
    draft: Draft,
}

type DeferredTarget<'a> = (Arc<Uri<String>>, Draft, &'a Value);

/// Lifetime-free traversal state passed to external-resource collection helpers.
struct CrawlState {
    external: AHashSet<(String, Uri<String>, ReferenceKind)>,
    found_metaschema_ref: bool,
    /// Tracks schema/base/draft traversal contexts we have already processed.
    visited_schemas: AHashSet<VisitedSchemaContext>,
    deferred_refs: Vec<PendingLocalRef>,
}

impl CrawlState {
    fn new() -> Self {
        Self {
            external: AHashSet::new(),
            found_metaschema_ref: false,
            visited_schemas: AHashSet::new(),
            deferred_refs: Vec::new(),
        }
    }
}

struct BuildState<'a> {
    queue: VecDeque<CrawlTask>,
    custom_metaschemas: Vec<String>,
    index: Index<'a>,
    crawl: CrawlState,
}

impl BuildState<'_> {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            custom_metaschemas: Vec::new(),
            index: Index::default(),
            crawl: CrawlState::new(),
        }
    }
}

/// Result of resolving a `$id` against the current base URI.
struct ResolvedId {
    base: Arc<Uri<String>>,
    /// True when the resolved URI differs from the input base URI.
    uri_changed: bool,
}

pub(super) fn index_resources<'a>(
    pairs: impl IntoIterator<Item = (Uri<String>, PendingResource<'a>)>,
    retriever: &dyn Retrieve,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    draft_override: Option<Draft>,
) -> Result<(Vec<String>, Index<'a>), Error> {
    let mut state = BuildState::new();
    enqueue_resources(
        pairs,
        documents,
        known_resources,
        &mut state,
        draft_override,
    );
    fetch_and_index(
        &mut state,
        documents,
        known_resources,
        resolution_cache,
        draft_override.unwrap_or_default(),
        retriever,
    )?;
    Ok((state.custom_metaschemas, state.index))
}

#[cfg(feature = "resolve-async")]
pub(super) async fn index_resources_async<'a>(
    pairs: impl IntoIterator<Item = (Uri<String>, PendingResource<'a>)>,
    retriever: &dyn crate::reference::AsyncRetrieve,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    draft_override: Option<Draft>,
) -> Result<(Vec<String>, Index<'a>), Error> {
    let mut state = BuildState::new();
    enqueue_resources(
        pairs,
        documents,
        known_resources,
        &mut state,
        draft_override,
    );
    fetch_and_index_async(
        &mut state,
        documents,
        known_resources,
        resolution_cache,
        draft_override.unwrap_or_default(),
        retriever,
    )
    .await?;
    Ok((state.custom_metaschemas, state.index))
}

/// Build prepared local index data for all documents already in `documents`.
pub(super) fn build_index_from_stored<'a>(
    documents: &ResourceStore<'a>,
    resolution_cache: &mut UriCache,
) -> Result<Index<'a>, Error> {
    let mut state = BuildState::new();
    let mut known_resources = KnownResources::default();

    for (doc_uri, document) in documents {
        known_resources.insert((**doc_uri).clone());
        state.index.register_document(doc_uri, document);
    }

    for (doc_uri, document) in documents {
        let document_root = document
            .borrowed_contents()
            .expect("build_index_from_stored is only called with borrowed documents");
        let mut visited_local_refs = VisitedLocalRefs::new();
        crawl_borrowed_at(
            Arc::clone(doc_uri),
            doc_uri,
            document_root,
            "",
            document.draft(),
            &mut state,
            &mut known_resources,
            resolution_cache,
            &mut visited_local_refs,
        )?;
    }
    Ok(state.index)
}

pub(super) fn validate_custom_metaschemas(
    custom_metaschemas: &[String],
    known_resources: &KnownResources,
) -> Result<(), Error> {
    for schema_uri in custom_metaschemas {
        match uri::from_str(schema_uri) {
            Ok(mut meta_uri) => {
                meta_uri.set_fragment(None);
                if !known_resources.contains(&meta_uri) {
                    return Err(Error::unknown_specification(schema_uri));
                }
            }
            Err(_) => {
                return Err(Error::unknown_specification(schema_uri));
            }
        }
    }
    Ok(())
}

/// Shared sync processing loop used during registry preparation. After the
/// initial input has been ingested into `state`, this function drives the
/// BFS-fetch cycle until all reachable external resources have been retrieved,
/// then handles meta-schema injection and runs a final queue pass.
fn fetch_and_index<'a>(
    state: &mut BuildState<'a>,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    default_draft: Draft,
    retriever: &dyn Retrieve,
) -> Result<(), Error> {
    while !(state.queue.is_empty() && state.crawl.external.is_empty()) {
        flush_pending(state, documents, known_resources, resolution_cache)?;
        fetch_external_resources(state, documents, known_resources, default_draft, retriever)?;
    }

    finalize_index(
        state,
        documents,
        known_resources,
        resolution_cache,
        default_draft,
    )
}

/// Shared async processing loop used during registry preparation. Batches
/// concurrent external retrievals with `join_all` and otherwise mirrors
/// [`fetch_and_index`].
#[cfg(feature = "resolve-async")]
async fn fetch_and_index_async<'a>(
    state: &mut BuildState<'a>,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    default_draft: Draft,
    retriever: &dyn crate::reference::AsyncRetrieve,
) -> Result<(), Error> {
    while !(state.queue.is_empty() && state.crawl.external.is_empty()) {
        flush_pending(state, documents, known_resources, resolution_cache)?;
        fetch_external_resources_async(state, documents, known_resources, default_draft, retriever)
            .await?;
    }

    finalize_index(
        state,
        documents,
        known_resources,
        resolution_cache,
        default_draft,
    )
}

/// Convert resources into stored documents, register them with the
/// index, and enqueue them as the starting set for BFS traversal.
///
/// `draft` forces a specific draft for all resources; `None` means auto-detect per resource.
/// Resources are added to `known_resources` here so the retriever does not re-fetch them
/// during the BFS loop.
fn enqueue_resources<'a>(
    pairs: impl IntoIterator<Item = (Uri<String>, PendingResource<'a>)>,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    state: &mut BuildState<'a>,
    draft: Option<Draft>,
) {
    for (uri, resource) in pairs {
        let key = Arc::new(uri);
        let (draft, document) = match resource {
            PendingResource::Value(value) => {
                let draft = draft.unwrap_or_else(|| Draft::default().detect(&value));
                (draft, StoredResource::owned(value, draft))
            }
            PendingResource::ValueRef(value) => {
                let draft = draft.unwrap_or_else(|| Draft::default().detect(value));
                (draft, StoredResource::borrowed(value, draft))
            }
            PendingResource::SharedValue(value) => {
                let draft = draft.unwrap_or_else(|| Draft::default().detect(&value));
                (draft, StoredResource::shared(value, draft))
            }
            PendingResource::Resource(resource) => {
                let (draft, contents) = resource.into_inner();
                (draft, StoredResource::owned(contents, draft))
            }
            PendingResource::ResourceRef(resource) => {
                let draft = resource.draft();
                (draft, StoredResource::borrowed(resource.contents(), draft))
            }
        };
        let document = Arc::new(document);

        documents.insert(Arc::clone(&key), Arc::clone(&document));
        known_resources.insert((*key).clone());
        state.index.register_document(&key, &document);

        // Draft::Unknown means the resource declared a custom $schema; collect its URI
        // for post-build validation that a matching meta-schema was registered.
        if draft == Draft::Unknown {
            if let Some(meta) = document
                .contents()
                .as_object()
                .and_then(|obj| obj.get("$schema"))
                .and_then(|schema| schema.as_str())
            {
                state.custom_metaschemas.push(meta.to_string());
            }
        }

        state.queue.push_back(CrawlTask {
            base_uri: Arc::clone(&key),
            document_root_uri: key,
            pointer_path: String::new(),
            draft,
        });
    }
}

fn drain_queue<'r>(
    state: &mut BuildState<'r>,
    documents: &ResourceStore<'r>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
) -> Result<(), Error> {
    while let Some(CrawlTask {
        base_uri: base,
        document_root_uri: root_uri,
        pointer_path,
        draft,
    }) = state.queue.pop_front()
    {
        let Some(document) = documents.get(&root_uri) else {
            continue;
        };
        if let Some(document_root) = document.borrowed_contents() {
            let mut visited_local_refs = VisitedLocalRefs::new();
            crawl_borrowed_at(
                base,
                &root_uri,
                document_root,
                &pointer_path,
                draft,
                state,
                known_resources,
                resolution_cache,
                &mut visited_local_refs,
            )?;
            continue;
        }
        crawl_owned_at(
            base,
            &root_uri,
            document,
            &pointer_path,
            draft,
            state,
            known_resources,
            resolution_cache,
        )?;
    }
    Ok(())
}

fn flush_pending<'a>(
    state: &mut BuildState<'a>,
    documents: &ResourceStore<'a>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
) -> Result<(), Error> {
    let mut visited_local_refs = VisitedLocalRefs::new();
    drain_queue(state, documents, known_resources, resolution_cache)?;
    process_deferred_refs(state, documents, resolution_cache, &mut visited_local_refs)?;
    Ok(())
}

/// Process deferred local-ref targets collected during the main traversal.
///
/// Called after `drain_queue` finishes so that all subresource nodes are already in
/// `visited_schemas`. Targets that were visited by the main BFS (e.g. `#/definitions/Foo`
/// under a JSON Schema keyword) are skipped in O(1) via the pre-stored value address,
/// avoiding a redundant `pointer()` traversal. Non-subresource targets
/// (e.g. `#/components/schemas/Foo`) are still fully traversed. New deferred entries
/// added during traversal are also processed iteratively until none remain.
fn process_deferred_refs<'a>(
    state: &mut BuildState<'_>,
    documents: &'a ResourceStore<'a>,
    resolution_cache: &mut UriCache,
    visited_local_refs: &mut VisitedLocalRefs<'a>,
) -> Result<(), Error> {
    while !state.crawl.deferred_refs.is_empty() {
        let batch = std::mem::take(&mut state.crawl.deferred_refs);
        for PendingLocalRef {
            document_root_uri: doc_key,
            pointer: pointer_path,
            schema_ptr,
        } in batch
        {
            let Some(document) = documents.get(&doc_key) else {
                continue;
            };
            let root = document.contents();
            let Some((effective_base_uri, draft, contents)) = resolve_deferred_target(
                &doc_key,
                root,
                &pointer_path,
                document.draft(),
                resolution_cache,
            )?
            else {
                continue;
            };

            // Fast path: if this target was already visited by the main BFS traversal
            // (e.g. a `#/definitions/Foo` that `walk_children_with_path` descended into),
            // all its subresources were processed and `record_refs_from_object` was already
            // called on each — skip without a redundant `pointer()` traversal.
            if state
                .crawl
                .visited_schemas
                .contains(&visited_schema_context_from_ptr(
                    &effective_base_uri,
                    draft,
                    schema_ptr,
                ))
            {
                continue;
            }
            record_refs_recursive(
                &effective_base_uri,
                root,
                contents,
                &mut state.crawl,
                resolution_cache,
                draft,
                &doc_key,
                visited_local_refs,
            )?;
        }
    }
    Ok(())
}

/// Inject meta-schemas if referenced, then run a final queue pass to index them.
fn finalize_index<'a>(
    state: &mut BuildState<'a>,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    default_draft: Draft,
) -> Result<(), Error> {
    inject_metaschemas(
        state.crawl.found_metaschema_ref,
        documents,
        known_resources,
        default_draft,
        state,
    )?;

    if !state.queue.is_empty() {
        let mut visited_local_refs = VisitedLocalRefs::new();
        drain_queue(state, documents, known_resources, resolution_cache)?;
        process_deferred_refs(state, documents, resolution_cache, &mut visited_local_refs)?;
    }

    Ok(())
}

/// Fetch all pending external resources synchronously and enqueue the results.
fn fetch_external_resources<'a>(
    state: &mut BuildState<'a>,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    default_draft: Draft,
    retriever: &dyn Retrieve,
) -> Result<(), Error> {
    for (original, uri, kind) in state.crawl.external.drain() {
        let mut fragmentless = uri.clone();
        fragmentless.set_fragment(None);
        if !known_resources.contains(&fragmentless) {
            let retrieved = match retriever.retrieve(&fragmentless) {
                Ok(retrieved) => retrieved,
                Err(error) => {
                    handle_retrieve_error(&uri, &original, &fragmentless, error, kind)?;
                    continue;
                }
            };
            let (key, draft) = store_retrieved(
                retrieved,
                fragmentless,
                default_draft,
                documents,
                known_resources,
                &mut state.index,
                &mut state.custom_metaschemas,
            );
            enqueue_fragment_entry(&uri, &key, default_draft, documents, &mut state.queue);
            state.queue.push_back(CrawlTask {
                base_uri: Arc::clone(&key),
                document_root_uri: key,
                pointer_path: String::new(),
                draft,
            });
        }
    }
    Ok(())
}

/// Fetch all pending external resources concurrently and enqueue the results.
/// Groups requests by base URI and issues them in a single `join_all` batch.
#[cfg(feature = "resolve-async")]
async fn fetch_external_resources_async<'a>(
    state: &mut BuildState<'a>,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    default_draft: Draft,
    retriever: &dyn crate::reference::AsyncRetrieve,
) -> Result<(), Error> {
    type ExternalRefsByBase = AHashMap<Uri<String>, Vec<(String, Uri<String>, ReferenceKind)>>;

    if state.crawl.external.is_empty() {
        return Ok(());
    }

    let mut grouped = ExternalRefsByBase::new();
    for (original, uri, kind) in state.crawl.external.drain() {
        let mut fragmentless = uri.clone();
        fragmentless.set_fragment(None);
        if !known_resources.contains(&fragmentless) {
            grouped
                .entry(fragmentless)
                .or_default()
                .push((original, uri, kind));
        }
    }

    // Use grouped.keys() for futures (borrows) then grouped.into_iter() for results (consumes).
    // The map is not mutated between the two iterations, so zip order is stable.
    let results = futures::future::join_all(grouped.keys().map(|u| retriever.retrieve(u))).await;

    for ((fragmentless, refs), result) in grouped.into_iter().zip(results) {
        let retrieved = match result {
            Ok(retrieved) => retrieved,
            Err(error) => {
                if let Some((original, uri, kind)) = refs.into_iter().next() {
                    handle_retrieve_error(&uri, &original, &fragmentless, error, kind)?;
                }
                continue;
            }
        };
        let (key, draft) = store_retrieved(
            retrieved,
            fragmentless,
            default_draft,
            documents,
            known_resources,
            &mut state.index,
            &mut state.custom_metaschemas,
        );
        for (_, uri, _) in &refs {
            enqueue_fragment_entry(uri, &key, default_draft, documents, &mut state.queue);
        }
        state.queue.push_back(CrawlTask {
            base_uri: Arc::clone(&key),
            document_root_uri: key,
            pointer_path: String::new(),
            draft,
        });
    }
    Ok(())
}

// Recursion depth at which the crawl stops descending and defers the remaining subtree to a heap
// work-list. Real schemas nest far shallower, so the list stays empty and the crawl runs as plain
// recursion; only pathologically deep documents pay for the iterative fallback that avoids a
// stack overflow.
// Recursion depth at which the crawl stops descending and defers the remaining subtree to a heap
// work-list. Real schemas nest far shallower, so the list stays empty and the crawl runs as plain
// recursion; only pathologically deep documents pay for the iterative fallback that avoids a
// stack overflow.
const RECURSION_DEPTH_CAP: usize = 128;

struct DeferredBorrowed<'v> {
    base: Arc<Uri<String>>,
    subschema: &'v Value,
    draft: Draft,
}

// Traversal state shared across the borrowed crawl, bundled so the hot recursive call stays small.
struct BorrowedCrawl<'v, 'a> {
    document_root: &'v Value,
    document_root_uri: &'a Arc<Uri<String>>,
    state: &'a mut BuildState<'v>,
    known_resources: &'a mut KnownResources,
    resolution_cache: &'a mut UriCache,
    visited_local_refs: &'a mut VisitedLocalRefs<'v>,
    deferred: Vec<DeferredBorrowed<'v>>,
}

fn crawl_borrowed_at<'r>(
    current_base_uri: Arc<Uri<String>>,
    document_root_uri: &Arc<Uri<String>>,
    document_root: &'r Value,
    pointer_path: &str,
    draft: Draft,
    state: &mut BuildState<'r>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    visited: &mut VisitedLocalRefs<'r>,
) -> Result<(), Error> {
    let Some(subschema) = (if pointer_path.is_empty() {
        Some(document_root)
    } else {
        pointer(document_root, pointer_path)
    }) else {
        return Ok(());
    };

    let mut crawl = BorrowedCrawl {
        document_root,
        document_root_uri,
        state,
        known_resources,
        resolution_cache,
        visited_local_refs: visited,
        deferred: Vec::new(),
    };
    crawl_schema(
        &mut crawl,
        current_base_uri,
        subschema,
        draft,
        pointer_path.is_empty(),
        0,
    )?;
    while let Some(DeferredBorrowed {
        base,
        subschema,
        draft,
    }) = crawl.deferred.pop()
    {
        crawl_schema(&mut crawl, base, subschema, draft, false, 0)?;
    }
    Ok(())
}

fn crawl_schema<'v>(
    crawl: &mut BorrowedCrawl<'v, '_>,
    mut base: Arc<Uri<String>>,
    subschema: &'v Value,
    draft: Draft,
    is_document_root: bool,
    depth: usize,
) -> Result<(), Error> {
    let Some(object) = subschema.as_object() else {
        return Ok(());
    };
    let analysis = draft.analyze_object(object);

    if let Some(id) = analysis.id {
        let ResolvedId {
            base: new_base,
            uri_changed,
        } = resolve_subresource_id(&base, id, crawl.known_resources, crawl.resolution_cache)?;
        base = new_base;
        if !(is_document_root && base.as_ref() == crawl.document_root_uri.as_ref()) {
            crawl
                .state
                .index
                .register_borrowed_subresource(&base, draft, uri_changed, subschema);
        }
    } else if analysis.has_anchor && !is_document_root {
        crawl
            .state
            .index
            .register_borrowed_subresource(&base, draft, false, subschema);
    }

    if crawl
        .state
        .crawl
        .visited_schemas
        .insert(visited_schema_context(&base, draft, subschema))
        && (analysis.dollar_ref.is_some() || analysis.meta_schema.is_some())
    {
        record_refs(
            &base,
            crawl.document_root,
            analysis.dollar_ref,
            analysis.meta_schema,
            &mut crawl.state.crawl,
            crawl.resolution_cache,
            draft,
            crawl.document_root_uri,
            crawl.visited_local_refs,
        )?;
    }

    draft.walk_children(object, &mut |_key, _sub, child, child_draft| {
        if depth + 1 >= RECURSION_DEPTH_CAP {
            crawl.deferred.push(DeferredBorrowed {
                base: Arc::clone(&base),
                subschema: child,
                draft: child_draft,
            });
            Ok(())
        } else {
            crawl_schema(
                crawl,
                Arc::clone(&base),
                child,
                child_draft,
                false,
                depth + 1,
            )
        }
    })
}

struct DeferredOwned<'doc> {
    base: Arc<Uri<String>>,
    subschema: &'doc Value,
    draft: Draft,
    pointer: ParsedPointer,
}

fn crawl_owned_at<'r>(
    current_base_uri: Arc<Uri<String>>,
    document_root_uri: &Arc<Uri<String>>,
    document: &Arc<StoredResource<'r>>,
    pointer_path: &str,
    draft: Draft,
    state: &mut BuildState<'r>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
) -> Result<(), Error> {
    let document_root = document.contents();
    let subschema = if pointer_path.is_empty() {
        Some(document_root)
    } else {
        pointer(document_root, pointer_path)
    };
    let Some(subschema) = subschema else {
        return Ok(());
    };
    let root_pointer = ParsedPointer::from_json_pointer(pointer_path).unwrap_or_default();
    let mut visited_local_refs = VisitedLocalRefs::new();
    let mut deferred = Vec::new();

    crawl_owned_schema(
        current_base_uri,
        document_root,
        subschema,
        draft,
        pointer_path.is_empty(),
        &root_pointer.segments,
        &JsonPointerNode::new(),
        0,
        &mut deferred,
        document_root_uri,
        document,
        state,
        known_resources,
        resolution_cache,
        &mut visited_local_refs,
    )?;

    while let Some(DeferredOwned {
        base,
        subschema,
        draft,
        pointer,
    }) = deferred.pop()
    {
        crawl_owned_schema(
            base,
            document_root,
            subschema,
            draft,
            false,
            &pointer.segments,
            &JsonPointerNode::new(),
            0,
            &mut deferred,
            document_root_uri,
            document,
            state,
            known_resources,
            resolution_cache,
            &mut visited_local_refs,
        )?;
    }
    Ok(())
}

// Absolute pointer of a node = the owned `prefix` (empty at a document root, the deferred
// subtree's root pointer otherwise) followed by `relative`, the bounded pointer walked since.
fn absolute_pointer(
    prefix: &[ParsedPointerSegment],
    relative: &JsonPointerNode<'_, '_>,
) -> ParsedPointer {
    let mut pointer = ParsedPointer::from_pointer_node(relative);
    if prefix.is_empty() {
        return pointer;
    }
    let mut segments = Vec::with_capacity(prefix.len() + pointer.segments.len());
    segments.extend(prefix.iter().cloned());
    segments.append(&mut pointer.segments);
    ParsedPointer { segments }
}

#[allow(clippy::too_many_arguments)]
fn crawl_owned_schema<'r, 'doc>(
    mut base: Arc<Uri<String>>,
    document_root: &'doc Value,
    subschema: &'doc Value,
    draft: Draft,
    is_document_root: bool,
    base_pointer: &[ParsedPointerSegment],
    path: &JsonPointerNode<'_, '_>,
    depth: usize,
    deferred: &mut Vec<DeferredOwned<'doc>>,
    document_root_uri: &Arc<Uri<String>>,
    document: &Arc<StoredResource<'r>>,
    state: &mut BuildState<'r>,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
    visited_local_refs: &mut VisitedLocalRefs<'doc>,
) -> Result<(), Error> {
    let Some(object) = subschema.as_object() else {
        return Ok(());
    };
    let analysis = draft.analyze_object(object);

    if let Some(id) = analysis.id {
        let ResolvedId {
            base: new_base,
            uri_changed,
        } = resolve_subresource_id(&base, id, known_resources, resolution_cache)?;
        base = new_base;
        if !(is_document_root && base.as_ref() == document_root_uri.as_ref())
            && (uri_changed || analysis.has_anchor)
        {
            let pointer = absolute_pointer(base_pointer, path);
            state.index.register_owned_subresource(
                &base,
                document,
                &pointer,
                draft,
                uri_changed,
                subschema,
            );
        }
    } else if analysis.has_anchor && !is_document_root {
        let pointer = absolute_pointer(base_pointer, path);
        state
            .index
            .register_owned_subresource(&base, document, &pointer, draft, false, subschema);
    }

    if state
        .crawl
        .visited_schemas
        .insert(visited_schema_context(&base, draft, subschema))
        && (analysis.dollar_ref.is_some() || analysis.meta_schema.is_some())
    {
        record_refs(
            &base,
            document_root,
            analysis.dollar_ref,
            analysis.meta_schema,
            &mut state.crawl,
            resolution_cache,
            draft,
            document_root_uri,
            visited_local_refs,
        )?;
    }

    draft.walk_children(object, &mut |key, sub, child_value, child_draft| {
        with_owned_child_path(path, key, sub.as_ref(), |child_path| {
            if depth + 1 >= RECURSION_DEPTH_CAP {
                deferred.push(DeferredOwned {
                    base: Arc::clone(&base),
                    subschema: child_value,
                    draft: child_draft,
                    pointer: absolute_pointer(base_pointer, child_path),
                });
                Ok(())
            } else {
                crawl_owned_schema(
                    Arc::clone(&base),
                    document_root,
                    child_value,
                    child_draft,
                    false,
                    base_pointer,
                    child_path,
                    depth + 1,
                    deferred,
                    document_root_uri,
                    document,
                    state,
                    known_resources,
                    resolution_cache,
                    visited_local_refs,
                )
            }
        })
    })
}

fn with_owned_child_path<R>(
    path: &JsonPointerNode<'_, '_>,
    key: &str,
    sub: Option<&JsonPointerSegment<'_>>,
    f: impl FnOnce(&JsonPointerNode<'_, '_>) -> R,
) -> R {
    let first = path.push(key);
    match sub {
        Some(JsonPointerSegment::Key(k)) => {
            let second = first.push(k.as_ref());
            f(&second)
        }
        Some(JsonPointerSegment::Index(i)) => {
            let second = first.push(*i);
            f(&second)
        }
        None => f(&first),
    }
}

fn record_refs<'doc>(
    base: &Arc<Uri<String>>,
    root: &'doc Value,
    dollar_ref: Option<&'doc str>,
    meta_schema: Option<&'doc str>,
    crawl: &mut CrawlState,
    resolution_cache: &mut UriCache,
    _draft: Draft,
    doc_key: &Arc<Uri<String>>,
    visited: &mut VisitedLocalRefs<'doc>,
) -> Result<(), Error> {
    for (reference, key) in [(dollar_ref, "$ref"), (meta_schema, "$schema")] {
        let Some(reference) = reference else {
            continue;
        };
        if reference.starts_with("https://json-schema.org/draft/")
            || reference.starts_with("http://json-schema.org/draft-")
            || base.as_str().starts_with("https://json-schema.org/draft/")
        {
            if key == "$ref" {
                crawl.found_metaschema_ref = true;
            }
            continue;
        }
        if reference == "#" {
            continue;
        }
        if reference.starts_with('#') {
            if record_local_ref_visit(visited, base, reference) {
                let pointer_path = reference.trim_start_matches('#');
                if let Some(referenced) = pointer(root, pointer_path) {
                    let schema_ptr = std::ptr::from_ref::<Value>(referenced) as usize;
                    crawl.deferred_refs.push(PendingLocalRef {
                        document_root_uri: Arc::clone(doc_key),
                        pointer: pointer_path.to_string(),
                        schema_ptr,
                    });
                }
            }
            continue;
        }
        let resolved = (*resolution_cache.resolve_against(&base.borrow(), reference)?).clone();

        let kind = if key == "$schema" {
            ReferenceKind::DollarSchema
        } else {
            ReferenceKind::DollarRef
        };
        crawl
            .external
            .insert((reference.to_string(), resolved, kind));
    }
    Ok(())
}

fn record_refs_from_object<'doc>(
    base: &Arc<Uri<String>>,
    root: &'doc Value,
    contents: &'doc Value,
    crawl: &mut CrawlState,
    resolution_cache: &mut UriCache,
    draft: Draft,
    doc_key: &Arc<Uri<String>>,
    visited: &mut VisitedLocalRefs<'doc>,
) -> Result<(), Error> {
    if base.scheme().as_str() == "urn" {
        return Ok(());
    }
    if let Some(object) = contents.as_object() {
        let dollar_ref = object.get("$ref").and_then(Value::as_str);
        let meta_schema = object.get("$schema").and_then(Value::as_str);
        if dollar_ref.is_some() || meta_schema.is_some() {
            record_refs(
                base,
                root,
                dollar_ref,
                meta_schema,
                crawl,
                resolution_cache,
                draft,
                doc_key,
                visited,
            )?;
        }
    }
    Ok(())
}

fn record_refs_recursive<'doc>(
    base: &Arc<Uri<String>>,
    root: &'doc Value,
    contents: &'doc Value,
    crawl: &mut CrawlState,
    resolution_cache: &mut UriCache,
    draft: Draft,
    doc_key: &Arc<Uri<String>>,
    visited_refs: &mut VisitedLocalRefs<'doc>,
) -> Result<(), Error> {
    let current_base = match draft.id_of(contents) {
        Some(id) => resolve_id(base, id, resolution_cache)?,
        None => Arc::clone(base),
    };

    if !crawl
        .visited_schemas
        .insert(visited_schema_context(&current_base, draft, contents))
    {
        return Ok(());
    }

    record_refs_from_object(
        &current_base,
        root,
        contents,
        crawl,
        resolution_cache,
        draft,
        doc_key,
        visited_refs,
    )?;

    for subresource in draft.subresources_of(contents) {
        let subresource_draft = draft.detect(subresource);
        record_refs_recursive(
            &current_base,
            root,
            subresource,
            crawl,
            resolution_cache,
            subresource_draft,
            doc_key,
            visited_refs,
        )?;
    }
    Ok(())
}

fn enqueue_fragment_entry(
    uri: &Uri<String>,
    key: &Arc<Uri<String>>,
    default_draft: Draft,
    documents: &ResourceStore<'_>,
    queue: &mut VecDeque<CrawlTask>,
) {
    if let Some(fragment) = uri.fragment() {
        let Some(document) = documents.get(key) else {
            return;
        };
        if let Some(resolved) = pointer(document.contents(), fragment.as_str()) {
            let fragment_draft = default_draft.detect(resolved);
            queue.push_back(CrawlTask {
                base_uri: Arc::clone(key),
                document_root_uri: Arc::clone(key),
                pointer_path: fragment.as_str().to_string(),
                draft: fragment_draft,
            });
        }
    }
}

fn inject_metaschemas<'a>(
    found_metaschema_ref: bool,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    draft_version: Draft,
    state: &mut BuildState<'a>,
) -> Result<(), Error> {
    // A `$schema` naming one of the bundled vocabulary meta-schemas is never retrieved, so the
    // bundle has to supply it, whatever draft it belongs to.
    let names_bundled_meta_schema = state
        .custom_metaschemas
        .iter()
        .any(|uri| uri.starts_with("https://json-schema.org/draft/"));
    let schemas: &[(&str, &Value)] = if names_bundled_meta_schema {
        META_SCHEMAS_ALL.as_slice()
    } else if found_metaschema_ref {
        metas_for_draft(draft_version)
    } else {
        return Ok(());
    };
    for (uri, schema) in schemas {
        let key = Arc::new(uri::from_str(uri.trim_end_matches('#'))?);
        if documents.contains_key(&key) {
            continue;
        }
        let draft = Draft::default().detect(schema);
        documents.insert(
            Arc::clone(&key),
            Arc::new(StoredResource::borrowed(schema, draft)),
        );
        known_resources.insert((*key).clone());
        state.index.register_document(
            &key,
            documents
                .get(&key)
                .expect("meta-schema document was just inserted into the store"),
        );
        state.queue.push_back(CrawlTask {
            base_uri: Arc::clone(&key),
            document_root_uri: Arc::clone(&key),
            pointer_path: String::new(),
            draft,
        });
    }
    Ok(())
}

fn store_retrieved<'a>(
    retrieved: Value,
    fragmentless: Uri<String>,
    default_draft: Draft,
    documents: &mut ResourceStore<'a>,
    known_resources: &mut KnownResources,
    index: &mut Index<'a>,
    custom_metaschemas: &mut Vec<String>,
) -> (Arc<Uri<String>>, Draft) {
    let draft = default_draft.detect(&retrieved);
    let key = Arc::new(fragmentless);
    documents.insert(
        Arc::clone(&key),
        Arc::new(StoredResource::owned(retrieved, draft)),
    );

    let contents = documents
        .get(&key)
        .expect("document was just inserted")
        .contents();
    known_resources.insert((*key).clone());
    index.register_document(
        &key,
        documents
            .get(&key)
            .expect("retrieved document was just inserted into the store"),
    );

    if draft == Draft::Unknown {
        if let Some(meta_schema) = contents
            .as_object()
            .and_then(|obj| obj.get("$schema"))
            .and_then(|schema| schema.as_str())
        {
            custom_metaschemas.push(meta_schema.to_string());
        }
    }

    (key, draft)
}

fn record_local_ref_visit<'a>(
    visited_local_refs: &mut VisitedLocalRefs<'a>,
    base: &Arc<Uri<String>>,
    reference: &'a str,
) -> bool {
    visited_local_refs.insert((base_uri_ptr(base), reference))
}

fn resolve_deferred_target<'a>(
    document_root_uri: &Arc<Uri<String>>,
    document_root: &'a Value,
    pointer_path: &str,
    document_draft: Draft,
    resolution_cache: &mut UriCache,
) -> Result<Option<DeferredTarget<'a>>, Error> {
    let mut current = document_root;
    let mut current_draft = document_draft;
    let mut current_base = Arc::clone(document_root_uri);

    if let Some(id) = current_draft.id_of(current) {
        current_base = resolve_id(&current_base, id, resolution_cache)?;
    }

    let pointer = ParsedPointer::from_json_pointer(pointer_path);
    let Some(pointer) = pointer.as_ref() else {
        return Ok(Some((current_base, current_draft, current)));
    };

    for segment in &pointer.segments {
        let Some(next) = (match segment {
            ParsedPointerSegment::Key(key) => current
                .as_object()
                .and_then(|object| object.get(key.as_ref())),
            ParsedPointerSegment::Index(index) => {
                current.as_array().and_then(|array| array.get(*index))
            }
        }) else {
            return Ok(None);
        };
        current = next;
        current_draft = current_draft.detect(current);
        if let Some(id) = current_draft.id_of(current) {
            current_base = resolve_id(&current_base, id, resolution_cache)?;
        }
    }

    Ok(Some((current_base, current_draft, current)))
}

fn visited_schema_context(
    base: &Arc<Uri<String>>,
    draft: Draft,
    schema: &Value,
) -> VisitedSchemaContext {
    VisitedSchemaContext {
        schema_ptr: schema_ptr(schema),
        base_ptr: base_uri_ptr(base),
        draft,
    }
}

fn visited_schema_context_from_ptr(
    base: &Arc<Uri<String>>,
    draft: Draft,
    schema_ptr: usize,
) -> VisitedSchemaContext {
    VisitedSchemaContext {
        schema_ptr: NonZeroUsize::new(schema_ptr).expect("Value pointer should never be null"),
        base_ptr: base_uri_ptr(base),
        draft,
    }
}

fn base_uri_ptr(base: &Arc<Uri<String>>) -> NonZeroUsize {
    NonZeroUsize::new(Arc::as_ptr(base) as usize).expect("Arc pointer should never be null")
}

fn schema_ptr(schema: &Value) -> NonZeroUsize {
    NonZeroUsize::new(std::ptr::from_ref::<Value>(schema) as usize)
        .expect("Value pointer should never be null")
}

fn handle_retrieve_error(
    uri: &Uri<String>,
    // The original reference string is used in error messages for `json-schema://` URIs
    // where the resolved URI is not user-friendly (e.g. "./foo.json" vs "json-schema:///foo.json").
    original: &str,
    fragmentless: &Uri<String>,
    error: Box<dyn std::error::Error + Send + Sync>,
    kind: ReferenceKind,
) -> Result<(), Error> {
    match kind {
        ReferenceKind::DollarSchema => Ok(()),
        ReferenceKind::DollarRef => {
            if uri.scheme().as_str() == "json-schema" {
                Err(Error::unretrievable(
                    original,
                    "No base URI is available".into(),
                ))
            } else {
                Err(Error::unretrievable(fragmentless.as_str(), error))
            }
        }
    }
}

fn resolve_id(
    base: &Arc<Uri<String>>,
    id: &str,
    resolution_cache: &mut UriCache,
) -> Result<Arc<Uri<String>>, Error> {
    if id.starts_with('#') {
        return Ok(Arc::clone(base));
    }
    let normalized = id.strip_suffix('#').unwrap_or(id);
    resolution_cache.resolve_against(&base.borrow(), normalized)
}

fn resolve_subresource_id(
    current_base_uri: &Arc<Uri<String>>,
    id: &str,
    known_resources: &mut KnownResources,
    resolution_cache: &mut UriCache,
) -> Result<ResolvedId, Error> {
    let base = resolve_id(current_base_uri, id, resolution_cache)?;
    let uri_changed = base != *current_base_uri;
    known_resources.insert((*base).clone());
    Ok(ResolvedId { base, uri_changed })
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/referencing/unit/src/registry/build/tests_1393.rs"
));

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/referencing/unit/src/registry/build/all_async_tests_1396.rs"
));
