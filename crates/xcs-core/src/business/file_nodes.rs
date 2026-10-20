//! Common identity, ancestry and inherited labels for every public file node.
use super::*;
use std::collections::BTreeSet;

pub(crate) fn node_version(state: &State, file: &str, node: &str) -> Result<String> {
    let version = state
        .versions
        .get(node)
        .filter(|v| v.file_id == file)
        .map(|v| v.version_id.clone())
        .or_else(|| {
            state
                .file_path_nodes
                .get(node)
                .filter(|n| n.file_id == file)
                .map(|n| n.version_id.clone())
        })
        .or_else(|| {
            state
                .file_metadata_nodes
                .get(node)
                .filter(|n| n.file_id == file)
                .map(|n| n.version_id.clone())
        })
        .ok_or_else(|| Error::new("FILE_NODE_NOT_FOUND", "node does not belong to this file"))?;
    if !state
        .versions
        .get(&version)
        .is_some_and(|v| v.file_id == file)
    {
        return Err(Error::new("STATE_INVALID", "file node body is missing"));
    }
    Ok(version)
}

pub(crate) fn base_parents(state: &State, node: &str) -> Vec<String> {
    if let Some(n) = state.file_metadata_nodes.get(node) {
        n.parent_node_ids.clone()
    } else if let Some(n) = state.file_path_nodes.get(node) {
        n.parent_node_ids.clone()
    } else if let Some(v) = state.versions.get(node) {
        state
            .file_node_parents
            .get(node)
            .cloned()
            .unwrap_or_else(|| v.source_version_ids.clone())
    } else {
        Vec::new()
    }
}

pub(crate) fn node_parents(state: &State, node: &str) -> Vec<String> {
    let mut parents = base_parents(state, node);
    for event in state
        .file_branch_merges
        .values()
        .filter(|m| m.result_node_id == node)
    {
        for parent in &event.parent_node_ids {
            if parent != node && !parents.contains(parent) {
                parents.push(parent.clone());
            }
        }
    }
    parents
}

/// Ordinary child nodes inherit their first parent's immutable tag snapshot.
/// A merge revision stores the explicitly selected result node's tags.
pub(crate) fn node_tags(state: &State, node: &str) -> BTreeSet<String> {
    let mut current = node.to_owned();
    let mut seen = BTreeSet::new();
    while seen.insert(current.clone()) {
        if let Some(n) = state.file_metadata_nodes.get(&current) {
            return n.tags.clone();
        }
        let Some(parent) = base_parents(state, &current).into_iter().next() else {
            break;
        };
        current = parent;
    }
    BTreeSet::new()
}

/// A bounded, immutable-state projection. Never reuse this across publications:
/// metadata nodes and merge provenance can change the meaning of an old node.
pub(crate) struct NodeProjection<'a> {
    state: &'a State,
    tags: std::collections::BTreeMap<String, std::rc::Rc<BTreeSet<String>>>,
    merges: std::collections::BTreeMap<&'a str, Vec<&'a FileBranchMerge>>,
    #[cfg(test)]
    tag_visits: usize,
}

impl<'a> NodeProjection<'a> {
    pub(crate) fn new(state: &'a State) -> Self {
        let mut merges = std::collections::BTreeMap::<_, Vec<_>>::new();
        // Preserve the original map iteration order within each result node.
        for event in state.file_branch_merges.values() {
            merges
                .entry(event.result_node_id.as_str())
                .or_default()
                .push(event);
        }
        Self {
            state,
            tags: Default::default(),
            merges,
            #[cfg(test)]
            tag_visits: 0,
        }
    }

    pub(crate) fn parents(&self, node: &str) -> Vec<String> {
        let mut parents = base_parents(self.state, node);
        if let Some(events) = self.merges.get(node) {
            for event in events {
                for parent in &event.parent_node_ids {
                    if parent != node && !parents.contains(parent) {
                        parents.push(parent.clone());
                    }
                }
            }
        }
        parents
    }

