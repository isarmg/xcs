//! Independently recoverable, publication-bound history packages.
use crate::{business::validate_entries, disk::*, journal::*, *};
use data_serialization::{Deserialize, Serialize};
use json_codec::json;
mod graph;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serialization(deny_unknown_fields)]
pub struct HistoryManifest {
    pub repository: Repository,
    pub history: HistoryTree,
    pub branches: BTreeMap<String, Branch>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_selections: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_branches: BTreeMap<String, DirectoryBranch>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_trees: BTreeMap<String, DirectoryTree>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_write_origins: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_contexts: BTreeMap<String, FileContext>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_path_nodes: BTreeMap<String, FilePathNode>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_metadata_nodes: BTreeMap<String, FileMetadataNode>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_branch_merges: BTreeMap<String, FileBranchMerge>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_branch_aliases: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_node_parents: BTreeMap<String, Vec<String>>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_path_notes: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_tree_selections: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_locations: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_backup_paths: BTreeMap<String, BTreeMap<String, String>>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_backup_policies: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_selections: BTreeMap<String, String>,
    #[serialization(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "storage_contract::u64_string_map"
    )]
    pub directory_selection_generations: BTreeMap<String, u64>,
    #[serialization(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "storage_contract::u64_string_map"
    )]
    pub file_selection_generations: BTreeMap<String, u64>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_notes: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub version_notes: BTreeMap<String, String>,
    #[serialization(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_names: BTreeMap<String, String>,
    pub commits: BTreeMap<String, Commit>,
    pub versions: BTreeMap<String, FileVersion>,
    pub placements: BTreeMap<String, Placement>,
    pub merge_links: BTreeMap<String, MergeLink>,
    pub boundary_marks: BTreeMap<String, HistoryBoundary>,
    pub content_states: BTreeMap<String, String>,
    pub placement_rules: BTreeMap<String, PlacementRule>,
    pub protection_policies: BTreeMap<String, ProtectionPolicy>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serialization(deny_unknown_fields)]
pub struct HistoryPackageReport {
    pub history_tree_id: String,
    pub repository_id: String,
    pub manifest_hash: String,
    #[serialization(with = "storage_contract::u64_string")]
    pub publication_sequence: u64,
    pub commits: usize,
    pub versions: usize,
    pub full_files: usize,
    pub tombstones: usize,
    #[serialization(with = "storage_contract::u64_string")]
    pub full_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serialization(deny_unknown_fields)]
