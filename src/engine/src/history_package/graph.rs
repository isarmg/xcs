//! Candidate-bound paged history roots. The partitioning constants belong
//! to this disk format, not to a caller's metadata admission budget.
use super::HistoryManifest;
use crate::{
    Error, HistoryTree, Repository, Result, canonical_json, disk::MAX_METADATA,
    encode_hashed_object, journal::PreparedMetadataObject, object_hash,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const PAGE_ENTRIES: usize = 128;
const PAGE_TARGET_BYTES: usize = 64 * 1024;
const RADIX_DEPTH: usize = 64;

// This is a schema, not a list inferred from fields present in untrusted JSON.
// Even optional, omitted logical maps have explicit roots in the paged graph.
const MAP_FIELDS: &[&str] = &[
    "branches",
    "file_selections",
    "directory_branches",
    "directory_trees",
    "file_write_origins",
    "file_contexts",
    "file_path_nodes",
    "file_metadata_nodes",
    "file_branch_merges",
    "file_branch_aliases",
    "file_node_parents",
    "file_path_notes",
    "directory_tree_selections",
    "file_locations",
    "file_backup_paths",
    "file_backup_policies",
    "directory_selections",
    "directory_selection_generations",
    "file_selection_generations",
    "file_notes",
    "version_notes",
    "file_names",
    "commits",
    "versions",
    "placements",
    "merge_links",
    "boundary_marks",
    "content_states",
    "placement_rules",
    "protection_policies",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
enum ReferenceKind {
    #[serde(rename = "state-page")]
    Page,
    #[serde(rename = "history-index")]
    Index,
}
impl ReferenceKind {
    fn domain(self) -> &'static str {
        match self {
            Self::Page => "state-page",
            Self::Index => "history-index",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    kind: ReferenceKind,
    hash: String,
}
impl Reference {
    fn validate(&self) -> Result<()> {
        if self.hash.len() != RADIX_DEPTH || !hexadecimal(&self.hash) {
            return Err(invalid("history graph reference has an invalid digest"));
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PagedHistoryRoot {
    repository: Repository,
    history: HistoryTree,
    maps: BTreeMap<String, Reference>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HistoryIndex {
    prefix: String,
    children: BTreeMap<String, Reference>,
}

pub(super) type GraphObject = (String, String, Value);
pub(super) type ResolvedGraph = (HistoryManifest, Vec<GraphObject>);

fn invalid(message: &str) -> Error {
    Error::new("HISTORY_PACKAGE_INVALID", message)
}

fn hexadecimal(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn key_route(key: &str) -> String {
    blake3::hash(key.as_bytes()).to_hex().to_string()
}

struct Entry {
    key: String,
    value: Value,
    route: String,
    // Canonical key, colon, and value, without surrounding braces or commas.
    encoded_bytes: usize,
}

struct Encoder {
    maximum: usize,
    objects: Vec<PreparedMetadataObject>,
    seen: BTreeSet<(&'static str, String)>,
}
impl Encoder {
    fn object<T: Serialize>(&mut self, kind: &'static str, value: &T) -> Result<String> {
        let object = PreparedMetadataObject::new(kind, value, self.maximum)?;
        let hash = object.hash.clone();
        if self.seen.insert((kind, hash.clone())) {
            self.objects.push(object);
        }
        Ok(hash)
    }

    fn map(&mut self, map: Map<String, Value>) -> Result<Reference> {
        let entries = map
            .into_iter()
            .map(|(key, value)| {
                let encoded_bytes = canonical_json(&key)?.len() + 1 + canonical_json(&value)?.len();
                // A single entity cannot be divided. Keep the existing reader
                // ceiling rather than turning the page target into a new
                // limit on Commit.entries and other existing entity payloads.
                if encoded_bytes > MAX_METADATA - 2 {
                    return Err(Error::new(
                        "RESOURCE_LIMIT",
                        "one history entity exceeds the metadata reader limit",
                    ));
                }
                Ok(Entry {
                    route: key_route(&key),
                    key,
                    value,
                    encoded_bytes,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        self.node(entries, String::new())
    }

    fn node(&mut self, entries: Vec<Entry>, prefix: String) -> Result<Reference> {
        let encoded_bytes = entries.iter().fold(
            2usize.saturating_add(entries.len().saturating_sub(1)),
            |size, entry| size.saturating_add(entry.encoded_bytes),
        );
        if entries.len() <= PAGE_ENTRIES
            && (encoded_bytes <= PAGE_TARGET_BYTES || entries.len() == 1)
        {
            let page = entries
                .into_iter()
                .map(|entry| (entry.key, entry.value))
                .collect::<Map<_, _>>();
            return Ok(Reference {
                kind: ReferenceKind::Page,
                hash: self.object("state-page", &page)?,
            });
        }
        let depth = prefix.len();
        if depth >= RADIX_DEPTH {
            return Err(Error::new(
                "RESOURCE_LIMIT",
                "history entity keys cannot be divided within the radix depth limit",
            ));
        }
        let mut buckets: [Vec<Entry>; 16] = std::array::from_fn(|_| Vec::new());
        for entry in entries {
            let digit = entry.route.as_bytes()[depth];
            let bucket = match digit {
                b'0'..=b'9' => digit - b'0',
                _ => digit - b'a' + 10,
            };
            buckets[usize::from(bucket)].push(entry);
        }
        let mut children = BTreeMap::new();
        for (bucket, entries) in buckets.into_iter().enumerate() {
            if !entries.is_empty() {
                let digit = format!("{bucket:x}");
                let child = self.node(entries, format!("{prefix}{digit}"))?;
                children.insert(digit, child);
            }
        }
        Ok(Reference {
            kind: ReferenceKind::Index,
            hash: self.object("history-index", &HistoryIndex { prefix, children })?,
        })
    }
}

/// Freeze the complete graph in dependency order. Admission limits may reject
/// a graph but never change its partitioning or its candidate-bound root hash.
pub(super) fn prepare(
    manifest: &HistoryManifest,
    maximum: usize,
) -> Result<(String, Vec<PreparedMetadataObject>)> {
    // Going through the logical serializer preserves all field-specific wire
    // rules, especially omitted optional maps and decimal-string u64 values.
    let Value::Object(mut logical) = serde_json::to_value(manifest)? else {
        return Err(invalid("history manifest is not an object"));
    };
    for field in ["repository", "history"] {
        logical.remove(field);
    }
    let mut encoder = Encoder {
        maximum: maximum.min(MAX_METADATA),
        objects: Vec::new(),
        seen: BTreeSet::new(),
    };
    let mut maps = BTreeMap::new();
    for &field in MAP_FIELDS {
        let value = logical
            .remove(field)
            .unwrap_or_else(|| Value::Object(Map::new()));
        let Value::Object(map) = value else {
            return Err(invalid("history manifest map is not an object"));
        };
        maps.insert(field.to_owned(), encoder.map(map)?);
    }
    if !logical.is_empty() {
        return Err(invalid(
            "history graph schema omits a logical manifest field",
        ));
    }
    let root = PagedHistoryRoot {
        repository: manifest.repository.clone(),
        history: manifest.history.clone(),
        maps,
    };
    let hash = encoder.object("history-manifest", &root)?;
    Ok((hash, encoder.objects))
}

struct Resolver<F> {
    load: F,
    objects: Vec<GraphObject>,
    seen: BTreeMap<(String, String), usize>,
}
impl<F: FnMut(&str, &str) -> Result<Value>> Resolver<F> {
    fn map(
        &mut self,
        reference: &Reference,
        prefix: &str,
        entries: &mut Map<String, Value>,
    ) -> Result<()> {
        // Validate before invoking a loader which may turn this address into a
        // filesystem path. The only possible domains come from ReferenceKind.
        reference.validate()?;
        if prefix.len() > RADIX_DEPTH || !hexadecimal(prefix) {
            return Err(invalid("invalid history graph routing prefix"));
        }
        let domain = reference.kind.domain();
        let address = (domain.to_owned(), reference.hash.clone());
        let value = match self.seen.get(&address) {
            Some(index) => self.objects[*index].2.clone(),
            None => {
                let value = (self.load)(domain, &reference.hash)?;
                if object_hash(domain, &value)? != reference.hash {
                    return Err(Error::new(
                        "OBJECT_CORRUPT",
                        "history graph object digest differs",
                    ));
                }
                value
            }
        };
        match reference.kind {
            ReferenceKind::Page => {
                let page = value
                    .as_object()
                    .ok_or_else(|| invalid("history graph leaf is not a map"))?;
                let bytes = canonical_json(&value)?.len();
                if page.len() > PAGE_ENTRIES
                    || bytes > MAX_METADATA
                    || (bytes > PAGE_TARGET_BYTES && page.len() != 1)
                    || (page.is_empty() && !prefix.is_empty())
                {
                    return Err(invalid("history graph leaf exceeds its format bounds"));
                }
                for (key, value) in page {
                    if !key_route(key).starts_with(prefix) {
                        return Err(invalid("history entity is in the wrong radix branch"));
                    }
                    if entries.insert(key.clone(), value.clone()).is_some() {
                        return Err(invalid("duplicate history entity in graph"));
                    }
                }
            }
            ReferenceKind::Index => {
                let index: HistoryIndex = serde_json::from_value(value.clone())?;
                if index.prefix != prefix
                    || prefix.len() >= RADIX_DEPTH
                    || index.children.is_empty()
                    || index.children.len() > 16
                {
                    return Err(invalid("invalid history graph index"));
                }
                for (digit, child) in &index.children {
                    if digit.len() != 1 || !hexadecimal(digit) {
                        return Err(invalid("invalid history graph radix digit"));
                    }
                    child.validate()?;
                }
                for (digit, child) in index.children {
                    self.map(&child, &format!("{prefix}{digit}"), entries)?;
                }
            }
        }
        if !self.seen.contains_key(&address) {
            self.seen.insert(address, self.objects.len());
            self.objects
                .push((domain.to_owned(), reference.hash.clone(), value));
        }
        Ok(())
    }
}

/// The caller must verify the root against the candidate's expected hash, and
/// strictly decode JSON before passing Values (duplicate JSON fields cannot be
/// recovered after parsing). Every referenced object's domain hash is checked
/// here independently of any verification performed by the loader.
pub(super) fn resolve(
    root: Value,
    load: impl FnMut(&str, &str) -> Result<Value>,
) -> Result<HistoryManifest> {
    resolve_with_objects(root, load).map(|(manifest, _)| manifest)
}

/// Return the exact reachable immutable values, deduplicated by domain and
/// hash, with dependencies first and the exact paged root last. The expanded
/// logical model is not an alternate root representation.
pub(super) fn resolve_with_objects(
    root: Value,
    load: impl FnMut(&str, &str) -> Result<Value>,
) -> Result<ResolvedGraph> {
    let hash = {
        let (bytes, hash) = encode_hashed_object("history-manifest", &root)?;
        if bytes.len() > MAX_METADATA {
            return Err(invalid("history root exceeds the metadata reader limit"));
        }
        hash
    };
    let decoded: PagedHistoryRoot = serde_json::from_value(root.clone())?;
    if decoded.maps.len() != MAP_FIELDS.len()
        || MAP_FIELDS
            .iter()
            .any(|field| !decoded.maps.contains_key(*field))
    {
        return Err(invalid("history root has an invalid logical schema"));
    }
    for reference in decoded.maps.values() {
        reference.validate()?;
    }
    let mut logical = Map::new();
    logical.insert(
        "repository".to_owned(),
        serde_json::to_value(decoded.repository)?,
    );
    logical.insert("history".to_owned(), serde_json::to_value(decoded.history)?);
    let mut resolver = Resolver {
        load,
        objects: Vec::new(),
        seen: BTreeMap::new(),
    };
    for (field, reference) in decoded.maps {
        let mut entries = Map::new();
        resolver.map(&reference, "", &mut entries)?;
        logical.insert(field, Value::Object(entries));
    }
    let manifest = serde_json::from_value(Value::Object(logical))?;
    resolver
        .objects
        .push(("history-manifest".to_owned(), hash, root));
    Ok((manifest, resolver.objects))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Config,
        journal::{load_object, object_path},
    };
    use serde_json::json;

    fn manifest() -> HistoryManifest {
        let mut value = json!({
            "repository": {
                "repository_id": "repository",
                "name": "history graph",
                "history_tree_id": "history",
                "main_branch_id": "branch",
                "primary_slot_id": "P01",
                "target_copies": 3,
                "name_policy": "portable",
                "unicode_version": "15.1",
                "grants": {"read": [], "write": [], "manage": []}
            },
            "history": {
                "history_tree_id": "history",
                "repository_id": "repository",
                "root_commit_id": "commit",
                "main_branch_id": "branch",
                "generation": "18446744073709551615",
                "state": "active",
                "provenance": null
            }
        });
        for &field in MAP_FIELDS {
            value[field] = json!({});
        }
        serde_json::from_value(value).unwrap()
    }

    fn encoded(
        manifest: &HistoryManifest,
        maximum: usize,
    ) -> (String, Value, BTreeMap<(String, String), Value>) {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("a"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let disk = &config.disks[0];
        let (hash, prepared) = prepare(manifest, maximum).unwrap();
        for object in prepared {
            object.store(std::slice::from_ref(disk)).unwrap();
        }
        let root: Value = load_object(disk, "history-manifest", &hash).unwrap();
        let (_, objects) =
            resolve_with_objects(root.clone(), |kind, digest| load_object(disk, kind, digest))
                .unwrap();
        for (kind, digest, value) in &objects {
            assert_eq!(
                std::fs::read(object_path(disk, kind, digest)).unwrap(),
                canonical_json(value).unwrap()
            );
        }
        (
            hash,
            root,
            objects
                .into_iter()
                .map(|(kind, hash, value)| ((kind, hash), value))
                .collect(),
        )
    }

    fn object(values: &mut BTreeMap<(String, String), Value>, kind: &str, value: Value) -> Value {
        let hash = object_hash(kind, &value).unwrap();
        values.insert((kind.to_owned(), hash.clone()), value);
        json!({"kind": kind, "hash": hash})
    }

    fn read(root: Value, values: &BTreeMap<(String, String), Value>) -> Result<ResolvedGraph> {
        resolve_with_objects(root, |kind, hash| {
            values
                .get(&(kind.to_owned(), hash.to_owned()))
                .cloned()
                .ok_or_else(|| Error::new("OBJECT_CORRUPT", "missing test graph object"))
        })
    }

    #[test]
    fn deterministic_bounded_graph_preserves_logical_wire_rules() {
        let mut manifest = manifest();
        for n in (0..1800).rev() {
            manifest
                .file_notes
                .insert(format!("file-{n:05}"), "文".repeat(90));
        }
        manifest
            .file_selection_generations
            .insert("large".to_owned(), u64::MAX);
        manifest
            .directory_selection_generations
            .insert("unsafe-number".to_owned(), (1 << 53) + 1);
        assert!(canonical_json(&manifest).unwrap().len() > PAGE_TARGET_BYTES);
        let (hash, root, values) = encoded(&manifest, PAGE_TARGET_BYTES);
        let (large_limit_hash, prepared) = prepare(&manifest, MAX_METADATA).unwrap();
        assert_eq!(hash, large_limit_hash);
        assert_eq!(values.len(), prepared.len());
        let (resolved, reachable) = read(root.clone(), &values).unwrap();
        assert_eq!(
            canonical_json(&manifest).unwrap(),
            canonical_json(&resolved).unwrap()
        );
        assert_eq!(
            reachable.last().unwrap(),
            &("history-manifest".to_owned(), hash, root.clone())
        );
        assert_eq!(
            root.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["history", "maps", "repository"])
        );
        assert_eq!(root["maps"].as_object().unwrap().len(), MAP_FIELDS.len());
        assert_eq!(root["history"]["generation"], u64::MAX.to_string());
        let mut seen = BTreeSet::new();
        for (kind, hash, value) in reachable {
            assert!(seen.insert((kind.clone(), hash)));
            assert!(canonical_json(&value).unwrap().len() <= PAGE_TARGET_BYTES);
            if kind == "state-page" {
                assert!(value.as_object().unwrap().len() <= PAGE_ENTRIES);
            } else if kind == "history-index" {
                assert_eq!(
                    value
                        .as_object()
                        .unwrap()
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>(),
                    BTreeSet::from(["children", "prefix"])
                );
                let index: HistoryIndex = serde_json::from_value(value).unwrap();
                assert!(index.children.len() <= 16);
                for reference in index.children.values() {
                    assert!(
                        seen.contains(&(
                            reference.kind.domain().to_owned(),
                            reference.hash.clone()
                        ))
                    );
                }
            }
        }
    }

    #[test]
    fn singleton_over_page_target_retains_existing_entity_ceiling() {
        let mut manifest = manifest();
        manifest.file_notes.insert(
            "large-single-entity".to_owned(),
            "x".repeat(PAGE_TARGET_BYTES),
        );
        let (hash, root, values) = encoded(&manifest, MAX_METADATA);
        assert_eq!(root["maps"]["file_notes"]["kind"], "state-page");
        assert!(read(root, &values).is_ok());
        let error = match prepare(&manifest, PAGE_TARGET_BYTES) {
            Err(error) => error,
            Ok(_) => panic!("a caller's smaller reader limit must still apply"),
        };
        assert_eq!(error.code, "RESOURCE_LIMIT");
        assert_eq!(hash, prepare(&manifest, MAX_METADATA / 2).unwrap().0);
    }

    #[test]
    fn truly_oversized_singleton_fails_before_any_object_can_be_installed() {
        let mut manifest = manifest();
        manifest
            .file_notes
            .insert("oversized".to_owned(), "x".repeat(MAX_METADATA + 1));
        let error = match prepare(&manifest, usize::MAX) {
            Err(error) => error,
            Ok(_) => panic!("a caller cannot enlarge the immutable object reader limit"),
        };
        assert_eq!(error.code, "RESOURCE_LIMIT");
        assert!(error.message.contains("one history entity"));
    }

    /// This is a codec capacity test, not a semantic notes-policy or an
    /// end-to-end incremental-performance benchmark. Run it alone: the
    /// logical projection and the frozen/resolved graph remain O(total) RAM.
    #[test]
    #[ignore = "real >64 MiB codec scale test; allocates a large logical manifest and graph"]
    fn distributed_history_larger_than_real_reader_limit_roundtrips() {
        let mut manifest = manifest();
        const ENTITIES: usize = 1300;
        const VALUE_BYTES: usize = 53 * 1024;
        for n in 0..ENTITIES {
            manifest
                .file_notes
                .insert(format!("capacity-{n:05}"), "x".repeat(VALUE_BYTES));
        }
        let (flat, logical_hash) = encode_hashed_object("history-manifest", &manifest).unwrap();
        let flat_bytes = flat.len();
        assert!(flat_bytes > MAX_METADATA);
        drop(flat);
        let flat_error =
            match PreparedMetadataObject::new("history-manifest", &manifest, MAX_METADATA) {
                Err(error) => error,
                Ok(_) => panic!("a single object must reject the oversized expanded manifest"),
            };
        assert_eq!(flat_error.code, "RESOURCE_LIMIT");

        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("a"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let disk = &config.disks[0];
        let (root_hash, prepared) = prepare(&manifest, MAX_METADATA).unwrap();
        let prepared_hashes = prepared
            .iter()
            .map(|object| object.hash.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(prepared_hashes.len(), prepared.len());
        // Store one disposable replica, releasing prepared bytes as we go.
        // No database, source tree, real pool, or version bodies are copied.
        for object in prepared {
            object.store(std::slice::from_ref(disk)).unwrap();
        }
        let (tighter_root, tighter_objects) = prepare(&manifest, PAGE_TARGET_BYTES).unwrap();
        assert_eq!(tighter_root, root_hash);
        assert_eq!(
            tighter_objects
                .iter()
                .map(|object| object.hash.clone())
                .collect::<BTreeSet<_>>(),
            prepared_hashes
        );
        drop(tighter_objects);
        let root = load_object(disk, "history-manifest", &root_hash).unwrap();
        let (resolved, objects) =
            resolve_with_objects(root, |kind, hash| load_object(disk, kind, hash)).unwrap();
        assert_eq!(resolved.file_notes, manifest.file_notes);
        assert_eq!(resolved.file_notes.len(), ENTITIES);
        // Compare complete logical canonical identity without retaining two
        // additional >64 MiB byte arrays at once.
        drop(manifest);
        let (resolved_bytes, resolved_hash) =
            encode_hashed_object("history-manifest", &resolved).unwrap();
        assert_eq!(resolved_hash, logical_hash);
        assert_eq!(resolved_bytes.len(), flat_bytes);
        drop(resolved_bytes);
        assert_eq!(objects.len(), prepared_hashes.len());
        let mut total_graph_bytes = 0u64;
        let mut largest_object_bytes = 0u64;
        let mut leaf_pages = 0usize;
        for (kind, hash, _) in &objects {
            assert!(prepared_hashes.contains(hash));
            let bytes = std::fs::metadata(object_path(disk, kind, hash))
                .unwrap()
                .len();
            assert!(bytes <= MAX_METADATA as u64);
            if kind == "state-page" {
                leaf_pages += 1;
                assert!(bytes <= PAGE_TARGET_BYTES as u64);
            }
            total_graph_bytes += bytes;
            largest_object_bytes = largest_object_bytes.max(bytes);
        }
        println!(
            "history codec capacity: flat_bytes={flat_bytes}, reader_limit={MAX_METADATA}, \
             graph_objects={}, leaf_pages={leaf_pages}, total_graph_bytes={total_graph_bytes}, \
             largest_object_bytes={largest_object_bytes}",
            objects.len()
        );
    }

    #[test]
    fn expanded_logical_model_is_not_a_paged_root() {
        let root = serde_json::to_value(manifest()).unwrap();
        let error = resolve_with_objects(root, |_, _| {
            panic!("invalid root shape must not load any dependencies")
        })
        .err()
        .unwrap();
        assert_eq!(error.code(), "INVALID_JSON");
    }

    #[test]
    fn root_requires_the_fixed_complete_schema_and_typed_references() {
        let (_, root, values) = encoded(&manifest(), MAX_METADATA);
        let mut missing = root.clone();
        missing["maps"]
            .as_object_mut()
            .unwrap()
            .remove("file_notes");
        assert!(read(missing, &values).is_err());
        let mut extra = root.clone();
        extra["maps"]["invented"] = root["maps"]["file_notes"].clone();
        assert!(read(extra, &values).is_err());
        let mut extra = root.clone();
        extra["unrecognized"] = json!(true);
        assert!(read(extra, &values).is_err());
        let mut missing_root_field = root.clone();
        missing_root_field
            .as_object_mut()
            .unwrap()
            .remove("history");
        assert!(read(missing_root_field, &values).is_err());
        let mut unknown_kind = root.clone();
        unknown_kind["maps"]["file_notes"]["kind"] = json!("snapshot");
        assert!(read(unknown_kind, &values).is_err());
        let mut extra_reference_field = root.clone();
        extra_reference_field["maps"]["file_notes"]["prefix"] = json!("");
        assert!(read(extra_reference_field, &values).is_err());
        for digest in ["../outside", &"A".repeat(64), &"0".repeat(63)] {
            let mut unsafe_address = root.clone();
            unsafe_address["maps"]["file_notes"]["hash"] = json!(digest);
            assert!(
                resolve(unsafe_address, |_, _| panic!(
                    "unsafe address reached loader"
                ))
                .is_err()
            );
        }
    }

    #[test]
    fn loader_cannot_substitute_another_valid_object() {
        let (_, root, _) = encoded(&manifest(), MAX_METADATA);
        let error = resolve(root, |_, _| Ok(json!({"unexpected": "entity"}))).unwrap_err();
        assert_eq!(error.code, "OBJECT_CORRUPT");
    }

    #[test]
    fn corrupt_index_routes_prefixes_kinds_and_fields_fail_closed() {
        let (_, root, values) = encoded(&manifest(), MAX_METADATA);
        let key = "one-entity";
        let route = key_route(key);
        let digit = &route[..1];
        let wrong_digit = if digit == "0" { "1" } else { "0" };
        for invalid_index in 0..8 {
            let mut values = values.clone();
            let page = object(&mut values, "state-page", json!({key: "note"}));
            let mut children = Map::new();
            children.insert(digit.to_owned(), page.clone());
            let mut index = json!({"prefix": "", "children": children});
            match invalid_index {
                0 => index["prefix"] = json!("0"),
                1 => {
                    index.as_object_mut().unwrap().remove("prefix");
                }
                2 => index["unknown_field"] = json!(true),
                3 => index["children"][wrong_digit] = page,
                4 => index["children"]["invalid-digit"] = page,
                5 => index["children"] = json!({}),
                6 => index["children"][digit]["kind"] = json!("history-manifest"),
                7 => index["children"][digit]["hash"] = json!("../outside"),
                _ => unreachable!(),
            }
            let mut altered_root = root.clone();
            altered_root["maps"]["file_notes"] = object(&mut values, "history-index", index);
            assert!(read(altered_root, &values).is_err(), "case {invalid_index}");
        }
    }

    #[test]
    fn leaf_bounds_and_empty_nonroot_pages_are_validated_on_read() {
        let (_, root, values) = encoded(&manifest(), MAX_METADATA);
        let too_many = (0..=PAGE_ENTRIES)
            .map(|n| (format!("key-{n}"), json!("value")))
            .collect::<Map<_, _>>();
        let too_large = json!({
            "one": "x".repeat(PAGE_TARGET_BYTES / 2),
            "two": "x".repeat(PAGE_TARGET_BYTES / 2)
        });
        for page in [Value::Object(too_many), too_large, json!(["not a map"])] {
            let mut values = values.clone();
            let mut altered_root = root.clone();
            altered_root["maps"]["file_notes"] = object(&mut values, "state-page", page);
            assert!(read(altered_root, &values).is_err());
        }
        let mut values = values;
        let empty = object(&mut values, "state-page", json!({}));
        let mut altered_root = root;
        altered_root["maps"]["file_notes"] = object(
            &mut values,
            "history-index",
            json!({"prefix": "", "children": {"0": empty}}),
        );
        assert!(read(altered_root, &values).is_err());
    }

    #[test]
    fn index_cannot_extend_beyond_the_full_key_digest() {
        let (_, mut root, mut values) = encoded(&manifest(), MAX_METADATA);
        let key = "deep-entity";
        let route = key_route(key);
        let mut reference = object(&mut values, "state-page", json!({key: "note"}));
        for depth in (0..=RADIX_DEPTH).rev() {
            let digit = if depth == RADIX_DEPTH {
                "0"
            } else {
                &route[depth..depth + 1]
            };
            let mut children = Map::new();
            children.insert(digit.to_owned(), reference);
            reference = object(
                &mut values,
                "history-index",
                json!({"prefix": &route[..depth], "children": children}),
            );
        }
        root["maps"]["file_notes"] = reference;
        assert!(read(root, &values).is_err());
    }

    #[test]
    fn cached_objects_are_revalidated_at_each_logical_route() {
        let (_, root, values) = encoded(&manifest(), MAX_METADATA);
        let key = (0..)
            .map(|n| format!("shared-{n}"))
            .find(|key| key_route(key).starts_with("00"))
            .unwrap();
        for reuse_index in [false, true] {
            let mut values = values.clone();
            let page = object(&mut values, "state-page", json!({key.clone(): "note"}));
            let child = if reuse_index {
                object(
                    &mut values,
                    "history-index",
                    json!({"prefix": "0", "children": {"0": page}}),
                )
            } else {
                page
            };
            let index = object(
                &mut values,
                "history-index",
                json!({
                    "prefix": "",
                    "children": {"0": child.clone(), "1": child}
                }),
            );
            let mut altered_root = root.clone();
            altered_root["maps"]["file_notes"] = index;
            let error = read(altered_root, &values).unwrap_err();
            assert_eq!(error.code, "HISTORY_PACKAGE_INVALID");
        }
    }

    #[test]
    fn identical_field_maps_share_objects_and_empty_maps_have_no_index() {
        let manifest = manifest();
        let (_, root, values) = encoded(&manifest, MAX_METADATA);
        assert_eq!(values.len(), 2, "one shared empty leaf and one root");
        assert!(
            root["maps"]
                .as_object()
                .unwrap()
                .values()
                .all(|reference| reference["kind"] == "state-page")
        );
        assert_eq!(read(root, &values).unwrap().1.len(), 2);

        let mut manifest = manifest;
        for n in 0..300 {
            let key = format!("file-{n}");
            manifest.file_notes.insert(key.clone(), format!("note-{n}"));
            manifest.version_notes.insert(key, format!("note-{n}"));
        }
        let (_, root, values) = encoded(&manifest, MAX_METADATA);
        assert_eq!(root["maps"]["file_notes"], root["maps"]["version_notes"]);
        let (resolved, _) = read(root, &values).unwrap();
        assert_eq!(resolved.file_notes, manifest.file_notes);
        assert_eq!(resolved.version_notes, manifest.version_notes);
    }

    #[test]
    fn changing_one_entity_reuses_unrelated_radix_pages() {
        let mut manifest = manifest();
        for n in 0..600 {
            manifest
                .file_notes
                .insert(format!("entity-{n}"), format!("note-{n}"));
        }
        let (old_hash, old_root, old_objects) = encoded(&manifest, MAX_METADATA);
        manifest
            .file_notes
            .insert("entity-10".to_owned(), "updated".to_owned());
        let (new_hash, new_root, new_objects) = encoded(&manifest, MAX_METADATA);
        assert_ne!(old_hash, new_hash);
        assert_eq!(old_root["maps"]["branches"], new_root["maps"]["branches"]);
        let old_pages = old_objects
            .keys()
            .filter(|(kind, _)| kind == "state-page")
            .collect::<BTreeSet<_>>();
        let new_pages = new_objects
            .keys()
            .filter(|(kind, _)| kind == "state-page")
            .collect::<BTreeSet<_>>();
        assert_eq!(old_pages.difference(&new_pages).count(), 1);
        assert_eq!(new_pages.difference(&old_pages).count(), 1);
    }
}