    pub(crate) fn tags(&mut self, node: &str) -> &BTreeSet<String> {
        if !self.tags.contains_key(node) {
            let mut current = node;
            let mut path = Vec::new();
            let mut seen = BTreeSet::new();
            let tags = loop {
                if let Some(tags) = self.tags.get(current) {
                    break tags.clone();
                }
                if !seen.insert(current) {
                    break std::rc::Rc::new(BTreeSet::new());
                }
                path.push(current);
                #[cfg(test)]
                {
                    self.tag_visits += 1;
                }
                // Metadata wins even for overlapping IDs and cyclic parents.
                if let Some(metadata) = self.state.file_metadata_nodes.get(current) {
                    break std::rc::Rc::new(metadata.tags.clone());
                }
                let parents = if let Some(path) = self.state.file_path_nodes.get(current) {
                    Some(&path.parent_node_ids)
                } else if let Some(version) = self.state.versions.get(current) {
                    Some(
                        self.state
                            .file_node_parents
                            .get(current)
                            .unwrap_or(&version.source_version_ids),
                    )
                } else {
                    None
                };
                let Some(parent) = parents.and_then(|parents| parents.first()) else {
                    break std::rc::Rc::new(BTreeSet::new());
                };
                current = parent;
            };
            // Every visited first-parent prefix has the same snapshot, including
            // dangling tails and cycles without metadata. Share the snapshot.
            for visited in path {
                self.tags.insert(visited.to_owned(), tags.clone());
            }
        }
        self.tags
            .get(node)
            .expect("visited node is memoized")
            .as_ref()
    }
}

pub(crate) fn branch_head(state: &State, branch: &str) -> Result<String> {
    let branch = state
        .branches
        .get(branch)
        .ok_or_else(|| Error::new("FILE_BRANCH_NOT_FOUND", "file branch missing"))?;
    let file = branch.file_id.as_deref().ok_or_else(|| {
        Error::new(
            "FILE_BRANCH_REQUIRED",
            "an independent file branch is required",
        )
    })?;
    let node = branch
        .file_node_id
        .clone()
        .or_else(|| {
            state
                .commits
                .get(&branch.head_commit_id)
                .and_then(|c| {
                    c.entries
                        .iter()
                        .find(|e| e.kind == "file" && e.file_id == file)
                })
                .map(|e| e.version_id.clone())
        })
        .ok_or_else(|| Error::new("STATE_INVALID", "file branch head is missing"))?;
    node_version(state, file, &node)?;
    Ok(node)
}