struct PackageProof {
    manifest_hash: String,
    candidate: Candidate,
    publication: Publication,
    publication_seal: PublicationSeal,
}
pub(crate) fn package_proof_bytes(
    candidate: &Candidate,
    publication: &Publication,
    seal: &PublicationSeal,
    maximum: usize,
) -> Result<usize> {
    Ok(encode_metadata(
        &PackageProof {
            manifest_hash: "0".repeat(64),
            candidate: candidate.clone(),
            publication: publication.clone(),
            publication_seal: seal.clone(),
        },
        maximum,
    )?
    .len())
}
impl HistoryManifest {
    fn from_state(state: &State, history_id: &str) -> Result<Self> {
        let history = state
            .histories
            .get(history_id)
            .cloned()
            .ok_or_else(|| Error::new("HISTORY_NOT_FOUND", "history does not exist"))?;
        let repository = state
            .repositories
            .get(&history.repository_id)
            .cloned()
            .ok_or_else(|| Error::new("STATE_INVALID", "history repository missing"))?;
        let repository_id = history.repository_id.clone();
        let directory_ids = state
            .commits
            .values()
            .filter(|commit| commit.history_tree_id == history_id)
            .flat_map(|commit| {
                commit
                    .entries
                    .iter()
                    .filter(|entry| entry.kind == "directory")
                    .map(|entry| entry.entry_id.as_str())
            })
            .chain(std::iter::once(repository_id.as_str()))
            .collect::<BTreeSet<_>>();
        let versions = state
            .versions
            .iter()
            .filter(|(_, v)| v.history_tree_id == history_id)
            .map(|(id, v)| (id.clone(), v.clone()))
            .collect::<BTreeMap<_, _>>();
        let file_ids = versions
            .values()
            .map(|version| version.file_id.as_str())
            .collect::<BTreeSet<_>>();
        let manifest = Self {
            repository,
            history,
            file_names: state
                .file_names
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, name)| (file.clone(), name.clone()))
                .collect(),
            file_locations: state
                .file_locations
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, path)| (file.clone(), path.clone()))
                .collect(),
            file_write_origins: state
                .file_write_origins
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, origin)| (file.clone(), origin.clone()))
                .collect(),
            file_contexts: state
                .file_contexts
                .iter()
                .filter(|(key, _)| key.starts_with(&format!("{repository_id}/")))
                .map(|(key, context)| (key.clone(), context.clone()))
                .collect(),
            file_path_nodes: state
                .file_path_nodes
                .iter()
                .filter(|(_, node)| node.history_tree_id == history_id)
                .map(|(id, node)| (id.clone(), node.clone()))
                .collect(),
            file_metadata_nodes: state
                .file_metadata_nodes
                .iter()
                .filter(|(_, node)| node.history_tree_id == history_id)
                .map(|(id, node)| (id.clone(), node.clone()))
                .collect(),
            file_branch_merges: state
                .file_branch_merges
                .iter()
                .filter(|(_, event)| event.history_tree_id == history_id)
                .map(|(id, event)| (id.clone(), event.clone()))
                .collect(),
            file_branch_aliases: state
                .file_branch_aliases
                .iter()
                .filter(|(id, _)| {
                    state
                        .branches
                        .get(*id)
                        .is_some_and(|branch| branch.history_tree_id == history_id)
                })
                .map(|(id, target)| (id.clone(), target.clone()))
                .collect(),
            file_backup_paths: state
                .file_backup_paths
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, paths)| (file.clone(), paths.clone()))
                .collect(),
            file_backup_policies: state
                .file_backup_policies
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, policy)| (file.clone(), policy.clone()))
                .collect(),
            file_node_parents: state
                .file_node_parents
                .iter()
                .filter(|(version, _)| versions.contains_key(*version))
                .map(|(version, parents)| (version.clone(), parents.clone()))
                .collect(),
            file_path_notes: state
                .file_path_notes
                .iter()
                .filter(|(id, _)| {
                    state
                        .file_path_nodes
                        .get(*id)
                        .is_some_and(|node| node.history_tree_id == history_id)
                        || state
                            .file_metadata_nodes
                            .get(*id)
                            .is_some_and(|node| node.history_tree_id == history_id)
                })
                .map(|(id, note)| (id.clone(), note.clone()))
                .collect(),
            file_selection_generations: state
                .file_selection_generations
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, generation)| (file.clone(), *generation))
                .collect(),
            directory_tree_selections: state
                .directory_tree_selections
                .iter()
                .filter(|(directory, _)| directory_ids.contains(directory.as_str()))
                .map(|(directory, tree)| (directory.clone(), tree.clone()))
                .collect(),
            file_notes: state
                .file_notes
                .iter()
                .filter(|(file, _)| file_ids.contains(file.as_str()))
                .map(|(file, note)| (file.clone(), note.clone()))
                .collect(),
            version_notes: state
                .version_notes
                .iter()
                .filter(|(version, _)| versions.contains_key(*version))
                .map(|(version, note)| (version.clone(), note.clone()))
                .collect(),
            file_selections: state
                .file_selections
                .iter()
                .filter(|(_, id)| {
                    state
                        .branches
                        .get(*id)
                        .is_some_and(|b| b.history_tree_id == history_id)
                })
                .map(|(file, branch)| (file.clone(), branch.clone()))
                .collect(),
            directory_branches: state
                .directory_branches
                .iter()
                .filter(|(_, branch)| branch.repository_id == repository_id)
                .map(|(id, branch)| (id.clone(), branch.clone()))
                .collect(),
            directory_trees: state
                .directory_trees
                .iter()
                .filter(|(_, tree)| {
                    state
                        .directory_branches
                        .get(&tree.directory_branch_id)
                        .is_some_and(|branch| branch.repository_id == repository_id)
                })
                .map(|(id, tree)| (id.clone(), tree.clone()))
                .collect(),
            directory_selections: state
                .directory_selections
                .iter()
                .filter(|(_, selected)| {
                    state
                        .directory_branches
                        .get(*selected)
                        .is_some_and(|branch| branch.repository_id == repository_id)
                })
                .map(|(directory, selected)| (directory.clone(), selected.clone()))
                .collect(),
            directory_selection_generations: state
                .directory_selection_generations
                .iter()
                .filter(|(directory, _)| directory_ids.contains(directory.as_str()))
                .map(|(directory, generation)| (directory.clone(), *generation))
                .collect(),
            branches: state
                .branches
                .iter()
                .filter(|(_, b)| b.history_tree_id == history_id)
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
            commits: state
                .commits
                .iter()
                .filter(|(_, c)| c.history_tree_id == history_id)
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
            placements: state
                .placements
                .iter()
                .filter(|(id, _)| versions.contains_key(*id))
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
            content_states: versions
                .keys()
                .map(|id| {
                    (
                        id.clone(),
                        state
                            .content_states
                            .get(id)
                            .cloned()
                            .unwrap_or_else(|| "AVAILABLE".into()),
                    )
                })
                .collect(),
            versions,
            merge_links: state
                .merge_links
                .iter()
                .filter(|(_, l)| l.history_tree_id == history_id)
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
            boundary_marks: state
                .history_boundaries
                .iter()
                .filter(|(_, b)| b.history_tree_id == history_id)
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
            placement_rules: state
                .placement_rules
                .iter()
                .filter(|(_, p)| p.repository_id == repository_id)
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
            protection_policies: state
                .protection_policies
                .iter()
                .filter(|(_, p)| p.repository_id == repository_id)
                .map(|(id, v)| (id.clone(), v.clone()))
                .collect(),
        };
        manifest.validate()?;
        Ok(manifest)
    }
    fn validate_directory_snapshot(&self, branch: &DirectoryBranch) -> Result<()> {
        let invalid = |message| Error::new("HISTORY_PACKAGE_INVALID", message);
        let history = &self.history.history_tree_id;
        let commit = self
            .commits
            .get(&branch.commit_id)
            .ok_or_else(|| invalid("directory composition commit is outside the history"))?;
        let scope_key = name_policy::path_key(&branch.path);
        if branch.path.is_empty() {
            if branch.directory_id != self.repository.repository_id {
                return Err(invalid(
                    "root directory identity differs from its repository",
                ));
            }
        } else {
            name_policy::validate_path(&branch.path)?;
            validate_id(&branch.directory_id, "entry")?;
            if !commit.entries.iter().any(|entry| {
                entry.kind == "directory"
                    && entry.entry_id == branch.directory_id
                    && entry.lookup_key == scope_key
            }) {
                return Err(invalid(
                    "directory composition stable identity is absent from its snapshot",
                ));
            }
        }
        let prefix = format!("{scope_key}/");
        let mut paths = BTreeSet::new();
        let mut entries = BTreeMap::<&str, &Entry>::new();
        let mut files = BTreeSet::new();
        for entry in &branch.entries {
            validate_id(&entry.entry_id, "entry")?;
            name_policy::validate_path(&entry.path)?;
            if (!scope_key.is_empty() && !entry.lookup_key.starts_with(&prefix))
                || entry.entry_id == branch.directory_id
                || entry.name != entry.path.rsplit('/').next().unwrap_or("")
                || entry.lookup_key != name_policy::path_key(&entry.path)
                || !paths.insert(entry.lookup_key.as_str())
            {
                return Err(invalid(
                    "directory composition entry escapes its scope or normalized identity",
                ));
            }
            if let Some(previous) = entries.insert(entry.entry_id.as_str(), entry)
                && (entry.kind != "file"
                    || previous.kind != "file"
                    || previous.file_id != entry.file_id)
            {
                return Err(invalid(
                    "directory composition entry identity is reused by different entities",
                ));
            }
            match entry.kind.as_str() {
                "file" => {
                    validate_id(&entry.file_id, "file")?;
                    validate_id(&entry.version_id, "ver")?;
                    let version = self
                        .versions
                        .get(&entry.version_id)
                        .ok_or_else(|| invalid("directory composition version is missing"))?;
                    let shared = branch
                        .file_branches
                        .get(&entry.file_id)
                        .and_then(|id| self.branches.get(id))
                        .ok_or_else(|| {
                            invalid("directory composition shared file branch is missing")
                        })?;
                    files.insert(entry.file_id.as_str());
                    if version.history_tree_id != *history
                        || version.file_id != entry.file_id
                        || version.logical_size != entry.size
                        || shared.history_tree_id != *history
                        || shared.branch_kind != "file"
                        || shared.file_id.as_deref() != Some(entry.file_id.as_str())
                    {
                        return Err(invalid(
                            "directory composition file, version or shared branch differs",
                        ));
                    }
                }
                "directory"
                    if entry.file_id.is_empty()
                        && entry.version_id.is_empty()
                        && entry.size == 0 => {}
                _ => {
                    return Err(invalid(
                        "directory composition entry kind or directory payload is invalid",
                    ));
                }
            }
        }
        if branch.file_branches.len() != files.len() {
            return Err(invalid(
                "directory composition file branch coverage differs",
            ));
        }
        let file_entries = branch
            .entries
            .iter()
            .filter(|entry| entry.kind == "file")
            .collect::<Vec<_>>();
        if branch.file_contexts.len() != file_entries.len() {
            return Err(invalid("directory path context coverage differs"));
        }
        for entry in file_entries {
            let context = branch
                .file_contexts
                .get(&entry.lookup_key)
                .ok_or_else(|| invalid("directory path write context is missing"))?;
            self.validate_file_context(context, &entry.file_id, &entry.version_id)?;
        }
        for (file, origin) in &branch.inherited_files {
            if !branch.file_branches.contains_key(file)
                || !self
                    .directory_branches
                    .get(origin)
                    .is_some_and(|origin| origin.repository_id == branch.repository_id)
            {
                return Err(invalid(
                    "inherited write intent escapes its directory history or file scope",
                ));
            }
        }
        for (directory, reference) in &branch.directory_refs {
            let selected = self
                .directory_branches
                .get(&reference.directory_branch_id)
                .ok_or_else(|| invalid("directory composition child branch is missing"))?;
            let tree = self
                .directory_trees
                .get(&reference.tree_id)
                .ok_or_else(|| invalid("directory composition child tree is missing"))?;
            let owner = self
                .directory_branches
                .get(&tree.directory_branch_id)
                .ok_or_else(|| invalid("directory composition child tree owner is missing"))?;
            if !branch
                .entries
                .iter()
                .any(|entry| entry.kind == "directory" && &entry.entry_id == directory)
                || selected.directory_id != *directory
                || owner.directory_id != *directory
                || selected.repository_id != branch.repository_id
                || owner.repository_id != branch.repository_id
                || !self.directory_branch_inherits(
                    &reference.directory_branch_id,
                    &tree.directory_branch_id,
                )
            {
                return Err(invalid(
                    "directory composition child reference escapes its stable scope",
                ));
            }
        }
        Ok(())
    }
    fn directory_branch_inherits(&self, selected: &str, ancestor: &str) -> bool {
        let mut cursor = Some(selected);
        let mut seen = BTreeSet::new();
        while let Some(id) = cursor {
            if id == ancestor {
                return true;
            }
            if !seen.insert(id) {
                return false;
            }
            cursor = self
                .directory_branches
                .get(id)
                .and_then(|branch| branch.parent_branch_id.as_deref());
        }
        false
    }
    fn validate_file_context(
        &self,
        context: &FileContext,
        file_id: &str,
        version_id: &str,
    ) -> Result<()> {
        let invalid = |message| Error::new("HISTORY_PACKAGE_INVALID", message);
        let branch = self
            .branches
            .get(&context.branch_id)
            .ok_or_else(|| invalid("file path context branch is missing"))?;
        if branch.branch_kind != "file"
            || branch.file_id.as_deref() != Some(file_id)
            || branch.history_tree_id != self.history.history_tree_id
        {
            return Err(invalid(
                "file path context belongs to another file or history",
            ));
        }
        if context
            .inherited_from
            .as_ref()
            .is_some_and(|origin| !self.directory_branches.contains_key(origin))
        {
            return Err(invalid("file path inherited context origin is missing"));
        }
        if let Some(node) = &context.node_id {
            let body = self.file_node_version(node)?;
            if body.file_id != file_id || body.version_id != version_id {
                return Err(invalid(
                    "file path node differs from its frozen content reference",
                ));
            }
        }
        Ok(())
    }
    fn file_node_version(&self, node_id: &str) -> Result<&FileVersion> {
        self.versions
            .get(node_id)
            .or_else(|| {
                self.file_path_nodes
                    .get(node_id)
                    .and_then(|node| self.versions.get(&node.version_id))
            })
            .or_else(|| {
                self.file_metadata_nodes
                    .get(node_id)
                    .and_then(|node| self.versions.get(&node.version_id))
            })
            .ok_or_else(|| {
                Error::new(
                    "HISTORY_PACKAGE_INVALID",
                    "file history node or its body reference is missing",
                )
            })
    }
    fn validate_file_nodes(&self) -> Result<()> {
        let invalid = |message| Error::new("HISTORY_PACKAGE_INVALID", message);
        let mut graph = BTreeMap::<&str, Vec<String>>::new();
        for (id, version) in &self.versions {
            graph.insert(
                id,
                self.file_node_parents
                    .get(id)
                    .unwrap_or(&version.source_version_ids)
                    .clone(),
            );
        }
        for (id, node) in &self.file_path_nodes {
            validate_id(id, "fnode")?;
            validate_id(&node.operation_id, "op")?;
            name_policy::validate_path(&node.path)?;
            name_policy::validate_path(&node.previous_path)?;
            let body = self
                .versions
                .get(&node.version_id)
                .ok_or_else(|| invalid("path event body version is missing"))?;
            let branch = self
                .branches
                .get(&node.branch_id)
                .ok_or_else(|| invalid("path event public branch is missing"))?;
            if node.node_id != *id
                || node.history_tree_id != self.history.history_tree_id
                || body.file_id != node.file_id
                || body.history_tree_id != node.history_tree_id
                || branch.file_id.as_deref() != Some(node.file_id.as_str())
                || branch.branch_kind != "file"
                || branch.history_tree_id != node.history_tree_id
                || node.author.is_empty()
                || node.path == node.previous_path
                || node.parent_node_ids.is_empty()
            {
                return Err(invalid(
                    "file path event identity, scope or metadata differs",
                ));
            }
            for parent in &node.parent_node_ids {
                let parent = self.file_node_version(parent)?;
                if parent.version_id != node.version_id {
                    return Err(invalid("rename event changes its body content"));
                }
            }
            graph.insert(id, node.parent_node_ids.clone());
        }
        for (id, node) in &self.file_metadata_nodes {
            validate_id(id, "fmeta")?;
            validate_id(&node.operation_id, "op")?;
            let body = self
                .versions
                .get(&node.version_id)
                .ok_or_else(|| invalid("file metadata body version is missing"))?;
            let branch = self
                .branches
                .get(&node.branch_id)
                .ok_or_else(|| invalid("file metadata public branch is missing"))?;
            if node.node_id != *id
                || node.history_tree_id != self.history.history_tree_id
                || body.file_id != node.file_id
                || body.history_tree_id != node.history_tree_id
                || branch.file_id.as_deref() != Some(node.file_id.as_str())
                || branch.branch_kind != "file"
                || branch.history_tree_id != node.history_tree_id
                || node.author.is_empty()
                || node.parent_node_ids.is_empty()
                || !["tags", "merge"].contains(&node.kind.as_str())
                || node.tags.len() > storage_contract::MAX_FILE_TAGS
                || node
                    .tags
                    .iter()
                    .any(|tag| storage_contract::validate_tag(tag).is_err())
            {
                return Err(invalid("file metadata identity, scope or labels differ"));
            }
            if node.kind == "tags"
                && (node.parent_node_ids.len() != 1
                    || self.file_node_version(&node.parent_node_ids[0])?.version_id
                        != node.version_id)
            {
                return Err(invalid("tag event changes its body content"));
            }
            graph.insert(id, node.parent_node_ids.clone());
        }
        for (id, event) in &self.file_branch_merges {
            validate_id(id, "fmerge")?;
            validate_id(&event.operation_id, "op")?;
            let result = self.file_node_version(&event.result_node_id)?;
            let chosen = self.file_node_version(&event.chosen_node_id)?;
            let inputs = event.input_branch_ids.iter().collect::<BTreeSet<_>>();
            if event.merge_id != *id
                || event.history_tree_id != self.history.history_tree_id
                || result.file_id != event.file_id
                || result.history_tree_id != event.history_tree_id
                || chosen.version_id != result.version_id
                || inputs.len() < 2
                || inputs.len() != event.input_branch_ids.len()
                || event.input_branch_ids.len() != event.input_node_ids.len()
                || !inputs.contains(&event.surviving_branch_id)
                || event.reused_node != event.input_node_ids.contains(&event.chosen_node_id)
                || event.reused_node != (event.result_node_id == event.chosen_node_id)
            {
                return Err(invalid("file merge provenance identity or result differs"));
            }
            for (branch, node) in event.input_branch_ids.iter().zip(&event.input_node_ids) {
                let branch = self
                    .branches
                    .get(branch)
                    .ok_or_else(|| invalid("file merge input branch is missing"))?;
                let node = self.file_node_version(node)?;
                if branch.branch_kind != "file"
                    || branch.file_id.as_deref() != Some(event.file_id.as_str())
                    || branch.history_tree_id != event.history_tree_id
                    || node.file_id != event.file_id
                    || node.history_tree_id != event.history_tree_id
                {
                    return Err(invalid("file merge input escapes its file or history"));
                }
            }
            if !event.reused_node
                && !self
                    .file_metadata_nodes
                    .get(&event.result_node_id)
                    .is_some_and(|node| {
                        node.kind == "merge"
                            && node.parent_node_ids == event.parent_node_ids
                            && node.branch_id == event.surviving_branch_id
                    })
            {
                return Err(invalid("file merge revision metadata differs"));
            }
            let permitted = event
                .input_node_ids
                .iter()
                .chain(std::iter::once(&event.chosen_node_id))
                .collect::<BTreeSet<_>>();
            let mut unique = BTreeSet::new();
            for parent in &event.parent_node_ids {
                if !permitted.contains(parent)
                    || parent == &event.result_node_id
                    || !unique.insert(parent)
                {
                    return Err(invalid("file merge graph edge is not a unique input"));
                }
                let parents = graph
                    .get_mut(event.result_node_id.as_str())
                    .ok_or_else(|| invalid("file merge result graph node is missing"))?;
                if !parents.contains(parent) {
                    parents.push(parent.clone());
                }
            }
        }
        for (source, target) in &self.file_branch_aliases {
            let original = self
                .branches
                .get(source)
                .ok_or_else(|| invalid("merged file alias source is missing"))?;
            if original.branch_kind != "file" || original.state != "merged" || source == target {
                return Err(invalid("merged file alias source is invalid"));
            }
            let mut cursor = target;
            let mut seen = BTreeSet::from([source]);
            loop {
                if !seen.insert(cursor) {
                    return Err(invalid("merged file aliases contain a cycle"));
                }
                let branch = self
                    .branches
                    .get(cursor)
                    .ok_or_else(|| invalid("merged file alias target is missing"))?;
                if branch.branch_kind != "file"
                    || branch.file_id != original.file_id
                    || branch.history_tree_id != original.history_tree_id
                {
                    return Err(invalid("merged file alias escapes its file or history"));
                }
                match self.file_branch_aliases.get(cursor) {
                    Some(next) => cursor = next,
                    None => {
                        if branch.state != "active" {
                            return Err(invalid("merged file alias final target is not active"));
                        }
                        break;
                    }
                }
            }
        }
        for (version, parents) in &self.file_node_parents {
            let body = self
                .versions
                .get(version)
                .ok_or_else(|| invalid("file node ancestry override has no real version"))?;
            let resolved = parents
                .iter()
                .map(|parent| {
                    self.file_node_version(parent)
                        .map(|version| version.version_id.as_str())
                })
                .collect::<Result<BTreeSet<_>>>()?;
            if resolved != body.source_version_ids.iter().map(String::as_str).collect() {
                return Err(invalid(
                    "file node ancestry override differs from immutable body sources",
                ));
            }
        }
        let mut dependencies = BTreeMap::new();
        let mut children = BTreeMap::<&str, Vec<&str>>::new();
        let mut ready = VecDeque::new();
        for (id, parents) in &graph {
            let body = self.file_node_version(id)?;
            let mut unique = BTreeSet::new();
            for parent in parents {
                let previous = self.file_node_version(parent)?;
                if !unique.insert(parent.as_str())
                    || previous.file_id != body.file_id
                    || previous.history_tree_id != body.history_tree_id
                    || !graph.contains_key(parent.as_str())
                {
                    return Err(invalid(
                        "file history parent repeats or escapes its stable file/history",
                    ));
                }
                children.entry(parent).or_default().push(id);
            }
            dependencies.insert(*id, parents.len());
            if parents.is_empty() {
                ready.push_back(*id);
            }
        }
        let mut resolved = 0usize;
        while let Some(id) = ready.pop_front() {
            resolved += 1;
            for child in children.get(id).into_iter().flatten() {
                let remaining = dependencies.get_mut(child).expect("graph child validated");
                *remaining -= 1;
                if *remaining == 0 {
                    ready.push_back(child);
                }
            }
        }
        if resolved != graph.len() {
            return Err(invalid(
                "file history contains a mixed content/path node cycle",
            ));
        }
        if self.file_path_notes.iter().any(|(id, note)| {
            (!self.file_path_nodes.contains_key(id) && !self.file_metadata_nodes.contains_key(id))
                || note.len() > storage_contract::MAX_NOTE_BYTES
                || note.chars().any(char::is_control)
        }) {
            return Err(invalid(
                "file path notes escape their node scope or exceed the byte limit",
            ));
        }
        Ok(())
    }
    fn validate(&self) -> Result<()> {
        let invalid = |message| Error::new("HISTORY_PACKAGE_INVALID", message);
        let history = &self.history.history_tree_id;
        validate_id(history, "history")?;
        validate_id(&self.repository.repository_id, "repo")?;
        if self.history.repository_id != self.repository.repository_id
            || self.repository.history_tree_id != *history
            || !self.commits.contains_key(&self.history.root_commit_id)
        {
            return Err(invalid("manifest root or repository binding differs"));
        }
        name_policy::validate_name(&self.repository.name)?;
        if self.repository.name_policy != name_policy::NAME_POLICY
            || self.repository.unicode_version != name_policy::UNICODE_VERSION
        {
            return Err(invalid("manifest uses an unsupported naming policy"));
        }
        for (id, commit) in &self.commits {
            validate_id(id, "commit")?;
            if commit.commit_id != *id
                || commit.history_tree_id != *history
                || commit
                    .parent_commit_ids
                    .iter()
                    .any(|p| !self.commits.contains_key(p))
            {
                return Err(invalid("commit identity or parent closure differs"));
            }
            if (id == &self.history.root_commit_id) != commit.parent_commit_ids.is_empty()
                || commit.parent_commit_ids.len() > 1
            {
                return Err(invalid(
                    "history must have one root and single-parent commits",
                ));
            }
            let mut entries = commit.entries.clone();
            validate_entries(&mut entries, 1_000_000)?;
            if canonical_json(&entries)? != canonical_json(&commit.entries)? {
                return Err(invalid(
                    "commit entries are not in canonical namespace form",
                ));
            }
            for entry in commit.entries.iter().filter(|e| e.kind == "file") {
                let version = self
                    .versions
                    .get(&entry.version_id)
                    .ok_or_else(|| invalid("entry version missing"))?;
                if version.file_id != entry.file_id || version.logical_size != entry.size {
                    return Err(invalid("entry version identity or size differs"));
                }
            }
            for (file, source) in &commit.pending_file_versions {
                validate_id(file, "file")?;
                validate_id(source, "ver")?;
                let version = self
                    .versions
                    .get(source)
                    .ok_or_else(|| invalid("pending snapshot source version is missing"))?;
                if version.file_id != *file
                    || version.history_tree_id != *history
                    || !commit.entries.iter().any(|entry| {
                        entry.kind == "file"
                            && entry.file_id == *file
                            && entry.version_id == *source
                    })
                {
                    return Err(invalid(
                        "pending snapshot source escapes its file or entries",
                    ));
                }
            }
        }
        // Reachability also rejects cycles and detached subgraphs.
        let reached = crate::business::descendant_commits(
            self.commits.values(),
            &self.history.root_commit_id,
        );
        if reached.len() != self.commits.len() {
            return Err(invalid("history contains a cycle or detached commits"));
        }
        for (id, branch) in &self.branches {
            validate_id(id, "br")?;
            name_policy::validate_name(&branch.name)?;
            if branch.branch_id != *id
                || branch.history_tree_id != *history
                || !self.commits.contains_key(&branch.head_commit_id)
                || !self.commits.contains_key(&branch.fork_commit_id)
            {
                return Err(invalid("branch has an external live commit reference"));
            }
            if let Some(file) = branch.file_id.as_deref() {
                let head = &self.commits[&branch.head_commit_id];
                if branch.branch_kind != "file"
                    || head.entries.iter().filter(|e| e.kind == "file").count() != 1
                    || !head
                        .entries
                        .iter()
                        .any(|e| e.kind == "file" && e.file_id == file)
                {
                    return Err(invalid("file branch head escapes its stable file identity"));
                }
            }
            if let Some(node) = &branch.file_node_id {
                let body = self.file_node_version(node)?;
                if branch.file_id.as_deref() != Some(body.file_id.as_str())
                    || !self.commits[&branch.head_commit_id]
                        .entries
                        .iter()
                        .any(|entry| entry.kind == "file" && entry.version_id == body.version_id)
                {
                    return Err(invalid("file branch node differs from its HEAD content"));
                }
            }
            for (file, source) in &branch.pending_file_versions {
                validate_id(file, "file")?;
                validate_id(source, "ver")?;
                let version = self
                    .versions
                    .get(source)
                    .ok_or_else(|| invalid("pending branch source version is missing"))?;
                if version.file_id != *file
                    || version.history_tree_id != *history
                    || branch.file_id.as_ref().is_some_and(|id| id != file)
                    || !self.commits[&branch.head_commit_id]
                        .entries
                        .iter()
                        .any(|entry| {
                            entry.kind == "file"
                                && entry.file_id == *file
                                && entry.version_id == *source
                        })
                {
                    return Err(invalid(
                        "pending branch source escapes its file or HEAD snapshot",
                    ));
                }
            }
        }
        for (file, branch) in &self.file_selections {
            if !self
                .branches
                .get(branch)
                .is_some_and(|b| b.file_id.as_deref() == Some(file.as_str()) && b.state == "active")
            {
                return Err(invalid(
                    "selected file branch is missing, inactive or belongs to another file",
                ));
            }
        }
        for (id, branch) in &self.directory_branches {
            validate_id(id, "dbr")?;
            name_policy::validate_name(&branch.name)?;
            if branch.directory_branch_id != *id
                || branch.repository_id != self.repository.repository_id
            {
                return Err(invalid(
                    "directory composition identity or repository differs",
                ));
            }
            // Parents are composition records within the same stable directory;
            // their historical paths may differ after a directory rename.
            if let Some(parent_id) = branch.parent_branch_id.as_deref() {
                let previous = self
                    .directory_branches
                    .get(parent_id)
                    .ok_or_else(|| invalid("directory composition parent is missing"))?;
                if previous.repository_id != branch.repository_id
                    || previous.directory_id != branch.directory_id
                {
                    return Err(invalid(
                        "directory composition parent escapes its stable directory",
                    ));
                }
            }
        }
        // Resolve each edge once rather than walking every ancestor again for
        // every retained composition in a long directory history.
        let mut resolved = BTreeSet::new();
        for id in self.directory_branches.keys() {
            let mut path = BTreeSet::new();
            let mut cursor = Some(id.as_str());
            while let Some(id) = cursor {
                if resolved.contains(id) {
                    break;
                }
                if !path.insert(id) {
                    return Err(invalid("directory composition parent cycle"));
                }
                cursor = self.directory_branches[id].parent_branch_id.as_deref();
            }
            resolved.extend(path);
        }
        for (id, tree) in &self.directory_trees {
            validate_id(id, "dtree")?;
            let branch = self
                .directory_branches
                .get(&tree.directory_branch_id)
                .ok_or_else(|| invalid("directory tree owner branch is missing"))?;
            if tree.tree_id != *id {
                return Err(invalid("directory tree identity differs"));
            }
            let mut snapshot = branch.clone();
            snapshot.commit_id = tree.commit_id.clone();
            snapshot.path = tree.path.clone();
            snapshot.entries = tree.entries.clone();
            snapshot.file_branches = tree.file_branches.clone();
            snapshot.file_contexts = tree.file_contexts.clone();
            snapshot.inherited_files = tree.inherited_files.clone();
            snapshot.directory_refs = tree.directory_refs.clone();
            self.validate_directory_snapshot(&snapshot)?;
            if let Some(parent) = &tree.parent_tree_id {
                let previous = self
                    .directory_trees
                    .get(parent)
                    .and_then(|tree| self.directory_branches.get(&tree.directory_branch_id))
                    .ok_or_else(|| invalid("directory tree parent or its branch is missing"))?;
                if previous.repository_id != branch.repository_id
                    || previous.directory_id != branch.directory_id
                {
                    return Err(invalid(
                        "directory tree parent escapes its stable directory",
                    ));
                }
            }
        }
        let mut resolved = BTreeSet::new();
        for id in self.directory_trees.keys() {
            let mut path = BTreeSet::new();
            let mut cursor = Some(id.as_str());
            while let Some(id) = cursor {
                if resolved.contains(id) {
                    break;
                }
                if !path.insert(id) {
                    return Err(invalid("directory tree parent cycle"));
                }
                cursor = self.directory_trees[id].parent_tree_id.as_deref();
            }
            resolved.extend(path);
        }
        for (id, branch) in &self.directory_branches {
            let head = self
                .directory_trees
                .get(&branch.head_tree_id)
                .ok_or_else(|| invalid("directory branch HEAD tree is missing"))?;
            let owner = &self.directory_branches[&head.directory_branch_id];
            if owner.directory_id != branch.directory_id
                || owner.repository_id != branch.repository_id
                || branch.commit_id != head.commit_id
                || branch.path != head.path
                || branch.file_branches != head.file_branches
                || branch.directory_refs != head.directory_refs
                || canonical_json(&branch.entries)? != canonical_json(&head.entries)?
            {
                return Err(invalid(
                    "directory branch projection differs from its HEAD tree",
                ));
            }
            if branch.materialized {
                if head.directory_branch_id != *id
                    || branch.file_contexts != head.file_contexts
                    || branch.sequence != head.sequence
                    || branch.inherited_files != head.inherited_files
                {
                    return Err(invalid(
                        "completed directory branch HEAD belongs to another branch",
                    ));
                }
            } else {
                self.validate_directory_snapshot(branch)?;
                if branch.sequence < head.sequence {
                    return Err(invalid(
                        "directory branch registration precedes its inherited tree",
                    ));
                }
                let mut parent = branch.parent_branch_id.as_deref();
                let mut inherited = false;
                while let Some(parent_id) = parent {
                    if parent_id == head.directory_branch_id {
                        inherited = true;
                        break;
                    }
                    parent = self.directory_branches[parent_id]
                        .parent_branch_id
                        .as_deref();
                }
                if !inherited {
                    return Err(invalid(
                        "unwritten directory branch HEAD is not inherited from its parent",
                    ));
                }
            }
        }
        for (directory, selected) in &self.directory_selections {
            if !self
                .directory_branches
                .get(selected)
                .is_some_and(|branch| branch.directory_id == *directory)
            {
                return Err(invalid(
                    "selected directory composition is missing or belongs to another directory",
                ));
            }
        }
        for (directory, tree) in &self.directory_tree_selections {
            let tree = self
                .directory_trees
                .get(tree)
                .ok_or_else(|| invalid("selected directory tree is missing"))?;
            if !self
                .directory_branches
                .get(&tree.directory_branch_id)
                .is_some_and(|branch| &branch.directory_id == directory)
                || !self.directory_selections.contains_key(directory)
                || !self.directory_branch_inherits(
                    &self.directory_selections[directory],
                    &tree.directory_branch_id,
                )
            {
                return Err(invalid(
                    "selected directory tree escapes its directory scope",
                ));
            }
        }
        let directory_ids = self
            .commits
            .values()
            .flat_map(|commit| {
                commit
                    .entries
                    .iter()
                    .filter(|entry| entry.kind == "directory")
                    .map(|entry| entry.entry_id.as_str())
            })
            .collect::<BTreeSet<_>>();
        for directory in self.directory_selection_generations.keys() {
            if directory == &self.repository.repository_id {
                continue;
            }
            validate_id(directory, "entry")?;
            if !directory_ids.contains(directory.as_str()) {
                return Err(invalid(
                    "directory selection generation is outside the history",
                ));
            }
        }
        let file_ids = self
            .versions
            .values()
            .map(|version| version.file_id.as_str())
            .collect::<BTreeSet<_>>();
        for (file, origin) in &self.file_write_origins {
            if !file_ids.contains(file.as_str()) || !self.directory_branches.contains_key(origin) {
                return Err(invalid("current inherited file intent escapes the history"));
            }
        }
        let prefix = format!("{}/", self.repository.repository_id);
        let live = self
            .branches
            .get(&self.repository.main_branch_id)
            .and_then(|branch| self.commits.get(&branch.head_commit_id))
            .ok_or_else(|| invalid("current namespace branch or commit is missing"))?;
        for (key, context) in &self.file_contexts {
            let path = key
                .strip_prefix(&prefix)
                .ok_or_else(|| invalid("file path context escapes its repository"))?;
            name_policy::validate_path(path)?;
            let entry = live
                .entries
                .iter()
                .find(|entry| entry.kind == "file" && entry.lookup_key == path)
                .ok_or_else(|| invalid("current file path context has no namespace entry"))?;
            self.validate_file_context(context, &entry.file_id, &entry.version_id)?;
        }
        if self
            .file_selection_generations
            .keys()
            .any(|file| !file_ids.contains(file.as_str()))
        {
            return Err(invalid("file selection generation escapes the history"));
        }
        for (file, location) in &self.file_locations {
            if !file_ids.contains(file.as_str()) {
                return Err(invalid("canonical file location escapes the history"));
            }
            name_policy::validate_path(location)?;
            if self.file_names.get(file).map(String::as_str) != location.rsplit('/').next() {
                return Err(invalid(
                    "canonical file name differs from its physical location",
                ));
            }
        }
        if self.file_notes.iter().any(|(file, note)| {
            storage_contract::validate_note(note).is_err() || !file_ids.contains(file.as_str())
        }) || self.version_notes.iter().any(|(version, note)| {
            storage_contract::validate_note(note).is_err() || !self.versions.contains_key(version)
        }) {
            return Err(invalid(
                "file or version notes escape the history or violate the current 128-byte single-line format",
            ));
        }
        for (file, name) in &self.file_names {
            if !file_ids.contains(file.as_str()) {
                return Err(invalid(
                    "container name references a file outside the history",
                ));
            }
            name_policy::validate_name(name)?;
        }
        if !self.branches.contains_key(&self.repository.main_branch_id)
            || self
                .history
                .main_branch_id
                .as_ref()
                .is_some_and(|id| !self.branches.contains_key(id))
        {
            return Err(invalid("main branch missing"));
        }
        if self.branches[&self.repository.main_branch_id]
            .file_id
            .is_some()
        {
            return Err(invalid("directory main branch is scoped to a file"));
        }
        if self.versions.len() != self.placements.len()
            || self.versions.len() != self.content_states.len()
        {
            return Err(invalid("version state or placement coverage differs"));
        }
        for (id, version) in &self.versions {
            // Tombstones retain identity even without a payload to trigger
            // content_path validation. Reject malformed retained records before
            // import can reach a decision or post-decision name projection.
            crate::container_storage::validate_version(version).map_err(|error| {
                Error::new(
                    "HISTORY_PACKAGE_INVALID",
                    format!("invalid version storage identity: {error}"),
                )
            })?;
            validate_id(id, "ver")?;
            validate_id(&version.file_id, "file")?;
            name_policy::validate_name(&version.creation_name)?;
            name_policy::validate_name(&version.storage_name)?;
            let placement = self
                .placements
                .get(id)
                .ok_or_else(|| invalid("version placement missing"))?;
            if version.version_id != *id
                || version.history_tree_id != *history
                || placement.version_id != *id
                || !["AVAILABLE", "PURGED", "MIGRATED"].contains(&self.content_states[id].as_str())
            {
                return Err(invalid("version, placement or tombstone identity differs"));
            }
            if !SUPPORTED_CHECKSUM_CHUNK_BYTES.contains(&version.checksum_chunk_bytes)
                || ![&version.content_hash, &version.checksum_manifest_hash]
                    .iter()
                    .all(|h| {
                        h.len() == 64
                            && h.bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    })
            {
                return Err(invalid("version checksum metadata is invalid"));
            }
        }
        self.validate_file_nodes()?;
        for (id, link) in &self.merge_links {
            validate_id(id, "link")?;
            if link.merge_link_id != *id
                || link.history_tree_id != *history
                || !self.branches.contains_key(&link.source_branch_id)
                || !self.branches.contains_key(&link.target_branch_id)
                || !self.commits.contains_key(&link.source_commit_id)
                || !self.commits.contains_key(&link.target_commit_id)
            {
                return Err(invalid("merge link crosses a live history boundary"));
            }
        }
        for (id, boundary) in &self.boundary_marks {
            validate_id(id, "boundary")?;
            if boundary.boundary_id != *id
                || boundary.history_tree_id != *history
                || !self.commits.contains_key(&boundary.local_commit_id)
                || boundary
                    .local_branch_id
                    .as_ref()
                    .is_some_and(|id| !self.branches.contains_key(id))
            {
                return Err(invalid("boundary local reference missing"));
            }
            validate_id(&boundary.external_history_tree_id, "history")?;
            validate_id(&boundary.external_commit_id, "commit")?;
        }
        for (id, rule) in &self.placement_rules {
            validate_id(id, "rule")?;
            if id != &rule.rule_id || rule.repository_id != self.repository.repository_id {
                return Err(invalid("placement rule binding differs"));
            }
            if !rule.path_prefix.is_empty() {
                name_policy::validate_path(&rule.path_prefix)?;
            }
        }
        for (file, paths) in &self.file_backup_paths {
            if !file_ids.contains(file.as_str()) {
                return Err(invalid("backup location escapes the history"));
            }
            for (slot, path) in paths {
                name_policy::validate_slot(slot)?;
                name_policy::validate_path(path)?;
            }
        }
        for (file, policy) in &self.file_backup_policies {
            if !file_ids.contains(file.as_str()) || !self.protection_policies.contains_key(policy) {
                return Err(invalid("backup binding escapes its history or policy"));
            }
        }
        let bound_policies = self
            .file_backup_policies
            .values()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut inheritable_scopes = BTreeSet::new();
        for (id, policy) in &self.protection_policies {
            validate_id(id, "policy")?;
            if id != &policy.policy_id || policy.repository_id != self.repository.repository_id {
                return Err(invalid("protection policy binding differs"));
            }
            if !policy.path_prefix.is_empty() {
                name_policy::validate_path(&policy.path_prefix)?;
            }
            if !policy.inheritable && !bound_policies.contains(id.as_str()) {
                return Err(invalid("retained backup settings have no file binding"));
            }
            let scope = policy
                .scope_directory_id
                .as_deref()
                .map(|id| ("directory", id))
                .or_else(|| policy.scope_file_id.as_deref().map(|id| ("file", id)));
            if policy.inheritable && scope.is_some_and(|scope| !inheritable_scopes.insert(scope)) {
                return Err(invalid("backup scope has multiple inheritable policies"));
            }
            if policy.target_copies == 0
                || (policy.scope_file_id.is_some() && policy.scope_directory_id.is_some())
                || policy
                    .scope_file_id
                    .as_ref()
                    .is_some_and(|file| !file_ids.contains(file.as_str()))
                || policy.scope_directory_id.as_ref().is_some_and(|directory| {
                    directory != &self.repository.repository_id
                        && !self
                            .commits
                            .values()
                            .flat_map(|commit| &commit.entries)
                            .any(|entry| entry.kind == "directory" && entry.entry_id == *directory)
                })
            {
                return Err(invalid("backup policy scope or count differs"));
            }
            for (slot, prefix) in &policy.backup_prefixes {
                name_policy::validate_slot(slot)?;
                if !prefix.is_empty() {
                    name_policy::validate_path(prefix)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn store_history_manifests_bounded(
    members: &[DiskConfig],
    state: &State,
    maximum: usize,
) -> Result<BTreeMap<String, String>> {
    let (hashes, objects) = prepare_history_manifests(state, maximum)?;
    for object in objects {
        object.store(members)?;
    }
    Ok(hashes)
}
pub(crate) fn prepare_history_manifests(
    state: &State,
    maximum: usize,
) -> Result<(BTreeMap<String, String>, Vec<PreparedMetadataObject>)> {
    let mut hashes = BTreeMap::new();
    let mut objects = Vec::new();
    for id in state.histories.keys() {
        let manifest = HistoryManifest::from_state(state, id)?;
        let (hash, graph) = graph::prepare(&manifest, maximum)?;
        hashes.insert(id.clone(), hash);
        objects.extend(graph);
    }
    Ok((hashes, objects))
}

type HistoryGraphObjects = Vec<(String, String, json_codec::Value)>;

/// Select every dependency independently from trusted replicas, without repair.
fn bound_history_graph(
    members: &[DiskConfig],
    hash: &str,
) -> Result<(HistoryManifest, HistoryGraphObjects)> {
    let root = crate::publication_evidence::verified_object(members, "history-manifest", hash)?;
    let (manifest, objects) = graph::resolve_with_objects(root, |kind, hash| {
        crate::publication_evidence::verified_object(members, kind, hash)
    })?;
    manifest.validate()?;
    Ok((manifest, objects))
}

/// Freeze exact candidate-bound history graphs before any recovery writes. Missing
/// roots cannot be recreated from a flat projection or a different encoder.
pub(crate) fn bound_history_objects(
    members: &[DiskConfig],
    candidate: &Candidate,
    state: Option<&State>,
) -> Result<HistoryGraphObjects> {
    let mut objects = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(state) = state
        && candidate
            .history_manifest_hashes
            .keys()
            .ne(state.histories.keys())
    {
        return Err(Error::new(
            "OBJECT_CORRUPT",
            "candidate history set differs",
        ));
    }
    for (id, hash) in &candidate.history_manifest_hashes {
        let expected = state
            .map(|state| HistoryManifest::from_state(state, id))
            .transpose()?;
        let (manifest, graph) = bound_history_graph(members, hash)?;
        let manifest_bytes = canonical_json(&manifest)?;
        if manifest.history.history_tree_id != *id
            || expected
                .as_ref()
                .map(canonical_json)
                .transpose()?
                .is_some_and(|expected| expected != manifest_bytes)
        {
            return Err(Error::new(
                "OBJECT_CORRUPT",
                "bound history projection differs",
            ));
        }
        for (kind, hash, value) in graph {
            encode_metadata(&value, MAX_METADATA)?;
            if seen.insert((kind.clone(), hash.clone())) {
                objects.push((kind, hash, value));
            }
        }
    }
    Ok(objects)
}

pub(crate) fn repair_history_objects(
    members: &[DiskConfig],
    objects: &HistoryGraphObjects,
    repaired: &mut BTreeSet<(String, String)>,
) -> Result<()> {
    // Resolver output is dependency-first, including child indexes before parents.
    for (kind, hash, value) in objects {
        if !repaired.contains(&(kind.clone(), hash.clone())) {
            crate::publication_evidence::repair_object_copies(members, kind, hash, value)?;
            // Optimization only: overflow objects remain fully verified and repaired.
            if repaired.len() < 16_384 {
                repaired.insert((kind.clone(), hash.clone()));
            }
        }
    }
    Ok(())
}

fn package_version(package: &Path, version: &str) -> PathBuf {
    package.join("versions").join(version)
}
fn package_object(package: &Path, kind: &str, hash: &str) -> Result<PathBuf> {
    if !matches!(kind, "state-page" | "history-index")
        || hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::new(
            "HISTORY_PACKAGE_INVALID",
            "invalid graph object address",
        ));
    }
    Ok(package
        .join("objects")
        .join(kind)
        .join(&hash[..2])
        .join(format!("{hash}.json")))
}

fn load_package(package: &Path) -> Result<(HistoryManifest, PackageProof, HistoryPackageReport)> {
    let root: json_codec::Value = read_json(&package.join("manifest.json"))?;
    let hash = object_hash("history-manifest", &root)?;
    let manifest = graph::resolve(root, |kind, hash| {
        read_json(&package_object(package, kind, hash)?)
    })?;
    manifest.validate()?;
    let proof: PackageProof = read_json(&package.join("publication.json"))?;
    verify_publication_seal(
        &proof.candidate,
        &proof.publication,
        &proof.publication_seal,
    )?;
    if proof.manifest_hash != hash
        || proof
            .candidate
            .history_manifest_hashes
            .get(&manifest.history.history_tree_id)
            != Some(&hash)
    {
        return Err(Error::new(
            "HISTORY_PACKAGE_INVALID",
            "manifest is not bound by the published candidate",
        ));
    }
    let mut full_files = 0;
    let mut tombstones = 0;
    let mut full_bytes = 0u64;
    for (id, version) in &manifest.versions {
        if manifest.content_states[id] == "AVAILABLE" {
            let directory = package_version(package, id);
            let metadata: FileVersion = read_json(&directory.join("version.json"))?;
            if object_hash("version", &metadata)? != object_hash("version", version)? {
                return Err(Error::new(
                    "HISTORY_PACKAGE_INVALID",
                    "packaged version metadata differs",
                ));
            }
            verify_payload(&directory, version)?;
            full_files += 1;
            full_bytes = full_bytes
                .checked_add(version.logical_size)
                .ok_or_else(|| Error::new("RESOURCE_LIMIT", "package byte count overflow"))?;
        } else {
            tombstones += 1;
        }
    }
    let report = HistoryPackageReport {
        history_tree_id: manifest.history.history_tree_id.clone(),
        repository_id: manifest.repository.repository_id.clone(),
        manifest_hash: hash,
        publication_sequence: proof.candidate.sequence,
        commits: manifest.commits.len(),
        versions: manifest.versions.len(),
        full_files,
        tombstones,
        full_bytes,
    };
    Ok((manifest, proof, report))
}
/// Validate one history without a system index, source pool, network or account store.
pub fn offline_check_history_package(package: &Path) -> Result<HistoryPackageReport> {
    load_package(package).map(|(_, _, report)| report)
}
impl Engine {
    pub fn export_history_package(
        &self,
        history: &str,
        destination: &Path,
    ) -> Result<HistoryPackageReport> {
        self.repository_for_history(history)?;
        let manifest = HistoryManifest::from_state(&self.state, history)?;
        let mut proof = None;
        let mut graph_objects = Vec::new();
        let members = self.trusted_evidence_members();
        for disk in &members {
            for (record, _) in scan(disk)?
                .records
                .into_iter()
                .rev()
                .filter(|(r, _)| r.record_type == "PUBLICATION")
            {
                let candidate: Candidate = crate::publication_evidence::verified_object(
                    &members,
                    "candidate",
                    &record.candidate_hash,
                )?;
                self.verify_candidate_membership(&candidate)?;
                if candidate.pool_id != self.config.pool_id || candidate.sequence != record.sequence
                {
                    return Err(Error::new(
                        "PUBLICATION_INVALID",
                        "candidate differs from journal or pool",
                    ));
                }
                let Some(hash) = candidate.history_manifest_hashes.get(history).cloned() else {
                    continue;
                };
                let (bound, objects) = bound_history_graph(&members, &hash)?;
                if canonical_json(&bound)? != canonical_json(&manifest)? {
                    continue;
                }
                graph_objects = objects;
                let publication: Publication = crate::publication_evidence::verified_object(
                    &members,
                    "publication",
                    &record.object_hash,
                )?;
                let publication_seal: PublicationSeal =
                    crate::publication_evidence::verified_publication_seal(
                        &members,
                        &candidate,
                        &publication,
                    )?;
                proof = Some(PackageProof {
                    manifest_hash: hash.clone(),
                    candidate,
                    publication,
                    publication_seal,
                });
                break;
            }
            if proof.is_some() {
                break;
            }
        }
        let proof = proof.ok_or_else(|| {
            Error::new(
                "RECOVERY_REQUIRED",
                "no published manifest matches the current history",
            )
        })?;
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        private_dir(parent)?;
        let staging = parent.join(format!(".zxs-history-{}", new_id("req")));
        private_dir(&staging)?;
        let result = (|| {
            for (id, version) in &manifest.versions {
                if manifest.content_states[id] != "AVAILABLE" {
                    continue;
                }
                let (source, _) = self.valid_replica(id)?;
                let dir = package_version(&staging, id);
                private_dir(&dir.join("content"))?;
                let (size, hash, checksums) = full_copy(
                    &source,
                    &dir.join("content").join(&version.creation_name),
                    version.checksum_chunk_bytes,
                    self.config.max_file_bytes,
                )?;
                if size != version.logical_size
                    || hash != version.content_hash
                    || blake3::hash(&checksums.encode()?).to_hex().as_str()
                        != version.checksum_manifest_hash
                {
                    return Err(Error::new(
                        "SOURCE_CHANGED",
                        "exported content differs from the published manifest",
                    ));
                }
                write_json(&dir.join("version.json"), version, false)?;
                write_atomic(&dir.join("checksums.bin"), &checksums.encode()?, false)?;
                sync_dir(&dir)?;
            }
            for (kind, hash, value) in &graph_objects {
                let path = if kind == "history-manifest" {
                    staging.join("manifest.json")
                } else {
                    package_object(&staging, kind, hash)?
                };
                write_json(&path, value, false)?;
            }
            write_json(&staging.join("publication.json"), &proof, false)?;
            sync_dir(&staging)?;
            let report = offline_check_history_package(&staging)?;
            rustix::fs::renameat_with(
                rustix::fs::CWD,
                &staging,
                rustix::fs::CWD,
                destination,
                rustix::fs::RenameFlags::NOREPLACE,
            )
            .map_err(std::io::Error::from)?;
            sync_dir(parent)?;
            Ok(report)
        })();
        if result.is_err() && staging.exists() {
            let _ = fs::remove_dir_all(staging);
        }
        result
    }
    /// Restore immutable logical identities into a new pool. Routes are explicitly
    /// rebound, every available file is fully copied, and the result is republished.
    pub fn restore_history_package(
        &mut self,
        package: &Path,
        primary: &str,
        copies: u32,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        self.require_write()?;
        let (manifest, proof, report) = load_package(package)?;
        let route = self.route(primary, copies)?;
        for version in manifest.versions.values() {
            if version.checksum_chunk_bytes != self.config.checksum_chunk_bytes
                || version.logical_size > self.config.max_file_bytes
            {
                return Err(Error::new(
                    "FORMAT_UNSUPPORTED",
                    "restore requires compatible checksum chunks and file size limits",
                ));
            }
        }
        // Recovery explicitly rebinds routes. Keep the user's logical prefixes
        // by route position, never derive them from a primary physical path.
        let mut backup_slots = BTreeMap::<String, String>::new();
        for placement in manifest.placements.values() {
            for (old, new) in placement
                .route_slot_ids
                .iter()
                .skip(1)
                .zip(route.iter().skip(1))
            {
                if backup_slots
                    .insert(old.clone(), new.clone())
                    .is_some_and(|prior| prior != *new)
                {
                    return Err(Error::new(
                        "RESTORE_BACKUP_CONFLICT",
                        "source backup slots cannot be rebound unambiguously",
                    ));
                }
            }
        }
        let remap_paths = |paths: &BTreeMap<String, String>| -> Result<BTreeMap<String, String>> {
            let mut rebound = BTreeMap::new();
            for (slot, path) in paths {
                if let Some(target) = backup_slots.get(slot)
                    && rebound
                        .insert(target.clone(), path.clone())
                        .is_some_and(|prior| prior != *path)
                {
                    return Err(Error::new(
                        "RESTORE_BACKUP_CONFLICT",
                        "backup prefixes collide after route rebinding",
                    ));
                }
            }
            if !paths.is_empty() && route.iter().skip(1).any(|slot| !rebound.contains_key(slot)) {
                return Err(Error::new(
                    "BACKUP_PREFIX_REQUIRED",
                    "the restored route has a backup slot without a user prefix",
                ));
            }
            Ok(rebound)
        };
        let backup_paths = manifest
            .file_backup_paths
            .iter()
            .map(|(file, paths)| Ok((file.clone(), remap_paths(paths)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let backup_prefixes = manifest
            .protection_policies
            .iter()
            .map(|(id, policy)| Ok((id.clone(), remap_paths(&policy.backup_prefixes)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let after_source =
            proof.candidate.sequence.checked_add(1).ok_or_else(|| {
                Error::new("RESOURCE_LIMIT", "source publication sequence exhausted")
            })?;
        self.mutate("history.restore",principal,operation,json!({"manifest_hash":report.manifest_hash,"primary_slot_id":primary,"target_copies":copies}),|engine,state,pending| {
            if state.histories.contains_key(&manifest.history.history_tree_id) || state.repositories.contains_key(&manifest.repository.repository_id)
                || manifest.commits.keys().any(|id| state.commits.contains_key(id))
                || manifest.branches.keys().any(|id| state.branches.contains_key(id))
                || manifest.directory_branches.keys().any(|id| state.directory_branches.contains_key(id))
                || manifest.directory_trees.keys().any(|id| state.directory_trees.contains_key(id))
                || manifest.directory_selections.keys().any(|id| state.directory_selections.contains_key(id))
                || manifest.directory_selection_generations.keys().any(|id| state.directory_selection_generations.contains_key(id))
                || manifest.file_write_origins.keys().any(|id| state.file_write_origins.contains_key(id))
                || manifest.file_contexts.keys().any(|id| state.file_contexts.contains_key(id))
                || manifest.file_path_nodes.keys().any(|id| state.file_path_nodes.contains_key(id))
                || manifest.file_metadata_nodes.keys().any(|id| state.file_metadata_nodes.contains_key(id))
                || manifest.file_branch_merges.keys().any(|id| state.file_branch_merges.contains_key(id))
                || manifest.file_branch_aliases.keys().any(|id| state.file_branch_aliases.contains_key(id))
                || manifest.file_backup_paths.keys().any(|id| state.file_backup_paths.contains_key(id))
                || manifest.file_backup_policies.keys().any(|id| state.file_backup_policies.contains_key(id))
                || manifest.file_node_parents.keys().any(|id| state.file_node_parents.contains_key(id))
                || manifest.file_path_notes.keys().any(|id| state.file_path_notes.contains_key(id))
                || manifest.directory_tree_selections.keys().any(|id| state.directory_tree_selections.contains_key(id))
                || manifest.file_locations.keys().any(|id| state.file_locations.contains_key(id))
                || manifest.file_selection_generations.keys().any(|id| state.file_selection_generations.contains_key(id))
                || manifest.versions.keys().any(|id| state.versions.contains_key(id))
                || manifest.versions.values().any(|incoming|state.versions.values().any(|existing|existing.file_id==incoming.file_id&&existing.history_tree_id!=incoming.history_tree_id))
                || manifest.merge_links.keys().any(|id| state.merge_links.contains_key(id))
                || manifest.boundary_marks.keys().any(|id| state.history_boundaries.contains_key(id))
                || manifest.placement_rules.keys().any(|id| state.placement_rules.contains_key(id))
                || manifest.protection_policies.keys().any(|id| state.protection_policies.contains_key(id)) {
                return Err(Error::new("OBJECT_COLLISION","package identities already exist in the destination pool"));
            }
            if state.repositories.values().any(|r| name_policy::lookup_key(&r.name) == name_policy::lookup_key(&manifest.repository.name)) {
                return Err(Error::new("NAME_COLLISION","restored repository name already exists"));
            }
            state.sequence = state.sequence.max(after_source);
            let mut repository = manifest.repository.clone();
            repository.primary_slot_id = primary.into(); repository.target_copies = copies;
            repository.smb_enabled = false;
            // This newly admitted repository has no existing destination
            // binding; imported source revisions do not authorize SMB sessions.
            repository.smb_binding_generation = 1;
            repository.file_manager_root=false;
            state.repositories.insert(repository.repository_id.clone(),repository.clone());
            state.histories.insert(manifest.history.history_tree_id.clone(),manifest.history.clone());
            state.commits.extend(manifest.commits.clone()); state.branches.extend(manifest.branches.clone());
            state.file_selections.extend(manifest.file_selections.clone());
            state.directory_branches.extend(manifest.directory_branches.clone());
            state.directory_trees.extend(manifest.directory_trees.clone());
            state.directory_selections.extend(manifest.directory_selections.clone());
            state.directory_selection_generations.extend(manifest.directory_selection_generations.clone());
            state.file_write_origins.extend(manifest.file_write_origins.clone());
            state.file_contexts.extend(manifest.file_contexts.clone());
            state.file_path_nodes.extend(manifest.file_path_nodes.clone());
            state.file_metadata_nodes.extend(manifest.file_metadata_nodes.clone());
            state.file_branch_merges.extend(manifest.file_branch_merges.clone());
            state.file_branch_aliases.extend(manifest.file_branch_aliases.clone());
            state.file_node_parents.extend(manifest.file_node_parents.clone());
            state.file_path_notes.extend(manifest.file_path_notes.clone());
            state.directory_tree_selections.extend(manifest.directory_tree_selections.clone());
            state.file_locations.extend(manifest.file_locations.clone());
            state.file_backup_paths.extend(backup_paths.clone());
            state.file_backup_policies.extend(manifest.file_backup_policies.clone());
            state.file_selection_generations.extend(manifest.file_selection_generations.clone());
            state.file_notes.extend(manifest.file_notes.clone());state.version_notes.extend(manifest.version_notes.clone());
            state.file_names.extend(manifest.file_names.clone());
            state.merge_links.extend(manifest.merge_links.clone()); state.history_boundaries.extend(manifest.boundary_marks.clone());
            for rule in manifest.placement_rules.values() {
                let mut rule = rule.clone(); rule.primary_slot_id = primary.into(); rule.generation = 1;
                state.placement_rules.insert(rule.rule_id.clone(),rule);
            }
            for policy in manifest.protection_policies.values() {
                let mut policy = policy.clone(); policy.target_copies = copies; policy.generation = 1; policy.backup_prefixes=backup_prefixes[&policy.policy_id].clone();
                state.protection_policies.insert(policy.policy_id.clone(),policy);
            }
            for (id,version) in &manifest.versions {
                let placement = Placement {version_id:id.clone(),primary_slot_id:primary.into(),target_copies:copies,
                    route_slot_ids:route.clone(),topology_generation:state.topology_generation,
                    binding_generations:route.iter().map(|slot| Ok((slot.clone(),engine.disk(slot)?.binding_generation))).collect::<Result<_>>()?};
                state.versions.insert(id.clone(),version.clone()); state.placements.insert(id.clone(),placement.clone());
                state.content_states.insert(id.clone(),manifest.content_states[id].clone());
                if manifest.content_states[id] == "AVAILABLE" {
                    pending.push(crate::engine::Prepared {version:version.clone(),placement,
                        source:package_version(package,id).join("content").join(&version.creation_name)});
                }
            }
            Ok(json!({"history_tree_id":manifest.history.history_tree_id,"repository_id":repository.repository_id,
                "branch_id":manifest.history.main_branch_id,"manifest_hash":report.manifest_hash,"full_files":report.full_files,
                "tombstones":report.tombstones,"restoration_manager":principal,"smb_rebind_required":true}))
        })
    }
}

#[cfg(test)]
#[path = "../../../test/engine/unit/history_package/tests.rs"]
mod tests;
