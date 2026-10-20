use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Production,
    Development,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskConfig {
    pub slot_id: String,
    pub disk_id: String,
    pub root: PathBuf,
    pub level: u32,
    pub upstream_slot_id: Option<String>,
    pub metadata_member: bool,
    #[serde(with = "crate::u64_string")]
    pub binding_generation: u64,
    /// Optional explicit admission budget, in addition to actual free space.
    #[serde(default, with = "crate::optional_u64_string")]
    pub capacity_bytes: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub pool_id: String,
    pub mode: Mode,
    pub disks: Vec<DiskConfig>,
    pub checksum_chunk_bytes: u32,
    #[serde(with = "crate::u64_string")]
    pub max_file_bytes: u64,
    #[serde(with = "crate::u64_string")]
    pub max_workspace_bytes: u64,
    pub max_workspaces: usize,
    pub max_entries: usize,
    #[serde(with = "crate::u64_string")]
    pub metadata_reserve_bytes: u64,
    #[serde(with = "crate::u64_string")]
    pub workspace_ttl_seconds: u64,
}
impl Config {
    pub fn development(disks: [PathBuf; 3]) -> Self {
        let slots = ["P01", "B01", "C01"];
        Self {
            pool_id: crate::new_id("pool"),
            mode: Mode::Development,
            disks: disks
                .into_iter()
                .enumerate()
                .map(|(i, root)| DiskConfig {
                    slot_id: slots[i].into(),
                    disk_id: crate::new_id("disk"),
                    root,
                    level: i as u32 + 1,
                    upstream_slot_id: i.checked_sub(1).map(|n| slots[n].into()),
                    metadata_member: true,
                    binding_generation: 1,
                    capacity_bytes: None,
                })
                .collect(),
            checksum_chunk_bytes: 262144,
            max_file_bytes: 1 << 40,
            max_workspace_bytes: 1 << 40,
            max_workspaces: 128,
            max_entries: 100_000,
            metadata_reserve_bytes: 64 << 20,
            workspace_ttl_seconds: 86_400,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Grants {
    pub read: BTreeSet<String>,
    pub write: BTreeSet<String>,
    pub manage: BTreeSet<String>,
}
/// Repository ACL capability. Read and write are independent; manage grants both.
/// Account admission and administrator identity are evaluated by the transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryAccess {
    Read,
    Write,
    Manage,
}

impl Grants {
    pub fn allows(&self, principal: &str, access: RepositoryAccess) -> bool {
        self.manage.contains(principal)
            || match access {
                RepositoryAccess::Read => self.read.contains(principal),
                RepositoryAccess::Write => self.write.contains(principal),
                RepositoryAccess::Manage => false,
            }
    }

    pub fn owner(principal: &str) -> Self {
        let set = BTreeSet::from([principal.to_string()]);
        Self {
            read: set.clone(),
            write: set.clone(),
            manage: set,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    pub repository_id: String,
    pub name: String,
    pub history_tree_id: String,
    pub main_branch_id: String,
    pub primary_slot_id: String,
    pub target_copies: u32,
    pub name_policy: String,
    pub unicode_version: String,
    pub grants: Grants,
    /// Account admission scope inherited when a history is split; grants remain local.
    #[serde(default)]
    pub account_scope_repository_id: Option<String>,
    #[serde(default = "enabled")]
    pub smb_enabled: bool,
    /// A controlled disk-root namespace, created lazily by the file manager.
    #[serde(default, skip_serializing_if = "is_false")]
    pub file_manager_root: bool,
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn enabled() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HistoryTree {
    pub history_tree_id: String,
    pub repository_id: String,
    pub root_commit_id: String,
    pub main_branch_id: Option<String>,
    #[serde(with = "crate::u64_string")]
    pub generation: u64,
    pub state: String,
    pub provenance: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub branch_id: String,
    pub history_tree_id: String,
    pub name: String,
    pub head_commit_id: String,
    pub fork_commit_id: String,
    pub branch_kind: String,
    /// File branches contain only this stable file identity and its parents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_node_id: Option<String>,
    /// Unwritten file identities and frozen parent versions. Reads inherit
    /// these versions until a first write creates branch-owned content.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pending_file_versions: BTreeMap<String, String>,
    #[serde(with = "crate::u64_string")]
    pub generation: u64,
    pub state: String,
}
/// A directory view composes shared file branches at frozen content versions.
/// It owns neither private file branches nor payloads. Edits advance its
/// immutable subtree head without changing the named branch identity.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DirectoryBranch {
    pub directory_branch_id: String,
    pub repository_id: String,
    pub directory_id: String,
    pub path: String,
    pub name: String,
    pub parent_branch_id: Option<String>,
    pub head_tree_id: String,
    pub commit_id: String,
    pub entries: Vec<Entry>,
    pub file_branches: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_contexts: BTreeMap<String, FileContext>,
    /// Frozen write intent: inherited entries fork publicly only on real save.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inherited_files: BTreeMap<String, String>,
    /// Exact child composition references; never follow a mutable HEAD.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_refs: BTreeMap<String, DirectoryReference>,
    pub materialized: bool,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DirectoryTree {
    pub tree_id: String,
    pub directory_branch_id: String,
    pub parent_tree_id: Option<String>,
    pub commit_id: String,
    pub path: String,
    pub entries: Vec<Entry>,
    pub file_branches: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_contexts: BTreeMap<String, FileContext>,
    /// Frozen write intent: inherited entries fork publicly only on real save.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub inherited_files: BTreeMap<String, String>,
    /// Exact child composition references; never follow a mutable HEAD.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_refs: BTreeMap<String, DirectoryReference>,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
}
/// An immutable namespace event in a public file's history. The body remains
/// the referenced real FileVersion; a rename creates no payload or fake FULL.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FilePathNode {
    pub node_id: String,
    pub history_tree_id: String,
    pub file_id: String,
    pub version_id: String,
    pub parent_node_ids: Vec<String>,
    pub branch_id: String,
    pub path: String,
    pub previous_path: String,
    pub operation_id: String,
    pub author: String,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
    #[serde(with = "crate::u64_string")]
    pub created_at: u64,
}

/// An immutable metadata revision. Its body is an existing verified FULL;
/// metadata edits never manufacture a content version or alter its proof.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FileMetadataNode {
    pub node_id: String,
    pub history_tree_id: String,
    pub file_id: String,
    pub version_id: String,
    pub parent_node_ids: Vec<String>,
    pub branch_id: String,
    pub kind: String,
    pub tags: BTreeSet<String>,
    pub operation_id: String,
    pub author: String,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
    #[serde(with = "crate::u64_string")]
    pub created_at: u64,
}

/// Merge provenance is separate from immutable content/path parent proofs.
/// Reusing an existing head records every input even when an ancestry edge
/// must be omitted to keep the displayed history acyclic.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FileBranchMerge {
    pub merge_id: String,
    pub history_tree_id: String,
    pub file_id: String,
    pub input_branch_ids: Vec<String>,
    pub input_node_ids: Vec<String>,
    pub chosen_node_id: String,
    pub result_node_id: String,
    pub surviving_branch_id: String,
    pub reused_node: bool,
    pub parent_node_ids: Vec<String>,
    pub operation_id: String,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileMergeInput {
    pub branch_id: String,
    pub expected_node_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileTagTarget {
    pub branch_id: String,
    pub source_node_id: String,
    pub expected_node_id: String,
}

/// A logical path's write target; several aliases of one identity may expose
/// different immutable versions and independently inherited write intent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero", with = "crate::u64_string")]
    pub selection_generation: u64,
    pub branch_id: String,
    pub inherited_from: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DirectoryReference {
    pub directory_branch_id: String,
    pub tree_id: String,
}

/// One atomic namespace request, with an explicit immutable scope per item.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryNamespaceBatch {
    pub kind: String,
    pub items: Vec<DirectoryNamespaceItem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryNamespaceItem {
    pub source: DirectoryNamespaceSource,
    pub target: Option<DirectoryNamespaceTarget>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryNamespaceSource {
    pub directory_branch_id: String,
    pub expected_tree_id: String,
    pub path: String,
    pub entry_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryNamespaceTarget {
    pub directory_branch_id: String,
    pub expected_tree_id: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub entry_id: String,
    pub path: String,
    pub name: String,
    pub lookup_key: String,
    pub kind: String,
    pub file_id: String,
    pub version_id: String,
    #[serde(with = "crate::u64_string")]
    pub size: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub commit_id: String,
    pub history_tree_id: String,
    pub kind: String,
    pub parent_commit_ids: Vec<String>,
    pub tree_id: String,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
    pub operation_id: String,
    pub author: String,
    pub message: String,
    pub entries: Vec<Entry>,
    /// Snapshot-local reservations preserve inherited parent views even after
    /// the live branch later gains its own content versions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pending_file_versions: BTreeMap<String, String>,
    pub provenance: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FileVersion {
    pub history_tree_id: String,
    pub version_id: String,
    pub file_id: String,
    pub creation_primary_slot_id: String,
    #[serde(with = "crate::u64_string")]
    pub logical_size: u64,
    #[serde(with = "crate::u64_string")]
    pub version_sequence: u64,
    pub content_hash: String,
    pub checksum_chunk_bytes: u32,
    pub checksum_manifest_hash: String,
    pub creation_name: String,
    pub storage_name: String,
    pub created_at: String,
    pub source_version_ids: Vec<String>,
    pub created_as_full_checkpoint: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub version_id: String,
    pub primary_slot_id: String,
    pub target_copies: u32,
    pub route_slot_ids: Vec<String>,
    #[serde(with = "crate::u64_string")]
    pub topology_generation: u64,
    #[serde(with = "crate::u64_string_map")]
    pub binding_generations: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MergeLink {
    pub merge_link_id: String,
    pub history_tree_id: String,
    pub source_branch_id: String,
    pub source_commit_id: String,
    pub target_branch_id: String,
    pub target_commit_id: String,
    pub author: String,
    pub operation_id: String,
}
/// Audit-only edge across a split boundary. External identifiers are provenance,
/// never traversed as live commits or branches of this history.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HistoryBoundary {
    pub boundary_id: String,
    pub history_tree_id: String,
    pub local_commit_id: String,
    pub local_branch_id: Option<String>,
    pub relation: String,
    pub external_history_tree_id: String,
    pub external_commit_id: String,
    pub source_merge_link_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagedFile {
    pub item_id: String,
    pub path: String,
    pub source_path: PathBuf,
    #[serde(with = "crate::u64_string")]
    pub size: u64,
    pub content_hash: String,
    #[serde(with = "crate::u64_string")]
    pub dirty_generation: u64,
    pub base_version_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamespaceOperation {
    pub kind: String,
    pub path: String,
    pub destination: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub workspace_id: String,
    #[serde(default = "normal_workspace_purpose")]
    pub purpose: String,
    pub branch_id: String,
    pub principal: String,
    pub base_head: String,
    #[serde(with = "crate::u64_string")]
    pub fencing_token: u64,
    pub files: BTreeMap<String, StagedFile>,
    /// Freeze the file selections visible when a directory workspace opens.
    /// Two unwritten branches can share a source version but are distinct
    /// destinations, so comparing version IDs alone cannot fence a switch.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub selected_file_branches: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub selected_file_contexts: BTreeMap<String, FileContext>,
    /// Selected composition identities at open time; selection is a write even
    /// when two directory snapshots happen to expose identical file bytes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub selected_directory_branches: BTreeMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "crate::u64_string_map"
    )]
    pub directory_selection_generations: BTreeMap<String, u64>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "crate::u64_string_map"
    )]
    pub file_selection_generations: BTreeMap<String, u64>,
    pub namespace_operations: Vec<NamespaceOperation>,
    #[serde(with = "crate::u64_string")]
    pub created_at: u64,
    pub state: String,
}
fn normal_workspace_purpose() -> String {
    "normal".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitRequest {
    pub workspace_id: String,
    pub operation_id: String,
    pub principal: String,
    pub expected_head: String,
    #[serde(with = "crate::u64_string")]
    pub fencing_token: u64,
    pub message: String,
    #[serde(default)]
    pub request_digest: Option<String>,
}
/// Closed publication lifecycle vocabulary shared by persisted and wire results.
/// Serde names are part of the hashed on-disk contract.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommitState {
    Staging,
    Sealed,
    Prepared,
    Deciding,
    Decided,
    Protected,
    Published,
    Acked,
    Recovering,
    RecoveryRequired,
    InDoubt,
    PublishedDegraded,
    Aborted,
}

/// Frozen evidence at publication, not a live assessment of copy health.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtectionAtPublish {
    NotApplicable,
    TargetMet,
    BelowTarget,
    Unconfirmed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    pub operation_id: String,
    pub kind: String,
    pub commit_id: Option<String>,
    pub history_tree_id: Option<String>,
    pub branch_id: Option<String>,
    pub commit_state: CommitState,
    pub protection_at_publish: ProtectionAtPublish,
    pub current_health: String,
    pub request_digest: String,
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
    pub result: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub plan_id: String,
    pub kind: String,
    pub principal: String,
    #[serde(with = "crate::u64_string")]
    pub expected_sequence: u64,
    pub digest: String,
    pub input: serde_json::Value,
    pub state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlacementRule {
    pub rule_id: String,
    pub repository_id: String,
    pub path_prefix: String,
    pub primary_slot_id: String,
    #[serde(with = "crate::u64_string")]
    pub generation: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProtectionPolicy {
    pub policy_id: String,
    pub repository_id: String,
    pub path_prefix: String,
    pub target_copies: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_directory_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_file_id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub backup_prefixes: BTreeMap<String, String>,
    #[serde(with = "crate::u64_string")]
    pub generation: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct State {
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
    #[serde(default, with = "crate::u64_string")]
    pub business_generation: u64,
    pub repositories: BTreeMap<String, Repository>,
    pub histories: BTreeMap<String, HistoryTree>,
    pub branches: BTreeMap<String, Branch>,
    /// Selected independent content branch for each file in its disk namespace.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_selections: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_branches: BTreeMap<String, DirectoryBranch>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_trees: BTreeMap<String, DirectoryTree>,
    /// Current projection intent, separate from each public file branch HEAD.
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
    /// Consumed names remain readable; writes must use the surviving branch.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_branch_aliases: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_node_parents: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_path_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_tree_selections: BTreeMap<String, String>,
    /// Physical owner path; selecting an old logical alias never relocates it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_locations: BTreeMap<String, String>,
    /// User-chosen backup owner paths are independent of primary aliases.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_backup_paths: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_backup_policies: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub directory_selections: BTreeMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "crate::u64_string_map"
    )]
    pub directory_selection_generations: BTreeMap<String, u64>,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "crate::u64_string_map"
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
    #[serde(default)]
    pub history_boundaries: BTreeMap<String, HistoryBoundary>,
    pub content_states: BTreeMap<String, String>,
    pub operations: BTreeMap<String, OperationResult>,
    pub plans: BTreeMap<String, Plan>,
    #[serde(default)]
    pub placement_rules: BTreeMap<String, PlacementRule>,
    #[serde(default)]
    pub protection_policies: BTreeMap<String, ProtectionPolicy>,
    #[serde(default)]
    pub operation_principals: BTreeMap<String, String>,
    /// Private authentication control state; never expose State as a public API DTO.
    #[serde(default)]
    pub control_auth: Option<serde_json::Value>,
    pub events: Vec<Event>,
    #[serde(with = "crate::u64_string")]
    pub topology_generation: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(with = "crate::u64_string")]
    pub sequence: u64,
    pub operation_id: String,
    pub kind: String,
    pub object_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryGraph {
    pub history: HistoryTree,
    pub commits: Vec<Commit>,
    pub branches: Vec<Branch>,
    pub merge_links: Vec<MergeLink>,
    #[serde(default)]
    pub boundary_marks: Vec<HistoryBoundary>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskStatus {
    pub slot_id: String,
    pub disk_id: String,
    pub level: u32,
    pub upstream_slot_id: Option<String>,
    pub online: bool,
    pub metadata_member: bool,
    pub physical_device: String,
    #[serde(with = "crate::u64_string")]
    pub available_bytes: u64,
    #[serde(with = "crate::u64_string")]
    pub used_bytes: u64,
    pub simulation: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckReport {
    pub checked_versions: usize,
    pub healthy_replicas: usize,
    pub tombstoned_versions: usize,
    pub issues: Vec<String>,
}

#[cfg(test)]
mod repository_access_tests {
    use super::{Grants, RepositoryAccess};

    #[test]
    fn capabilities_are_independent_and_manage_implies_both() {
        for bits in 0..8 {
            let mut grants = Grants::default();
            if bits & 1 != 0 {
                grants.read.insert("user".into());
            }
            if bits & 2 != 0 {
                grants.write.insert("user".into());
            }
            if bits & 4 != 0 {
                grants.manage.insert("user".into());
            }
            for (access, expected) in [
                (RepositoryAccess::Read, bits & 5 != 0),
                (RepositoryAccess::Write, bits & 6 != 0),
                (RepositoryAccess::Manage, bits & 4 != 0),
            ] {
                assert_eq!(grants.allows("user", access), expected);
                assert!(!grants.allows("other", access));
            }
        }
    }
}

#[cfg(test)]
mod operation_status_tests {
    use super::*;

    // These strings are the existing public commit-state vocabulary, plus the
    // persisted protection values emitted by normal and recovery publication.
    const COMMIT_STATES: &[&str] = &[
        "STAGING",
        "SEALED",
        "PREPARED",
        "DECIDING",
        "DECIDED",
        "PROTECTED",
        "PUBLISHED",
        "ACKED",
        "RECOVERING",
        "RECOVERY_REQUIRED",
        "IN_DOUBT",
        "PUBLISHED_DEGRADED",
        "ABORTED",
    ];
    const PROTECTION_STATES: &[&str] = &[
        "NOT_APPLICABLE",
        "TARGET_MET",
        "BELOW_TARGET",
        "UNCONFIRMED",
    ];

    fn frozen_operation(commit: &str, protection: &str) -> Vec<u8> {
        format!(r#"{{"branch_id":null,"commit_id":null,"commit_state":"{commit}","current_health":"HEALTHY","history_tree_id":null,"kind":"workspace.commit","operation_id":"operation_fixture","protection_at_publish":"{protection}","request_digest":"fixture","result":{{}},"sequence":"18446744073709551615"}}"#).into_bytes()
    }

    #[test]
    fn operation_status_rejects_unknown_or_malformed_values() {
        for (commit, protection) in [
            ("published", "TARGET_MET"),
            ("PUBLISHED_TYPO", "TARGET_MET"),
            ("", "TARGET_MET"),
            ("PUBLISHED", "HEALTHY"),
            ("PUBLISHED", "target_met"),
            ("PUBLISHED", ""),
        ] {
            assert!(
                crate::decode_strict::<OperationResult>(&frozen_operation(commit, protection))
                    .is_err()
            );
        }
        for invalid in ["null", "0", "{}", "[]"] {
            assert!(serde_json::from_str::<CommitState>(invalid).is_err());
            assert!(serde_json::from_str::<ProtectionAtPublish>(invalid).is_err());
        }
    }

    #[test]
    fn operation_status_preserves_frozen_bytes_and_digest() {
        for commit in COMMIT_STATES {
            for protection in PROTECTION_STATES {
                let bytes = frozen_operation(commit, protection);
                let operation: OperationResult = crate::decode_strict(&bytes).unwrap();
                assert_eq!(crate::canonical_json(&operation).unwrap(), bytes);
                let mut expected = blake3::Hasher::new();
                expected.update(b"zxs:state-page\0");
                expected.update(&bytes);
                assert_eq!(
                    crate::object_hash("state-page", &operation).unwrap(),
                    expected.finalize().to_hex().as_str()
                );
            }
        }
    }
}