impl Engine {
    pub fn file_node_version(&self, file: &str, node: &str) -> Result<String> {
        node_version(&self.state, file, node)
    }
    pub fn file_node_tags(&self, file: &str, node: &str) -> Result<BTreeSet<String>> {
        node_version(&self.state, file, node)?;
        Ok(node_tags(&self.state, node))
    }
    pub fn file_node_parents(&self, file: &str, node: &str) -> Result<Vec<String>> {
        node_version(&self.state, file, node)?;
        Ok(node_parents(&self.state, node))
    }
    pub fn file_branch_head(&self, branch: &str) -> Result<String> {
        branch_head(&self.state, branch)
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn path(state: &mut State, id: &str, parents: &[&str]) {
        state.file_path_nodes.insert(
            id.into(),
            FilePathNode {
                node_id: id.into(),
                history_tree_id: "history".into(),
                file_id: "file".into(),
                version_id: "body".into(),
                parent_node_ids: parents.iter().map(|p| (*p).into()).collect(),
                branch_id: "branch".into(),
                path: "new".into(),
                previous_path: "old".into(),
                operation_id: "operation".into(),
                author: "author".into(),
                sequence: 1,
                created_at: 1,
            },
        );
    }

    fn metadata(state: &mut State, id: &str, parents: &[&str], tags: &[&str]) {
        state.file_metadata_nodes.insert(
            id.into(),
            FileMetadataNode {
                node_id: id.into(),
                history_tree_id: "history".into(),
                file_id: "file".into(),
                version_id: "body".into(),
                parent_node_ids: parents.iter().map(|p| (*p).into()).collect(),
                branch_id: "branch".into(),
                kind: "tags".into(),
                tags: tags.iter().map(|t| (*t).into()).collect(),
                operation_id: "operation".into(),
                author: "author".into(),
                sequence: 1,
                created_at: 1,
            },
        );
    }

    fn merge(state: &mut State, id: &str, result: &str, parents: &[&str]) {
        state.file_branch_merges.insert(
            id.into(),
            FileBranchMerge {
                merge_id: id.into(),
                history_tree_id: "history".into(),
                file_id: "file".into(),
                input_branch_ids: Vec::new(),
                input_node_ids: Vec::new(),
                chosen_node_id: result.into(),
                result_node_id: result.into(),
                surviving_branch_id: "branch".into(),
                reused_node: true,
                parent_node_ids: parents.iter().map(|p| (*p).into()).collect(),
                operation_id: "operation".into(),
                sequence: 1,
            },
        );
    }

    #[test]
    fn projection_matches_uncached_nodes_and_serialized_bytes() {
        let mut state = State::default();
        metadata(&mut state, "tagged", &["tagged"], &["z", "a"]);
        metadata(&mut state, "empty", &["tagged"], &[]);
        path(&mut state, "tagged", &["missing"]); // metadata wins over a path ID
        path(&mut state, "left", &["tagged"]);
        path(&mut state, "right", &["empty"]);
        path(&mut state, "merged", &["left", "left", "right"]);
        path(&mut state, "cycle-a", &["cycle-b"]);
        path(&mut state, "cycle-b", &["cycle-a"]);
        path(&mut state, "cycle-tail", &["cycle-a", "tagged"]);
        path(&mut state, "dangling", &["missing"]);
        // Reverse insertion order must not change BTreeMap merge order.
        merge(&mut state, "z", "merged", &["other", "left", "merged"]);
        merge(&mut state, "a", "merged", &["right", "first", "first"]);
        merge(&mut state, "unrelated", "elsewhere", &["tagged"]);
        for (id, parents) in [
            ("body", vec!["left"]),
            ("source-only", vec!["left"]),
            ("override", vec!["right"]),
            ("empty-override", vec!["left"]),
        ] {
            state.versions.insert(
                id.into(),
                FileVersion {
                    format_version: 2,
                    history_tree_id: "history".into(),
                    version_id: id.into(),
                    file_id: "file".into(),
                    creation_primary_slot_id: "P01".into(),
                    logical_size: 0,
                    version_sequence: 1,
                    content_hash: "0".repeat(64),
                    checksum_chunk_bytes: 262144,
                    checksum_manifest_hash: "0".repeat(64),
                    creation_name: "body".into(),
                    storage_name: None,
                    created_at: None,
                    source_version_ids: parents.into_iter().map(str::to_owned).collect(),
                    created_as_full_checkpoint: true,
                },
            );
        }
        state
            .file_node_parents
            .insert("override".into(), vec!["left".into()]);
        state
            .file_node_parents
            .insert("empty-override".into(), Vec::new());
        // A path node takes precedence over a body with the same ID.
        path(&mut state, "body", &["right"]);
        let nodes = [
            "body",
            "source-only",
            "override",
            "empty-override",
            "merged",
            "right",
            "left",
            "tagged",
            "empty",
            "cycle-tail",
            "cycle-b",
            "cycle-a",
            "dangling",
            "missing",
        ];
        let mut projection = NodeProjection::new(&state);
        for node in nodes.into_iter().chain(nodes.into_iter().rev()) {
            let expected = serde_json::json!({"parent_node_ids": node_parents(&state, node), "tags": node_tags(&state, node)});
            let actual = serde_json::json!({"parent_node_ids": projection.parents(node), "tags": projection.tags(node)});
            assert_eq!(
                serde_json::to_vec(&actual).unwrap(),
                serde_json::to_vec(&expected).unwrap(),
                "{node}"
            );
        }
        assert_eq!(
            projection.parents("merged"),
            ["left", "left", "right", "first", "other"]
        );
    }

    #[test]
    fn projection_walks_a_linear_history_once_in_either_query_order() {
        const LENGTH: usize = 4096;
        let mut state = State::default();
        metadata(&mut state, "0", &[], &["tag"]);
        for n in 1..LENGTH {
            path(&mut state, &n.to_string(), &[&(n - 1).to_string()]);
        }
        for reverse in [false, true] {
            let mut projection = NodeProjection::new(&state);
            for n in 0..LENGTH {
                let id = if reverse { LENGTH - n - 1 } else { n }.to_string();
                assert_eq!(projection.tags(&id), &BTreeSet::from(["tag".into()]));
            }
            assert_eq!(projection.tag_visits, LENGTH);
            for n in 0..LENGTH {
                projection.tags(&n.to_string());
            }
            assert_eq!(
                projection.tag_visits, LENGTH,
                "replica lookups must not traverse again"
            );
        }
    }
}
