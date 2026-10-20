//! Independently recoverable, publication-bound history packages.
use crate::{business::validate_entries, disk::*, journal::*, *};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryManifest {
    pub format_version: u32,
    pub repository: Repository,
    pub history: HistoryTree,
    pub branches: BTreeMap<String, Branch>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_selections: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_branches: BTreeMap<String, DirectoryBranch>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_trees: BTreeMap<String, DirectoryTree>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_write_origins: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_contexts: BTreeMap<String, FileContext>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_path_nodes: BTreeMap<String, FilePathNode>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_metadata_nodes: BTreeMap<String, FileMetadataNode>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_branch_merges: BTreeMap<String, FileBranchMerge>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_branch_aliases: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_node_parents: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_path_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_tree_selections: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_locations: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_backup_paths: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_backup_policies: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_selections: BTreeMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "zxs_contract::u64_string_map"
    )]
    pub directory_selection_generations: BTreeMap<String, u64>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "zxs_contract::u64_string_map"
    )]
    pub file_selection_generations: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub version_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
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
#[serde(deny_unknown_fields)]
pub struct HistoryPackageReport {
    pub history_tree_id: String,
    pub repository_id: String,
    pub manifest_hash: String,
    #[serde(with = "zxs_contract::u64_string")]
    pub publication_sequence: u64,
    pub commits: usize,
    pub versions: usize,
    pub full_files: usize,
    pub tombstones: usize,
    #[serde(with = "zxs_contract::u64_string")]
    pub full_bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageProof {
    format_version: u32,
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
            format_version: 2,
            manifest_hash: "0".repeat(64),
            candidate: candidate.clone(),
            publication: publication.clone(),
            publication_seal: seal.clone(),
        },
        maximum,
    )?
    .len())
}
// Previously published packages accepted notes up to 4096 bytes. Keep the
// reader compatible; new metadata writes use zxs_contract::validate_note.
const LEGACY_NOTE_READ_MAX_BYTES: usize = 4096;

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
            format_version: 2,
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
        let scope_key = zxs_naming::path_key(&branch.path);
        if branch.path.is_empty() {
            if branch.directory_id != self.repository.repository_id {
                return Err(invalid(
                    "root directory identity differs from its repository",
                ));
            }
        } else {
            zxs_naming::validate_path(&branch.path)?;
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
            zxs_naming::validate_path(&entry.path)?;
            if (!scope_key.is_empty() && !entry.lookup_key.starts_with(&prefix))
                || entry.entry_id == branch.directory_id
                || entry.name != entry.path.rsplit('/').next().unwrap_or("")
                || entry.lookup_key != zxs_naming::path_key(&entry.path)
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
        if !branch.file_contexts.is_empty() {
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
            zxs_naming::validate_path(&node.path)?;
            zxs_naming::validate_path(&node.previous_path)?;
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
                || node.tags.len() > zxs_contract::MAX_FILE_TAGS
                || node
                    .tags
                    .iter()
                    .any(|tag| zxs_contract::validate_tag(tag).is_err())
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
                || note.len() > zxs_contract::MAX_NOTE_BYTES
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
        if self.format_version != 2
            || self.history.repository_id != self.repository.repository_id
            || self.repository.history_tree_id != *history
            || !self.commits.contains_key(&self.history.root_commit_id)
        {
            return Err(invalid("manifest root or repository binding differs"));
        }
        zxs_naming::validate_name(&self.repository.name)?;
        if self.repository.name_policy != zxs_naming::NAME_POLICY
            || self.repository.unicode_version != zxs_naming::UNICODE_VERSION
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
            zxs_naming::validate_name(&branch.name)?;
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
            zxs_naming::validate_name(&branch.name)?;
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
            zxs_naming::validate_path(path)?;
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
            zxs_naming::validate_path(location)?;
            if self.file_names.get(file).map(String::as_str) != location.rsplit('/').next() {
                return Err(invalid(
                    "canonical file name differs from its physical location",
                ));
            }
        }
        if self.file_notes.iter().any(|(file, note)| {
            note.len() > LEGACY_NOTE_READ_MAX_BYTES || !file_ids.contains(file.as_str())
        }) || self.version_notes.iter().any(|(version, note)| {
            note.len() > LEGACY_NOTE_READ_MAX_BYTES || !self.versions.contains_key(version)
        }) {
            return Err(invalid(
                "file or version notes escape the history or exceed the size limit",
            ));
        }
        for (file, name) in &self.file_names {
            if !file_ids.contains(file.as_str()) {
                return Err(invalid(
                    "container name references a file outside the history",
                ));
            }
            zxs_naming::validate_name(name)?;
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
            validate_id(id, "ver")?;
            validate_id(&version.file_id, "file")?;
            zxs_naming::validate_name(&version.creation_name)?;
            if let Some(name) = &version.storage_name {
                zxs_naming::validate_name(name)?;
            }
            let placement = self
                .placements
                .get(id)
                .ok_or_else(|| invalid("version placement missing"))?;
            if version.version_id != *id
                || version.history_tree_id != *history
                || version.format_version != 2
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
                zxs_naming::validate_path(&rule.path_prefix)?;
            }
        }
        for (file, paths) in &self.file_backup_paths {
            if !file_ids.contains(file.as_str()) {
                return Err(invalid("backup location escapes the history"));
            }
            for (slot, path) in paths {
                zxs_naming::validate_slot(slot)?;
                zxs_naming::validate_path(path)?;
            }
        }
        for (file, policy) in &self.file_backup_policies {
            if !file_ids.contains(file.as_str()) || !self.protection_policies.contains_key(policy) {
                return Err(invalid("backup binding escapes its history or policy"));
            }
        }
        for (id, policy) in &self.protection_policies {
            validate_id(id, "policy")?;
            if id != &policy.policy_id || policy.repository_id != self.repository.repository_id {
                return Err(invalid("protection policy binding differs"));
            }
            if !policy.path_prefix.is_empty() {
                zxs_naming::validate_path(&policy.path_prefix)?;
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
                zxs_naming::validate_slot(slot)?;
                if !prefix.is_empty() {
                    zxs_naming::validate_path(prefix)?;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn store_history_manifests(
    members: &[DiskConfig],
    state: &State,
) -> Result<BTreeMap<String, String>> {
    store_history_manifests_bounded(members, state, MAX_METADATA)
}
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
        let object = PreparedMetadataObject::new("history-manifest", &manifest, maximum)?;
        hashes.insert(id.clone(), object.hash.clone());
        objects.push(object);
    }
    Ok((hashes, objects))
}

fn package_version(package: &Path, version: &str) -> PathBuf {
    package.join("versions").join(version)
}
fn load_package(package: &Path) -> Result<(HistoryManifest, PackageProof, HistoryPackageReport)> {
    let manifest: HistoryManifest = read_json(&package.join("manifest.json"))?;
    manifest.validate()?;
    let proof: PackageProof = read_json(&package.join("publication.json"))?;
    let hash = object_hash("history-manifest", &manifest)?;
    verify_publication_seal(
        &proof.candidate,
        &proof.publication,
        &proof.publication_seal,
    )?;
    if proof.format_version != 2
        || proof.manifest_hash != hash
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
        principal: &str,
    ) -> Result<HistoryPackageReport> {
        self.require_grant(
            self.repository_for_history(history)?,
            principal,
            zxs_contract::RepositoryAccess::Manage,
        )?;
        let manifest = HistoryManifest::from_state(&self.state, history)?;
        let hash = object_hash("history-manifest", &manifest)?;
        let mut proof = None;
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
                if candidate.pool_id != self.config.pool_id || candidate.sequence != record.sequence
                {
                    return Err(Error::new(
                        "PUBLICATION_INVALID",
                        "candidate differs from journal or pool",
                    ));
                }
                if candidate.history_manifest_hashes.get(history) != Some(&hash) {
                    continue;
                }
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
                    format_version: 2,
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
            write_json(&staging.join("manifest.json"), &manifest, false)?;
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
            if state.repositories.values().any(|r| zxs_naming::lookup_key(&r.name) == zxs_naming::lookup_key(&manifest.repository.name)) {
                return Err(Error::new("NAME_COLLISION","restored repository name already exists"));
            }
            state.sequence = state.sequence.max(after_source);
            let mut repository = manifest.repository.clone();
            repository.primary_slot_id = primary.into(); repository.target_copies = copies;
            repository.account_scope_repository_id = None; repository.smb_enabled = false;
            repository.file_manager_root=false;
            repository.grants.manage.insert(principal.into());
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
mod tests {
    use super::*;
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, PermissionsExt},
    };

    fn initialize(root: &Path) -> Engine {
        Engine::initialize(
            &root.join("state"),
            Config::development([root.join("p"), root.join("b"), root.join("c")]),
        )
        .unwrap()
    }
    fn save(engine: &mut Engine, branch: &str, bytes: &[u8]) -> String {
        let workspace = engine.create_workspace(branch, "alice").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "file.txt", bytes)
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "save".into(),
                request_digest: None,
            })
            .unwrap()
            .commit_id
            .unwrap()
    }

    fn legacy_placeholder(
        disk: &DiskConfig,
        history: &str,
        file: &str,
        name: &str,
        branch: &str,
    ) -> PathBuf {
        disk.root
            .join("histories")
            .join(history)
            .join("file-containers")
            .join(file)
            .join("namespace")
            .join(name)
            .join("pending-branches")
            .join(format!("{branch}.pending"))
    }

    #[test]
    fn nested_directory_compositions_keep_frozen_paths_and_restore_after_rename() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temp.path().join("source"));
        let repo = engine
            .create_repository("嵌套恢复", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "alice")
            .unwrap();
        for (path, bytes) in [
            ("folder/a.txt", b"parent A".as_slice()),
            ("folder/nested/b.txt", b"parent B".as_slice()),
        ] {
            engine
                .stage_bytes(&workspace.workspace_id, path, bytes)
                .unwrap();
        }
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "nested parent files".into(),
                request_digest: None,
            })
            .unwrap();
        engine
            .initialize_directory_scope(
                &repo.repository_id,
                "folder/nested",
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let created = engine
            .create_directory_branches(
                &repo.repository_id,
                "folder",
                "draft",
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let id = created.result["directory_branch_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let directory = engine.state.directory_branches[&id].directory_id.clone();
        let inherited_tree = engine.state.directory_branches[&id].head_tree_id.clone();
        let frozen = engine.state.directory_branches[&id]
            .entries
            .iter()
            .find(|entry| entry.path == "folder/nested/b.txt")
            .unwrap()
            .clone();
        engine
            .select_directory_branches(
                &repo.repository_id,
                "folder",
                "draft",
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "alice")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "folder/a.txt", b"owned A")
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "complete one file".into(),
                request_digest: None,
            })
            .unwrap();
        assert!(engine.state.directory_branches[&id].materialized);
        assert!(
            engine.state.directory_branches[&id]
                .entries
                .iter()
                .any(|entry| entry.file_id == frozen.file_id
                    && entry.version_id == frozen.version_id)
        );
        let body_proofs = engine
            .state
            .versions
            .iter()
            .map(|(id, version)| (id.clone(), canonical_json(version).unwrap()))
            .collect::<BTreeMap<_, _>>();
        engine
            .file_namespace(
                &repo.repository_id,
                NamespaceOperation {
                    kind: "rename".into(),
                    path: "folder".into(),
                    destination: Some("renamed".into()),
                },
                "alice",
                &new_id("op"),
            )
            .unwrap();
        assert_eq!(engine.state.file_path_nodes.len(), 2);
        for node in engine.state.file_path_nodes.values() {
            assert!(node.previous_path.starts_with("folder/"));
            assert!(node.path.starts_with("renamed/"));
            assert_eq!(
                canonical_json(&engine.state.versions[&node.version_id]).unwrap(),
                body_proofs[&node.version_id]
            );
        }
        let manifest = HistoryManifest::from_state(&engine.state, &repo.history_tree_id).unwrap();
        assert_eq!(manifest.directory_branches[&id].directory_id, directory);
        assert!(!manifest.directory_branches[&id].directory_refs.is_empty());
        assert_eq!(manifest.file_write_origins.get(&frozen.file_id), Some(&id));
        assert_eq!(manifest.directory_trees[&inherited_tree].path, "folder");
        assert_eq!(
            manifest.directory_branches[&id].path,
            manifest.directory_trees[&manifest.directory_branches[&id].head_tree_id].path
        );
        let visible = engine
            .directory_branches(&repo.repository_id, "renamed")
            .unwrap();
        assert!(
            visible
                .iter()
                .any(|branch| branch["directory_branch_id"] == id)
        );
        let package = temp.path().join("package");
        engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        let mut restored = initialize(&temp.path().join("restored"));
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            canonical_json(&restored.state.directory_branches).unwrap(),
            canonical_json(&manifest.directory_branches).unwrap()
        );
        assert_eq!(
            canonical_json(&restored.state.directory_trees).unwrap(),
            canonical_json(&manifest.directory_trees).unwrap()
        );
        assert_eq!(
            restored.state.directory_selection_generations,
            manifest.directory_selection_generations
        );
        assert_eq!(
            restored.state.file_write_origins,
            manifest.file_write_origins
        );
        assert_eq!(
            restored.state.directory_tree_selections,
            manifest.directory_tree_selections
        );
        assert_eq!(restored.state.file_contexts, manifest.file_contexts);
        assert_eq!(
            canonical_json(&restored.state.file_path_nodes).unwrap(),
            canonical_json(&manifest.file_path_nodes).unwrap()
        );
        assert_eq!(restored.state.file_node_parents, manifest.file_node_parents);
        assert_eq!(restored.state.file_path_notes, manifest.file_path_notes);
        assert_eq!(restored.state.file_locations, manifest.file_locations);
        assert_eq!(
            restored.state.file_selection_generations,
            manifest.file_selection_generations
        );
        assert!(
            restored
                .directory_branches(&repo.repository_id, "renamed")
                .unwrap()
                .iter()
                .any(|branch| branch["directory_branch_id"] == id)
        );
        assert!(restored.open_content(&frozen.version_id).is_ok());
    }

    #[test]
    fn moved_file_aliases_restore_independent_path_contexts_and_one_physical_owner() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temporary.path().join("source"));
        let repo = engine
            .create_disk_root("P01", "alice", &new_id("op"))
            .unwrap();
        let save_path = |engine: &mut Engine, path: &str, bytes: &[u8]| {
            let workspace = engine
                .create_workspace(&repo.main_branch_id, "alice")
                .unwrap();
            engine
                .stage_bytes(&workspace.workspace_id, path, bytes)
                .unwrap();
            engine
                .commit_workspace(CommitRequest {
                    workspace_id: workspace.workspace_id,
                    operation_id: new_id("op"),
                    principal: "alice".into(),
                    expected_head: workspace.base_head,
                    fencing_token: workspace.fencing_token,
                    message: "alias write".into(),
                    request_digest: None,
                })
                .unwrap();
        };
        save_path(&mut engine, "a/f.txt", b"F1");
        save_path(&mut engine, "b/keep.txt", b"keep");
        let original = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.path == "a/f.txt")
            .unwrap();
        let create = |engine: &mut Engine, path: &str, name: &str| {
            engine
                .create_directory_branches(&repo.repository_id, path, name, "alice", &new_id("op"))
                .unwrap()
                .result["directory_branch_id"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let m = create(&mut engine, "a", "m");
        let n = create(&mut engine, "a", "n");
        let l = create(&mut engine, "b", "l");
        engine
            .select_directory_branches(&repo.repository_id, "a", "m", "alice", &new_id("op"))
            .unwrap();
        engine
            .select_directory_branches(&repo.repository_id, "b", "l", "alice", &new_id("op"))
            .unwrap();
        let request = DirectoryNamespaceBatch {
            kind: "move".into(),
            items: vec![DirectoryNamespaceItem {
                source: DirectoryNamespaceSource {
                    directory_branch_id: m.clone(),
                    expected_tree_id: engine.state.directory_branches[&m].head_tree_id.clone(),
                    path: "a/f.txt".into(),
                    entry_id: original.entry_id.clone(),
                },
                target: Some(DirectoryNamespaceTarget {
                    directory_branch_id: l.clone(),
                    expected_tree_id: engine.state.directory_branches[&l].head_tree_id.clone(),
                    path: "b/f.txt".into(),
                }),
            }],
        };
        engine
            .directory_namespace_batch(&repo.repository_id, &request, "alice", &new_id("op"))
            .unwrap();
        save_path(&mut engine, "b/f.txt", b"F2");
        let moved = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.path == "b/f.txt")
            .unwrap();
        engine
            .select_directory_branches(&repo.repository_id, "a", "n", "alice", &new_id("op"))
            .unwrap();
        let saved_root = engine.state.directory_selections[&repo.repository_id].clone();
        let saved_tree = engine.state.directory_branches[&saved_root]
            .head_tree_id
            .clone();
        let saved_contexts = engine.state.file_contexts.clone();
        let alternate = create(&mut engine, "", "whole-view-draft");
        engine
            .select_default_directory_branch(
                &repo.repository_id,
                &alternate,
                "alice",
                &new_id("op"),
            )
            .unwrap();
        engine
            .select_default_directory_branch(
                &repo.repository_id,
                &saved_root,
                "alice",
                &new_id("op"),
            )
            .unwrap();
        assert_eq!(
            engine.state.directory_branches[&saved_root].head_tree_id,
            saved_tree
        );
        assert_eq!(engine.state.file_contexts, saved_contexts);
        let manifest = HistoryManifest::from_state(&engine.state, &repo.history_tree_id).unwrap();
        let a_key = format!("{}/a/f.txt", repo.repository_id);
        let b_key = format!("{}/b/f.txt", repo.repository_id);
        assert_eq!(
            manifest.file_contexts[&a_key].inherited_from.as_deref(),
            Some(n.as_str())
        );
        assert!(manifest.file_contexts[&b_key].inherited_from.is_none());
        assert_ne!(
            manifest.file_contexts[&a_key].branch_id,
            manifest.file_contexts[&b_key].branch_id
        );
        assert_eq!(manifest.file_locations[&original.file_id], "b/f.txt");
        let package = temporary.path().join("package");
        engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        let restored_root = temporary.path().join("restored");
        let mut restored = initialize(&restored_root);
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(restored.state.file_contexts, manifest.file_contexts);
        assert_eq!(restored.state.file_locations, manifest.file_locations);
        assert_eq!(
            fs::read(restored.valid_replica(&original.version_id).unwrap().0).unwrap(),
            b"F1"
        );
        assert_eq!(
            fs::read(restored.valid_replica(&moved.version_id).unwrap().0).unwrap(),
            b"F2"
        );
        save_path(&mut restored, "a/f.txt", b"F3 from old alias");
        let live = restored.list_tree(&repo.main_branch_id).unwrap();
        let a = live.iter().find(|entry| entry.path == "a/f.txt").unwrap();
        let b = live.iter().find(|entry| entry.path == "b/f.txt").unwrap();
        assert_eq!(a.file_id, b.file_id);
        assert_eq!(b.version_id, moved.version_id);
        assert_ne!(a.version_id, original.version_id);
        assert_eq!(restored.state.file_locations[&original.file_id], "b/f.txt");
        assert_eq!(
            restored.state.file_contexts[&b_key],
            manifest.file_contexts[&b_key]
        );
        let latest = a.version_id.clone();
        drop(restored);
        let restored = Engine::open(&restored_root.join("state")).unwrap();
        assert_eq!(
            fs::read(restored.valid_replica(&latest).unwrap().0).unwrap(),
            b"F3 from old alias"
        );
        assert_eq!(
            fs::read(restored.valid_replica(&moved.version_id).unwrap().0).unwrap(),
            b"F2"
        );
    }

    #[test]
    fn rename_nodes_preserve_body_proofs_and_mixed_lineage_through_purge_export_and_restore() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temporary.path().join("source"));
        let repo = engine
            .create_disk_root("P01", "alice", &new_id("op"))
            .unwrap();
        let save_path = |engine: &mut Engine, path: &str, bytes: &[u8]| {
            let workspace = engine
                .create_workspace(&repo.main_branch_id, "alice")
                .unwrap();
            engine
                .stage_bytes(&workspace.workspace_id, path, bytes)
                .unwrap();
            engine
                .commit_workspace(CommitRequest {
                    workspace_id: workspace.workspace_id,
                    operation_id: new_id("op"),
                    principal: "alice".into(),
                    expected_head: workspace.base_head,
                    fencing_token: workspace.fencing_token,
                    message: "node lifecycle".into(),
                    request_digest: None,
                })
                .unwrap();
        };
        save_path(&mut engine, "a/f.txt", b"original body");
        let old = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        let proof = canonical_json(&engine.state.versions[&old.version_id]).unwrap();
        let inode = fs::metadata(engine.valid_replica(&old.version_id).unwrap().0)
            .unwrap()
            .ino();
        for (source, destination) in [("a/f.txt", "a/g.txt"), ("a", "b")] {
            let versions = engine.state.versions.len();
            engine
                .file_namespace(
                    &repo.repository_id,
                    NamespaceOperation {
                        kind: "rename".into(),
                        path: source.into(),
                        destination: Some(destination.into()),
                    },
                    "alice",
                    &new_id("op"),
                )
                .unwrap();
            assert_eq!(engine.state.versions.len(), versions);
            assert_eq!(
                canonical_json(&engine.state.versions[&old.version_id]).unwrap(),
                proof
            );
            assert_eq!(
                fs::metadata(engine.valid_replica(&old.version_id).unwrap().0)
                    .unwrap()
                    .ino(),
                inode
            );
        }
        assert_eq!(engine.state.file_path_nodes.len(), 2);
        let context = &engine.state.file_contexts[&format!("{}/b/g.txt", repo.repository_id)];
        let node = context.node_id.clone().unwrap();
        let parent = engine.state.file_path_nodes[&node].parent_node_ids[0].clone();
        assert_eq!(
            engine.state.file_path_nodes[&parent].parent_node_ids,
            vec![old.version_id.clone()]
        );
        assert_eq!(engine.state.file_path_nodes[&node].previous_path, "a/g.txt");
        assert_eq!(engine.state.file_path_nodes[&node].path, "b/g.txt");
        engine
            .set_file_path_note(
                &old.file_id,
                &node,
                "目录改名，正文未变",
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let body = engine.valid_replica(&old.version_id).unwrap().0;
        let cache: serde_json::Value = read_json(
            &body
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("container.json"),
        )
        .unwrap();
        assert_eq!(cache["versions"].as_array().unwrap().len(), 1);
        assert_eq!(cache["path_nodes"].as_array().unwrap().len(), 2);
        assert!(
            cache["path_nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|record| record["node"]["node_id"] == node
                    && record["note"] == "目录改名，正文未变")
        );
        save_path(&mut engine, "b/g.txt", b"new content after both renames");
        let latest = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        assert_eq!(
            engine.state.file_node_parents[&latest.version_id],
            vec![node.clone()]
        );
        assert_eq!(
            engine.state.versions[&latest.version_id].source_version_ids,
            vec![old.version_id.clone()]
        );
        let plan = engine
            .create_purge_plan(&repo.history_tree_id, vec![old.version_id.clone()], "alice")
            .unwrap();
        engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        let manifest = HistoryManifest::from_state(&engine.state, &repo.history_tree_id).unwrap();
        assert_eq!(manifest.content_states[&old.version_id], "PURGED");
        let mut cyclic = manifest.clone();
        cyclic
            .file_path_nodes
            .get_mut(&parent)
            .unwrap()
            .parent_node_ids = vec![node.clone()];
        assert_eq!(
            cyclic.validate().unwrap_err().code(),
            "HISTORY_PACKAGE_INVALID"
        );
        let mut wrong_body = manifest.clone();
        wrong_body
            .file_path_nodes
            .get_mut(&node)
            .unwrap()
            .version_id = latest.version_id.clone();
        assert_eq!(
            wrong_body.validate().unwrap_err().code(),
            "HISTORY_PACKAGE_INVALID"
        );
        let mut wrong_parent = manifest.clone();
        wrong_parent
            .file_node_parents
            .insert(latest.version_id.clone(), vec![latest.version_id.clone()]);
        assert_eq!(
            wrong_parent.validate().unwrap_err().code(),
            "HISTORY_PACKAGE_INVALID"
        );
        let package = temporary.path().join("package");
        let report = engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        assert_eq!(report.full_files, 1);
        assert_eq!(report.tombstones, 1);
        let mut restored = initialize(&temporary.path().join("restored"));
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            canonical_json(&restored.state.file_path_nodes).unwrap(),
            canonical_json(&manifest.file_path_nodes).unwrap()
        );
        assert_eq!(restored.state.file_node_parents, manifest.file_node_parents);
        assert_eq!(restored.state.file_path_notes, manifest.file_path_notes);
        assert_eq!(
            canonical_json(&restored.state.versions[&old.version_id]).unwrap(),
            proof
        );
        assert_eq!(
            restored.open_content(&old.version_id).err().unwrap().code(),
            "CONTENT_PURGED"
        );
        assert_eq!(
            fs::read(restored.valid_replica(&latest.version_id).unwrap().0).unwrap(),
            b"new content after both renames"
        );
    }

    #[test]
    fn composed_directory_records_validate_and_restore_frozen_shared_file_references() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temp.path().join("source"));
        let repo = engine
            .create_repository("组合恢复", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        let original_commit = save(&mut engine, &repo.main_branch_id, b"frozen parent");
        let original = engine.list_tree(&repo.main_branch_id).unwrap()[0].clone();
        let shared = engine
            .create_file_branch(
                &original.file_id,
                &original.version_id,
                "shared",
                "alice",
                &new_id("op"),
            )
            .unwrap()
            .branch_id
            .unwrap();
        save(&mut engine, &shared, b"shared branch advanced");
        let advanced = engine.list_tree(&shared).unwrap()[0].clone();
        let first = new_id("dbr");
        let second = new_id("dbr");
        let first_tree = new_id("dtree");
        engine
            .mutate(
                "fixture.directory_compositions",
                "alice",
                &new_id("op"),
                json!({}),
                |_, state, _| {
                    let parent = DirectoryBranch {
                        directory_branch_id: first.clone(),
                        repository_id: repo.repository_id.clone(),
                        directory_id: repo.repository_id.clone(),
                        path: String::new(),
                        name: "frozen".into(),
                        parent_branch_id: None,
                        head_tree_id: first_tree.clone(),
                        commit_id: original_commit.clone(),
                        entries: vec![original.clone()],
                        file_branches: BTreeMap::from([(original.file_id.clone(), shared.clone())]),
                        file_contexts: BTreeMap::new(),
                        inherited_files: BTreeMap::new(),
                        directory_refs: BTreeMap::new(),
                        materialized: true,
                        sequence: state.sequence,
                    };
                    let mut child = parent.clone();
                    child.directory_branch_id = second.clone();
                    child.name = "composed".into();
                    child.parent_branch_id = Some(first.clone());
                    child.materialized = false;
                    state.directory_trees.insert(
                        first_tree.clone(),
                        DirectoryTree {
                            tree_id: first_tree.clone(),
                            directory_branch_id: first.clone(),
                            parent_tree_id: None,
                            commit_id: parent.commit_id.clone(),
                            path: parent.path.clone(),
                            entries: parent.entries.clone(),
                            file_branches: parent.file_branches.clone(),
                            file_contexts: parent.file_contexts.clone(),
                            inherited_files: parent.inherited_files.clone(),
                            directory_refs: parent.directory_refs.clone(),
                            sequence: parent.sequence,
                        },
                    );
                    state.directory_branches.insert(first.clone(), parent);
                    state.directory_branches.insert(second.clone(), child);
                    state
                        .directory_selections
                        .insert(repo.repository_id.clone(), second.clone());
                    state
                        .directory_selection_generations
                        .insert(repo.repository_id.clone(), 42);
                    Ok(json!({}))
                },
            )
            .unwrap();
        let manifest = HistoryManifest::from_state(&engine.state, &repo.history_tree_id).unwrap();
        // Shared branches can advance without rewriting an older composition.
        assert_ne!(
            manifest.directory_branches[&first].entries[0].version_id,
            advanced.version_id
        );
        assert_eq!(
            manifest.directory_branches[&first].file_branches[&original.file_id],
            shared
        );
        let bad = |manifest: &HistoryManifest| {
            assert_eq!(
                manifest.validate().unwrap_err().code(),
                "HISTORY_PACKAGE_INVALID"
            )
        };
        let mut invalid = manifest.clone();
        invalid
            .directory_branches
            .get_mut(&first)
            .unwrap()
            .repository_id = new_id("repo");
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_branches
            .get_mut(&first)
            .unwrap()
            .commit_id = new_id("commit");
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid.directory_branches.get_mut(&first).unwrap().entries[0].version_id = new_id("ver");
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_branches
            .get_mut(&first)
            .unwrap()
            .file_branches
            .clear();
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_branches
            .get_mut(&first)
            .unwrap()
            .file_branches
            .insert(original.file_id.clone(), repo.main_branch_id.clone());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_branches
            .get_mut(&first)
            .unwrap()
            .parent_branch_id = Some(second.clone());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_selections
            .insert(new_id("entry"), second.clone());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_trees
            .get_mut(&first_tree)
            .unwrap()
            .parent_tree_id = Some(first_tree.clone());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_branches
            .get_mut(&second)
            .unwrap()
            .head_tree_id = new_id("dtree");
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_trees
            .get_mut(&first_tree)
            .unwrap()
            .entries[0]
            .size += 1;
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_selection_generations
            .insert(new_id("entry"), 1);
        bad(&invalid);
        let mut invalid_json = serde_json::to_value(&manifest).unwrap();
        assert_eq!(
            invalid_json["directory_selection_generations"][&repo.repository_id],
            "42"
        );
        invalid_json["directory_selection_generations"][&repo.repository_id] = json!(42);
        assert!(serde_json::from_value::<HistoryManifest>(invalid_json).is_err());
        let mut invalid = manifest.clone();
        invalid
            .file_locations
            .insert(new_id("file"), "other.txt".into());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .file_locations
            .insert(original.file_id.clone(), "wrong-name.txt".into());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .file_write_origins
            .insert(original.file_id.clone(), new_id("dbr"));
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid.file_selection_generations.insert(new_id("file"), 1);
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid.file_contexts.insert(
            format!("{}/missing.txt", repo.repository_id),
            FileContext {
                node_id: None,
                branch_id: shared.clone(),
                inherited_from: None,
                selection_generation: 0,
            },
        );
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_trees
            .get_mut(&first_tree)
            .unwrap()
            .file_contexts
            .insert(
                "missing.txt".into(),
                FileContext {
                    node_id: None,
                    branch_id: shared.clone(),
                    inherited_from: None,
                    selection_generation: 0,
                },
            );
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_tree_selections
            .insert(repo.repository_id.clone(), new_id("dtree"));
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_trees
            .get_mut(&first_tree)
            .unwrap()
            .inherited_files
            .insert(new_id("file"), first.clone());
        bad(&invalid);
        let mut invalid = manifest.clone();
        invalid
            .directory_trees
            .get_mut(&first_tree)
            .unwrap()
            .directory_refs
            .insert(
                new_id("entry"),
                DirectoryReference {
                    directory_branch_id: first.clone(),
                    tree_id: first_tree.clone(),
                },
            );
        bad(&invalid);

        let package = temp.path().join("package");
        let report = engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        assert_eq!(report.versions, 2);
        assert_eq!(report.full_files, 2);
        let restored_root = temp.path().join("restored");
        let mut restored = initialize(&restored_root);
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            canonical_json(&restored.state.directory_branches).unwrap(),
            canonical_json(&manifest.directory_branches).unwrap()
        );
        assert_eq!(
            canonical_json(&restored.state.directory_trees).unwrap(),
            canonical_json(&manifest.directory_trees).unwrap()
        );
        assert_eq!(
            restored.state.directory_selections,
            manifest.directory_selections
        );
        assert_eq!(
            restored.state.directory_selection_generations,
            manifest.directory_selection_generations
        );
        drop(restored);
        let restored = Engine::open(&restored_root.join("state")).unwrap();
        assert_eq!(
            restored.state.directory_branches[&first].entries[0].version_id,
            original.version_id
        );
        assert_eq!(
            restored.state.directory_selections[&repo.repository_id],
            second
        );
        assert_eq!(
            restored.state.directory_selection_generations[&repo.repository_id],
            42
        );
        let mut content = Vec::new();
        restored
            .open_content(&original.version_id)
            .unwrap()
            .file
            .read_to_end(&mut content)
            .unwrap();
        assert_eq!(content, b"frozen parent");
    }

    #[test]
    fn package_preserves_inherited_references_without_inventing_files_or_versions() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temp.path().join("source"));
        let repo = engine
            .create_repository("引用恢复", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"source bytes");
        let file = engine.list_tree(&repo.main_branch_id).unwrap()[0].clone();
        let before_versions = engine.state.versions.len();
        let result = engine
            .create_file_branch(
                &file.file_id,
                &file.version_id,
                "draft",
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let branch = result.branch_id.unwrap();
        assert_eq!(engine.state.versions.len(), before_versions);
        let selected = engine.list_tree(&branch).unwrap();
        assert_eq!(selected[0].version_id, file.version_id);
        assert_eq!(selected[0].size, file.size);
        let manifest = HistoryManifest::from_state(&engine.state, &repo.history_tree_id).unwrap();
        let mut invalid = manifest.clone();
        invalid
            .branches
            .get_mut(&branch)
            .unwrap()
            .pending_file_versions
            .insert(file.file_id.clone(), new_id("ver"));
        assert_eq!(
            invalid.validate().unwrap_err().code(),
            "HISTORY_PACKAGE_INVALID"
        );
        let pending_commit = engine.state.branches[&branch].head_commit_id.clone();
        assert_eq!(
            manifest.commits[&pending_commit].pending_file_versions[&file.file_id],
            file.version_id
        );
        let mut invalid = manifest.clone();
        invalid
            .commits
            .get_mut(&pending_commit)
            .unwrap()
            .pending_file_versions
            .insert(file.file_id.clone(), new_id("ver"));
        assert_eq!(
            invalid.validate().unwrap_err().code(),
            "HISTORY_PACKAGE_INVALID"
        );
        let package = temp.path().join("package");
        let exported = engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        assert_eq!(exported.full_files, 1);
        assert_eq!(exported.versions, 1);
        assert_eq!(fs::read_dir(package.join("versions")).unwrap().count(), 1);
        assert_eq!(
            offline_check_history_package(&package)
                .unwrap()
                .manifest_hash,
            exported.manifest_hash
        );

        let restored_root = temp.path().join("restored");
        let mut restored = initialize(&restored_root);
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            restored.state.branches[&branch].pending_file_versions[&file.file_id],
            file.version_id
        );
        assert_eq!(restored.state.versions.len(), 1);
        for disk in &restored.config.disks {
            let stub = legacy_placeholder(
                disk,
                &repo.history_tree_id,
                &file.file_id,
                &file.name,
                &branch,
            );
            assert!(!stub.parent().unwrap().exists());
            let cache = stub
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("container.json");
            let metadata: serde_json::Value = read_json(&cache).unwrap();
            assert_eq!(
                metadata["inherited_branches"][0]["version_id"],
                file.version_id
            );
            assert!(metadata["inherited_branches"][0]["own_version_id"].is_null());
            // The optional cache can be rebuilt without manufacturing files.
            fs::remove_file(cache).unwrap();
        }
        drop(restored);
        let mut restored = Engine::open(&restored_root.join("state")).unwrap();
        for disk in &restored.config.disks {
            let stub = legacy_placeholder(
                disk,
                &repo.history_tree_id,
                &file.file_id,
                &file.name,
                &branch,
            );
            assert!(!stub.parent().unwrap().exists());
            let cache = stub
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("container.json");
            let metadata: serde_json::Value = read_json(&cache).unwrap();
            assert_eq!(
                metadata["inherited_branches"][0]["source_version_id"],
                file.version_id
            );
            assert_eq!(
                metadata["inherited_branches"][0]["version_id"],
                file.version_id
            );
        }
        assert_eq!(
            restored.list_tree(&branch).unwrap()[0].version_id,
            file.version_id
        );
        save(&mut restored, &branch, b"first branch bytes");
        assert!(
            restored.state.branches[&branch]
                .pending_file_versions
                .is_empty()
        );
        let new = restored.list_tree(&branch).unwrap()[0].clone();
        assert_eq!(
            restored.state.versions[&new.version_id].source_version_ids,
            vec![file.version_id.clone()]
        );
        assert_eq!(restored.state.versions.len(), 2);
        // Materializing current Branch state must not rewrite the old snapshot:
        // its visible content remains the frozen inherited parent, not new bytes.
        let old = restored.list_tree_at(&branch, &pending_commit).unwrap();
        assert_eq!(old[0].version_id, file.version_id);
        assert_eq!(old[0].size, file.size);
        assert_ne!(old[0].version_id, new.version_id);
        for disk in &restored.config.disks {
            assert!(
                !legacy_placeholder(
                    disk,
                    &repo.history_tree_id,
                    &file.file_id,
                    &file.name,
                    &branch
                )
                .parent()
                .unwrap()
                .exists()
            );
        }
    }

    #[test]
    fn inherited_metadata_replays_after_decision_without_preparing_payloads_or_stubs() {
        for point in ["after_finalize", "after_container_names"] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("source");
            let mut engine = initialize(&root);
            let repo = engine
                .create_repository("回放", "P01", 3, "alice", &new_id("op"))
                .unwrap();
            let source_commit = save(&mut engine, &repo.main_branch_id, b"published source");
            let file = engine.list_tree(&repo.main_branch_id).unwrap()[0].clone();
            let operation = new_id("op");
            engine.inject_fault_once(point);
            assert_eq!(
                engine
                    .create_branch(&source_commit, "pending", "alice", &operation)
                    .unwrap_err()
                    .code(),
                "IN_DOUBT"
            );
            assert_eq!(engine.state.versions.len(), 1);
            drop(engine);
            let mut engine = Engine::open(&root.join("state")).unwrap();
            let branch = engine
                .create_branch(&source_commit, "pending", "alice", &operation)
                .unwrap();
            assert_eq!(engine.state.versions.len(), 1);
            assert_eq!(branch.pending_file_versions[&file.file_id], file.version_id);
            for disk in &engine.config.disks {
                let stub = legacy_placeholder(
                    disk,
                    &repo.history_tree_id,
                    &file.file_id,
                    &file.name,
                    &branch.branch_id,
                );
                assert!(!stub.parent().unwrap().exists());
                let cache = stub
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("container.json");
                let metadata: serde_json::Value = read_json(&cache).unwrap();
                assert_eq!(
                    metadata["inherited_branches"][0]["source_version_id"],
                    file.version_id
                );
                assert_eq!(
                    metadata["inherited_branches"][0]["version_id"],
                    file.version_id
                );
                assert_eq!(
                    fs::read_dir(
                        disk.root
                            .join("histories")
                            .join(&repo.history_tree_id)
                            .join("versions")
                    )
                    .unwrap()
                    .count(),
                    1
                );
            }
            let projected = engine.list_tree(&branch.branch_id).unwrap();
            assert_eq!(projected[0].version_id, file.version_id);
            assert_eq!(projected[0].size, file.size);
        }
    }
    #[test]
    fn split_package_restores_full_history_after_all_original_paths_are_removed() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let source = temp.path().join("original");
        let mut engine = initialize(&source);
        let repository = engine
            .create_repository("来源", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        let cut = save(&mut engine, &repository.main_branch_id, b"first full bytes");
        save(
            &mut engine,
            &repository.main_branch_id,
            b"second full bytes",
        );
        let plan = engine
            .create_split_plan(&repository.history_tree_id, &cut, "alice")
            .unwrap();
        let split = engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        let history = split.history_tree_id.unwrap();
        let package = temp.path().join("history-package");
        let exported = engine
            .export_history_package(&history, &package, "alice")
            .unwrap();
        assert_eq!(exported.full_files, 2);
        let manifest = fs::read_to_string(package.join("manifest.json")).unwrap();
        assert!(!manifest.contains("control_auth"));
        assert!(!manifest.contains("operation_principals"));
        drop(engine);
        fs::remove_dir_all(source).unwrap();
        assert_eq!(
            offline_check_history_package(&package)
                .unwrap()
                .manifest_hash,
            exported.manifest_hash
        );
        let restored_root = temp.path().join("restored");
        let mut restored = initialize(&restored_root);
        let operation = new_id("op");
        let result = restored
            .restore_history_package(&package, "P01", 3, "rescuer", &operation)
            .unwrap();
        assert_eq!(
            restored
                .restore_history_package(&package, "P01", 3, "rescuer", &operation)
                .unwrap()
                .sequence,
            result.sequence
        );
        assert_eq!(restored.state.histories.len(), 1);
        let graph = restored.graph(&history).unwrap();
        assert_eq!(graph.commits.len(), 2);
        let mut contents = Vec::new();
        for commit in &graph.commits {
            for entry in commit.entries.iter().filter(|e| e.kind == "file") {
                let mut content = restored.open_content(&entry.version_id).unwrap();
                let mut bytes = Vec::new();
                content.file.read_to_end(&mut bytes).unwrap();
                contents.push(bytes);
            }
        }
        contents.sort();
        assert_eq!(
            contents,
            vec![b"first full bytes".to_vec(), b"second full bytes".to_vec()]
        );
        let repo = &restored.state.repositories[&graph.history.repository_id];
        assert!(repo.grants.manage.contains("alice"));
        assert!(repo.grants.manage.contains("rescuer"));
        assert!(!repo.smb_enabled);
        assert!(repo.account_scope_repository_id.is_none());
        assert!(result.sequence > exported.publication_sequence);
        let branch = graph.history.main_branch_id.unwrap();
        let next = save(&mut restored, &branch, b"third");
        assert!(restored.state.commits[&next].sequence > result.sequence);
        drop(restored);
        let reopened = Engine::open(&restored_root.join("state")).unwrap();
        assert_eq!(reopened.graph(&history).unwrap().commits.len(), 3);
    }
    #[test]
    fn package_preserves_purge_and_rejects_missing_bytes_or_unpublished_manifest() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temp.path().join("source"));
        let repo = engine
            .create_repository("历史", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"old");
        let old = engine.list_tree(&repo.main_branch_id).unwrap()[0]
            .version_id
            .clone();
        save(&mut engine, &repo.main_branch_id, b"current");
        let purge = engine
            .create_purge_plan(&repo.history_tree_id, vec![old.clone()], "alice")
            .unwrap();
        engine
            .commit_plan(&purge.plan_id, "alice", &new_id("op"))
            .unwrap();
        let package = temp.path().join("package");
        let report = engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        assert_eq!(report.tombstones, 1);
        assert_eq!(report.full_files, 1);
        let mut restored = initialize(&temp.path().join("restored"));
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            restored.open_content(&old).err().unwrap().code(),
            "CONTENT_PURGED"
        );
        let original = fs::read(package.join("manifest.json")).unwrap();
        let mut manifest: HistoryManifest = decode_strict(&original).unwrap();
        manifest.repository.name = "被修改的名字".into();
        write_json(&package.join("manifest.json"), &manifest, true).unwrap();
        assert_eq!(
            offline_check_history_package(&package).unwrap_err().code(),
            "HISTORY_PACKAGE_INVALID"
        );
        write_atomic(&package.join("manifest.json"), &original, true).unwrap();
        let version = manifest
            .versions
            .values()
            .find(|v| manifest.content_states[&v.version_id] == "AVAILABLE")
            .unwrap();
        fs::remove_file(
            package_version(&package, &version.version_id)
                .join("content")
                .join(&version.creation_name),
        )
        .unwrap();
        assert!(offline_check_history_package(&package).is_err());
    }
    #[test]
    fn metadata_merges_labels_and_backup_paths_export_restore_without_rewriting_full_proofs() {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut engine = initialize(&temp.path().join("source"));
        let repo = engine
            .create_repository("metadata recovery", "P01", 1, "alice", &new_id("op"))
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"original body");
        engine
            .initialize_directory_scope(&repo.repository_id, "", "alice", &new_id("op"))
            .unwrap();
        let entry = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        let original_proof = fs::read(
            version_dir(
                engine.disk("P01").unwrap(),
                &repo.history_tree_id,
                &entry.version_id,
            )
            .join("commit-proof.json"),
        )
        .unwrap();
        let branch = engine
            .file_context(&repo.repository_id, "file.txt")
            .unwrap()
            .branch_id;
        let tagged = engine
            .update_file_node_tags(
                &entry.file_id,
                &[FileTagTarget {
                    branch_id: branch.clone(),
                    source_node_id: entry.version_id.clone(),
                    expected_node_id: engine.file_branch_head(&branch).unwrap(),
                }],
                &["release".into(), "中文".into()],
                &[],
                "alice",
                &new_id("op"),
            )
            .unwrap()
            .result["nodes"][0]["node_id"]
            .as_str()
            .unwrap()
            .to_owned();
        engine
            .set_file_node_note(&entry.file_id, &tagged, "tag note", "alice", &new_id("op"))
            .unwrap();
        let left = engine
            .create_file_branch_from_node(
                &entry.file_id,
                &entry.version_id,
                &tagged,
                "left",
                "alice",
                &new_id("op"),
            )
            .unwrap()
            .result["branch_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let right = engine
            .create_file_branch_from_node(
                &entry.file_id,
                &entry.version_id,
                &tagged,
                "right",
                "alice",
                &new_id("op"),
            )
            .unwrap()
            .result["branch_id"]
            .as_str()
            .unwrap()
            .to_owned();
        save(&mut engine, &left, b"left body");
        save(&mut engine, &right, b"right body");
        let chosen = engine.file_branch_head(&branch).unwrap();
        let inputs = [left.as_str(), right.as_str()]
            .into_iter()
            .map(|id| FileMergeInput {
                branch_id: id.into(),
                expected_node_id: engine.file_branch_head(id).unwrap(),
            })
            .collect::<Vec<_>>();
        let merged = engine
            .merge_file_branches(
                &entry.file_id,
                &inputs,
                &chosen,
                &left,
                "alice",
                &new_id("op"),
            )
            .unwrap()
            .result["node_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_ne!(merged, chosen);
        let versions = canonical_json(&engine.state.versions).unwrap();
        engine
            .configure_backups(
                &repo.repository_id,
                "file.txt",
                3,
                &BTreeMap::from([
                    ("B01".into(), "自己选择/备份".into()),
                    ("C01".into(), "独立前缀".into()),
                ]),
                0,
                "alice",
                &new_id("op"),
            )
            .unwrap();
        assert_eq!(canonical_json(&engine.state.versions).unwrap(), versions);
        assert_eq!(
            fs::read(
                version_dir(
                    engine.disk("P01").unwrap(),
                    &repo.history_tree_id,
                    &entry.version_id
                )
                .join("commit-proof.json")
            )
            .unwrap(),
            original_proof
        );
        let obsolete = engine
            .state
            .versions
            .values()
            .filter(|version| {
                version.file_id == entry.file_id && version.version_id != entry.version_id
            })
            .map(|version| version.version_id.clone())
            .collect::<Vec<_>>();
        let purge = engine
            .create_purge_plan(&repo.history_tree_id, obsolete, "alice")
            .unwrap();
        engine
            .commit_plan(&purge.plan_id, "alice", &new_id("op"))
            .unwrap();
        let manifest = HistoryManifest::from_state(&engine.state, &repo.history_tree_id).unwrap();
        assert_eq!(manifest.file_metadata_nodes.len(), 2);
        assert_eq!(manifest.file_branch_merges.len(), 1);
        assert_eq!(manifest.file_branch_aliases[&right], left);
        let mut cycle = manifest.clone();
        cycle
            .file_metadata_nodes
            .get_mut(&tagged)
            .unwrap()
            .parent_node_ids = vec![merged.clone()];
        assert!(cycle.validate().is_err());
        let mut alias_cycle = manifest.clone();
        alias_cycle
            .file_branch_aliases
            .insert(left.clone(), right.clone());
        assert!(alias_cycle.validate().is_err());
        let mut scope = manifest.clone();
        scope
            .file_branch_merges
            .values_mut()
            .next()
            .unwrap()
            .input_branch_ids[0] = repo.main_branch_id.clone();
        assert!(scope.validate().is_err());
        let package = temp.path().join("package");
        engine
            .export_history_package(&repo.history_tree_id, &package, "alice")
            .unwrap();
        let mut restored = initialize(&temp.path().join("restored"));
        restored
            .restore_history_package(&package, "P01", 3, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            canonical_json(&restored.state.file_metadata_nodes).unwrap(),
            canonical_json(&manifest.file_metadata_nodes).unwrap()
        );
        assert_eq!(
            canonical_json(&restored.state.file_branch_merges).unwrap(),
            canonical_json(&manifest.file_branch_merges).unwrap()
        );
        assert_eq!(
            restored.state.file_branch_aliases,
            manifest.file_branch_aliases
        );
        assert_eq!(restored.state.file_path_notes, manifest.file_path_notes);
        assert_eq!(restored.state.file_backup_paths, manifest.file_backup_paths);
        assert_eq!(
            restored.state.file_backup_policies,
            manifest.file_backup_policies
        );
        assert_eq!(
            restored.file_node_tags(&entry.file_id, &merged).unwrap(),
            BTreeSet::from(["release".into(), "中文".into()])
        );
        for disk in &restored.config.disks {
            assert!(offline_check(&disk.root).unwrap().issues.is_empty());
        }
        drop(restored);
        let restored = Engine::open(&temp.path().join("restored/state")).unwrap();
        assert_eq!(restored.file_branch_head(&left).unwrap(), merged);
    }
}
