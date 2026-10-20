use crate::{disk::*, journal::*, *};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

mod config;
mod index;
mod publication;
mod reads;
pub use reads::{ContentReadPlan, RangeContent, WorkspaceReadPlan};
mod recovery;
mod workspace_storage;
use config::{acquire_locks, validate_config};

pub struct Engine {
    pub(crate) root: PathBuf,
    pub(crate) config: Config,
    pub(crate) state: State,
    pub(crate) workspaces: BTreeMap<String, Workspace>,
    pub(crate) blocked: bool,
    pub(crate) snapshot: Option<String>,
    pub(crate) next_slot: u64,
    pub(crate) reads: Arc<Mutex<BTreeMap<String, usize>>>,
    _locks: Vec<File>,
    pub(crate) fault: Option<String>,
}
pub struct ReadLease {
    version: String,
    reads: Arc<Mutex<BTreeMap<String, usize>>>,
}
impl Drop for ReadLease {
    fn drop(&mut self) {
        if let Ok(mut reads) = self.reads.lock() {
            if let Some(count) = reads.get_mut(&self.version) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    reads.remove(&self.version);
                }
            }
        }
    }
}
pub struct OpenedContent {
    pub file: File,
    pub size: u64,
    pub content_hash: String,
    pub lease: ReadLease,
}
pub(crate) struct Prepared {
    pub version: FileVersion,
    pub placement: Placement,
    pub source: PathBuf,
}

