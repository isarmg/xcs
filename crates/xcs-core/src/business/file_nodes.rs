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