impl Engine {
    pub fn initialize(root: &Path, mut config: Config) -> Result<Self> {
        if root.join("config.json").exists() {
            return Err(Error::new(
                "ALREADY_INITIALIZED",
                "configuration already exists; use open/recovery",
            ));
        }
        validate_config(&config)?;
        private_dir(root)?;
        let root = existing_private_dir(root)?.path().to_path_buf();
        for d in &mut config.disks {
            if !d.root.is_absolute() {
                return Err(Error::new("CONFIG_INVALID", "disk roots must be absolute"));
            }
            private_dir(&d.root)?;
            d.root = d.root.canonicalize()?;
            if d.root.join("disk.json").exists() {
                return Err(Error::new(
                    "DISK_ALREADY_ENROLLED",
                    "disk already contains a managed identity",
                ));
            }
        }
        validate_config(&config)?;
        let mut physical = BTreeSet::new();
        for d in &config.disks {
            let seal = identity(&config, d)?;
            if !physical.insert(seal.physical_device.clone()) {
                return Err(Error::new(
                    "DISK_NOT_INDEPENDENT",
                    "two slots reference one physical disk",
                ));
            }
        }
        let locks = acquire_locks(&root, &config, false)?;
        for d in &config.disks {
            let seal = identity(&config, d)?;
            write_json(&d.root.join("disk.json"), &seal, false)?;
            write_atomic(&d.root.join("README.txt"),b"XCS disk format 2. Ordinary full files are stored below histories/. Use xcs offline-check/offline-export before recovery. Development disks are not physical redundancy evidence.\n",false)?;
            for p in [
                "histories",
                "metadata/application-log",
                "metadata/objects",
                "staging",
                "quarantine",
            ] {
                private_dir(&d.root.join(p))?;
            }
            sync_dir(&d.root)?;
        }
        for d in config.disks.iter().filter(|d| d.metadata_member) {
            write_json(
                &d.root.join("metadata/pool-membership/config.json"),
                &config,
                false,
            )?;
        }
        write_json(&root.join("config.json"), &config, false)?;
        private_dir(&root.join("workspaces"))?;
        let state = State {
            topology_generation: 1,
            ..State::default()
        };
        let engine = Self {
            root,
            config,
            state,
            workspaces: BTreeMap::new(),
            blocked: false,
            snapshot: None,
            next_slot: 1,
            reads: Arc::new(Mutex::new(BTreeMap::new())),
            _locks: locks,
            fault: None,
        };
        engine.rebuild_index()?;
        Ok(engine)
    }
    pub fn open(root: &Path) -> Result<Self> {
        Self::open_mode(root, true)
    }
    /// Offline recovery entry. Unpublished candidates are never advanced here.
    pub fn open_for_recovery(root: &Path) -> Result<Self> {
        Self::open_mode(root, false)
    }
    fn open_mode(root: &Path, complete_unpublished: bool) -> Result<Self> {
        let root = existing_private_dir(root)?.path().to_path_buf();
        let config: Config = read_json(&root.join("config.json"))?;
        validate_config(&config)?;
        let locks = acquire_locks(&root, &config, true)?;
        let mut engine = Self {
            root,
            config,
            state: State {
                topology_generation: 1,
                ..State::default()
            },
            workspaces: BTreeMap::new(),
            blocked: false,
            snapshot: None,
            next_slot: 1,
            reads: Arc::new(Mutex::new(BTreeMap::new())),
            _locks: locks,
            fault: None,
        };
        engine.load_workspaces()?;
        engine.recover_mode(complete_unpublished)?;
        engine.load_workspaces()?;
        engine.rebuild_index()?;
        Ok(engine)
    }
    /// Explicit recovery after loss of the system disk. Never initializes data disks.
    pub fn restore_from_disks(root: &Path, disks: &[PathBuf]) -> Result<Self> {
        if root.join("config.json").exists() {
            return Err(Error::new(
                "ALREADY_INITIALIZED",
                "restore requires a new system state directory",
            ));
        }
        if disks.len() < 2 {
            return Err(Error::new(
                "METADATA_QUORUM_UNAVAILABLE",
                "restore requires at least two trusted old metadata members",
            ));
        }
        let mut configs = BTreeMap::<String, (Config, usize)>::new();
        let mut seals = BTreeMap::new();
        for path in disks {
            let seal: DiskSeal = read_json(&path.join("disk.json"))?;
            let config: Config = read_json(&path.join("metadata/pool-membership/config.json"))?;
            let hash = object_hash("pool-config", &config)?;
            configs
                .entry(hash)
                .and_modify(|(_, n)| *n += 1)
                .or_insert((config, 1));
            if seals.insert(seal.disk_id, path.canonicalize()?).is_some() {
                return Err(Error::new(
                    "DISK_NOT_INDEPENDENT",
                    "duplicate disk id in recovery input",
                ));
            }
        }
        let mut config = configs
            .into_values()
            .filter(|(_, n)| *n >= 2)
            .map(|(c, _)| c)
            .next()
            .ok_or_else(|| {
                Error::new(
                    "CONFIG_CONFLICT",
                    "two metadata members do not agree on pool configuration",
                )
            })?;
        for d in &mut config.disks {
            if let Some(path) = seals.get(&d.disk_id) {
                d.root = path.clone();
            }
        }
        private_dir(root)?;
        write_json(&root.join("config.json"), &config, false)?;
        Self::open(root)
    }
    pub(crate) fn require_recovery_locks(&self) -> Result<()> {
        if self._locks.len() != self.config.disks.len() + 1 {
            return Err(Error::new(
                "RECOVERY_REQUIRED",
                "recovery requires every original disk and its exclusive OS writer lock",
            ));
        }
        for disk in &self.config.disks {
            validate_seal(&self.config, disk)?;
        }
        Ok(())
    }
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn is_read_only(&self) -> bool {
        self.blocked
    }
    pub fn auth_state(&self) -> Option<serde_json::Value> {
        self.state.control_auth.clone()
    }
    /// Trusted local control-plane hook; this method is never a public route.
    pub fn persist_auth_state(
        &mut self,
        snapshot: serde_json::Value,
        principal: &str,
        operation: &str,
    ) -> Result<()> {
        if !snapshot.is_object() || canonical_json(&snapshot)?.len() > 4 * 1024 * 1024 {
            return Err(Error::new(
                "AUTH_STATE_INVALID",
                "authentication snapshot is not a bounded object",
            ));
        }
        let digest = object_hash("auth-state", &snapshot)?;
        self.mutate(
            "auth.control-state",
            principal,
            operation,
            json!({"auth_state_digest":digest}),
            |_, state, _| {
                state.control_auth = Some(snapshot);
                Ok(json!({"auth_state_digest":digest}))
            },
        )?;
        Ok(())
    }
    pub fn workspace(&self, id: &str) -> Result<Workspace> {
        self.workspaces
            .get(id)
            .cloned()
            .ok_or_else(|| Error::new("WORKSPACE_NOT_FOUND", "workspace does not exist"))
    }
    pub fn operation_owner(&self, id: &str) -> Option<String> {
        if let Some(owner) = self.state.operation_principals.get(id) {
            return Some(owner.clone());
        }
        for d in self.members() {
            if let Ok(scan) = scan(&d) {
                for (r, _) in scan
                    .records
                    .iter()
                    .rev()
                    .filter(|(r, _)| matches!(r.record_type.as_str(), "ACCEPT" | "PUBLICATION"))
                {
                    if let Ok(c) = load_object::<Candidate>(&d, "candidate", &r.candidate_hash) {
                        if c.operation_id == id {
                            if let Ok(s) = load_snapshot(&d, &c.snapshot_hash) {
                                return s.operation_principals.get(id).cloned();
                            }
                        }
                    }
                }
            }
        }
        None
    }
    pub fn operation(&self, id: &str) -> Result<OperationResult> {
        if let Some(mut op) = self.state.operations.get(id).cloned() {
            if let Some(commit) = op
                .commit_id
                .as_ref()
                .and_then(|c| self.state.commits.get(c))
            {
                let health = commit
                    .entries
                    .iter()
                    .filter(|e| e.kind == "file")
                    .map(|e| {
                        self.version_health(&e.version_id)
                            .unwrap_or_else(|_| "UNAVAILABLE".into())
                    })
                    .collect::<Vec<_>>();
                if health.iter().any(|s| s == "UNAVAILABLE" || s == "PURGED") {
                    op.current_health = "UNAVAILABLE".into();
                } else if health.iter().any(|s| s == "DEGRADED") {
                    op.current_health = "DEGRADED".into();
                }
            }
            return Ok(op);
        }
        for d in self.config.disks.iter().filter(|d| d.metadata_member) {
            if let Ok(scan) = scan(d) {
                for (record, _) in scan
                    .records
                    .iter()
                    .rev()
                    .filter(|(r, _)| matches!(r.record_type.as_str(), "ACCEPT" | "PUBLICATION"))
                {
                    if let Ok(c) = load_object::<Candidate>(d, "candidate", &record.candidate_hash)
                    {
                        if c.operation_id == id {
                            let state = load_snapshot(d, &c.snapshot_hash)?;
                            if let Some(mut op) = state.operations.get(id).cloned() {
                                op.commit_state = "IN_DOUBT".into();
                                op.protection_at_publish = "UNCONFIRMED".into();
                                op.current_health = "RECOVERING".into();
                                return Ok(op);
                            }
                        }
                    }
                }
            }
        }
        Err(Error::new(
            "OPERATION_NOT_FOUND",
            "operation is not registered",
        ))
    }
    /// Deterministic injection only, never enabled by network or environment input.
    pub fn inject_fault_once(&mut self, point: &str) {
        self.fault = Some(point.to_string());
    }
    pub(crate) fn failpoint(&mut self, point: &str) -> Result<()> {
        if self.fault.as_deref() == Some(point) {
            self.fault = None;
            return Err(Error::new(
                "INJECTED_FAILURE",
                format!("interrupted at {point}"),
            ));
        }
        Ok(())
    }
    pub(crate) fn members(&self) -> Vec<DiskConfig> {
        let mut members = self
            .config
            .disks
            .iter()
            .filter(|d| d.metadata_member)
            .cloned()
            .collect::<Vec<_>>();
        members.sort_by(|a, b| a.disk_id.cmp(&b.disk_id));
        members
    }
    pub(crate) fn disk(&self, slot: &str) -> Result<&DiskConfig> {
        self.config
            .disks
            .iter()
            .find(|d| d.slot_id.eq_ignore_ascii_case(slot))
            .ok_or_else(|| Error::new("SLOT_NOT_FOUND", "storage slot is not registered"))
    }
    pub(crate) fn route(&self, primary: &str, copies: u32) -> Result<Vec<String>> {
        let first = self.disk(primary)?;
        if first.level != 1 || copies == 0 {
            return Err(Error::new(
                "PLACEMENT_INVALID",
                "placement requires a primary slot and at least one copy",
            ));
        }
        let mut route = vec![first.slot_id.clone()];
        let mut current = first;
        while route.len() < copies as usize {
            let next = self
                .config
                .disks
                .iter()
                .find(|d| {
                    d.level == current.level + 1
                        && d.upstream_slot_id
                            .as_deref()
                            .is_some_and(|s| s.eq_ignore_ascii_case(&current.slot_id))
                })
                .ok_or_else(|| {
                    Error::new(
                        "ROUTE_INCOMPLETE",
                        "requested copies exceed the configured fixed route",
                    )
                })?;
            route.push(next.slot_id.clone());
            current = next;
        }
        Ok(route)
    }
    pub(crate) fn require_write(&self) -> Result<()> {
        if self.blocked {
            return Err(Error::new(
                "RECOVERY_REQUIRED",
                "an unresolved decision or unavailable metadata member blocks writes",
            )
            .retryable());
        }
        for d in self.members() {
            validate_seal(&self.config, &d)?;
        }
        Ok(())
    }
    pub(crate) fn require_grant(
        &self,
        repo: &Repository,
        principal: &str,
        action: &str,
    ) -> Result<()> {
        let set = match action {
            "read" => &repo.grants.read,
            "write" => &repo.grants.write,
            _ => &repo.grants.manage,
        };
        if repo.grants.manage.contains(principal) || set.contains(principal) {
            Ok(())
        } else {
            Err(Error::new(
                "PERMISSION_DENIED",
                "principal has no permission for this operation",
            ))
        }
    }
    pub(crate) fn repository_for_history(&self, history: &str) -> Result<&Repository> {
        let h = self
            .state
            .histories
            .get(history)
            .ok_or_else(|| Error::new("HISTORY_NOT_FOUND", "history tree does not exist"))?;
        self.state
            .repositories
            .get(&h.repository_id)
            .ok_or_else(|| Error::new("STATE_INVALID", "history has no repository"))
    }
    pub(crate) fn repository_for_branch(&self, branch: &str) -> Result<&Repository> {
        let b = self
            .state
            .branches
            .get(branch)
            .ok_or_else(|| Error::new("BRANCH_NOT_FOUND", "branch does not exist"))?;
        self.repository_for_history(&b.history_tree_id)
    }
    pub(crate) fn mutate<F>(
        &mut self,
        kind: &str,
        principal: &str,
        operation_id: &str,
        input: serde_json::Value,
        change: F,
    ) -> Result<OperationResult>
    where
        F: FnOnce(&Self, &mut State, &mut Vec<Prepared>) -> Result<serde_json::Value>,
    {
        self.mutate_digest(kind, principal, operation_id, input, None, change)
    }
    pub(crate) fn mutate_digest<F>(
        &mut self,
        kind: &str,
        principal: &str,
        operation_id: &str,
        input: serde_json::Value,
        digest_override: Option<String>,
        change: F,
    ) -> Result<OperationResult>
    where
        F: FnOnce(&Self, &mut State, &mut Vec<Prepared>) -> Result<serde_json::Value>,
    {
        validate_id(operation_id, "op")?;
        if principal.is_empty() {
            return Err(Error::new("UNAUTHENTICATED", "principal is required"));
        }
        let digest = digest_override.map(Ok).unwrap_or_else(|| {
            object_hash(
                "request",
                &json!({"kind":kind,"principal":principal,"body":input}),
            )
        })?;
        if let Some(result) = self.state.operations.get(operation_id) {
            if result.request_digest == digest
                && self.operation_owner(operation_id).as_deref() == Some(principal)
            {
                return self.operation(operation_id);
            }
            return Err(Error::new(
                "IDEMPOTENCY_KEY_REUSED",
                "operation id is already bound to different input",
            ));
        }
        self.require_write()?;
        let mut next = self.state.clone();
        crate::container_storage::capture_names(&mut next);
        next.sequence = self.next_slot;
        if !kind.starts_with("auth.") {
            next.business_generation = next
                .business_generation
                .checked_add(1)
                .ok_or_else(|| Error::new("RESOURCE_LIMIT", "business generation exhausted"))?;
        }
        let mut prepared = Vec::new();
        let mut result = change(self, &mut next, &mut prepared)?;
        crate::container_storage::capture_names(&mut next);
        result["timestamp"] = serde_json::json!(crate::business::now().to_string());
        if let Some(branch_id) = result
            .get("branch_id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
        {
            if let Some(branch) = next.branches.get(&branch_id) {
                result["branch_generation"] = json!(branch.generation.to_string());
            }
        }
        result["membership_epoch"] = json!("1");
        let mut protection = Vec::new();
        for p in &prepared {
            let mut slots = Vec::new();
            for slot in &p.placement.route_slot_ids {
                let disk = self.disk(slot)?;
                slots.push(json!({"slot_id":slot,"copy_level":disk.level,"disk_id":disk.disk_id,"binding_generation":disk.binding_generation.to_string(),"state":"COMPLETE_VERIFIED"}));
            }
            protection.push(json!({"version_id":p.version.version_id,"primary_slot_id":p.placement.primary_slot_id,"target_copies":p.placement.target_copies,"required_slots":p.placement.route_slot_ids,"topology_generation":p.placement.topology_generation.to_string(),"slots":slots}));
        }
        result["version_protection"] = json!(protection);
        let protection = if prepared.is_empty() {
            "NOT_APPLICABLE"
        } else {
            "TARGET_MET"
        };
        let op = OperationResult {
            operation_id: operation_id.into(),
            kind: kind.into(),
            commit_id: result
                .get("commit_id")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            history_tree_id: result
                .get("history_tree_id")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            branch_id: result
                .get("branch_id")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            commit_state: "PUBLISHED".into(),
            protection_at_publish: protection.into(),
            current_health: "HEALTHY".into(),
            request_digest: digest.clone(),
            sequence: next.sequence,
            result,
        };
        next.operations.insert(operation_id.into(), op.clone());
        next.operation_principals
            .insert(operation_id.into(), principal.into());
        next.events.push(Event {
            sequence: next.sequence,
            operation_id: operation_id.into(),
            kind: kind.into(),
            object_id: op
                .commit_id
                .clone()
                .or_else(|| op.history_tree_id.clone())
                .unwrap_or_else(|| operation_id.into()),
        });
        self.persist(next, prepared, op.clone())?;
        Ok(op)
    }
}
