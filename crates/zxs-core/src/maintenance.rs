use crate::{
    business::{now, sorted},
    disk::*,
    journal::*,
    *,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

impl Engine {
    fn check_directory_split(&self, history: &str) -> Result<()> {
        if self
            .state
            .file_path_nodes
            .values()
            .any(|node| node.history_tree_id == history)
            || self
                .state
                .file_metadata_nodes
                .values()
                .any(|node| node.history_tree_id == history)
            || self
                .state
                .file_branch_merges
                .values()
                .any(|event| event.history_tree_id == history)
            || self.state.file_node_parents.keys().any(|version| {
                self.state
                    .versions
                    .get(version)
                    .is_some_and(|version| version.history_tree_id == history)
            })
        {
            return Err(Error::new(
                "FILE_NODE_HISTORY_SPLIT_UNSUPPORTED",
                "history splitting does not yet remap immutable file path nodes and their mixed ancestry",
            ));
        }
        if self.state.histories.get(history).is_some_and(|history| {
            self.state
                .directory_branches
                .values()
                .any(|branch| branch.repository_id == history.repository_id)
        }) {
            return Err(Error::new(
                "DIRECTORY_HISTORY_SPLIT_UNSUPPORTED",
                "history splitting does not yet remap shared directory composition records",
            ));
        }
        Ok(())
    }
    pub fn create_purge_plan(
        &mut self,
        history: &str,
        versions: Vec<String>,
        principal: &str,
    ) -> Result<Plan> {
        self.require_write()?;
        self.require_grant(
            self.repository_for_history(history)?,
            principal,
            zxs_contract::RepositoryAccess::Manage,
        )?;
        let mut ids = versions;
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            return Err(Error::new("INVALID_REQUEST", "purge selection is empty"));
        }
        self.check_purge(history, &ids)?;
        self.save_plan(
            "purge",
            principal,
            json!({"history_tree_id":history,"version_ids":ids}),
        )
    }
    pub fn create_split_plan(&mut self, history: &str, cut: &str, principal: &str) -> Result<Plan> {
        self.require_write()?;
        self.require_grant(
            self.repository_for_history(history)?,
            principal,
            zxs_contract::RepositoryAccess::Manage,
        )?;
        self.check_directory_split(history)?;
        if self
            .state
            .branches
            .values()
            .any(|branch| branch.history_tree_id == history && branch.file_id.is_some())
        {
            return Err(Error::new(
                "FILE_HISTORY_SPLIT_UNSUPPORTED",
                "directory history splitting cannot be applied to independent file branches",
            ));
        }
        if !self
            .state
            .commits
            .get(cut)
            .is_some_and(|c| c.history_tree_id == history)
        {
            return Err(Error::new(
                "COMMIT_NOT_FOUND",
                "cut commit is not in the selected history",
            ));
        }
        if self
            .state
            .histories
            .get(history)
            .is_some_and(|h| h.root_commit_id == cut)
        {
            return Err(Error::new(
                "SPLIT_ROOT_FORBIDDEN",
                "a split must leave the source root and prefix history intact",
            ));
        }
        if self.workspaces.values().any(|w| {
            matches!(w.state.as_str(), "open" | "sealed")
                && self
                    .state
                    .branches
                    .get(&w.branch_id)
                    .is_some_and(|b| b.history_tree_id == history)
        }) {
            return Err(Error::new(
                "WORKSPACE_BUSY",
                "split requires all affected workspaces to close",
            ));
        }
        self.save_plan(
            "split",
            principal,
            json!({"history_tree_id":history,"cut_commit_id":cut}),
        )
    }
    fn save_plan(&mut self, kind: &str, principal: &str, input: serde_json::Value) -> Result<Plan> {
        let plan = Plan {
            plan_id: new_id("plan"),
            kind: kind.into(),
            principal: principal.into(),
            expected_sequence: self.state.business_generation,
            digest: object_hash("plan", &input)?,
            input,
            state: "ready".into(),
        };
        // Plans are durable local preparations; business effects still require publication.
        write_json(
            &self
                .root
                .join("plans")
                .join(format!("{}.json", plan.plan_id)),
            &plan,
            false,
        )?;
        self.state.plans.insert(plan.plan_id.clone(), plan.clone());
        Ok(plan)
    }
    pub fn cancel_plan(
        &mut self,
        id: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        if let Some(op) = self.state.operations.get(operation) {
            if op.kind == "plan.cancel"
                && op.result["plan_id"].as_str() == Some(id)
                && self.operation_owner(operation).as_deref() == Some(principal)
            {
                return self.operation(operation);
            }
            return Err(Error::new(
                "IDEMPOTENCY_KEY_REUSED",
                "operation already names a different action",
            ));
        }
        let plan = self.plan(id)?;
        if plan.principal != principal {
            return Err(Error::new(
                "PERMISSION_DENIED",
                "plan belongs to another principal",
            ));
        }
        if plan.state != "ready" {
            return Err(Error::new(
                "PLAN_CLOSED",
                "only an unexecuted plan can be cancelled",
            ));
        }
        self.mutate(
            "plan.cancel",
            principal,
            operation,
            json!({"plan_id":id}),
            |_, state, _| {
                let mut p = plan.clone();
                p.state = "cancelled".into();
                state.plans.insert(id.into(), p);
                Ok(json!({"plan_id":id,"state":"cancelled"}))
            },
        )
    }
    pub fn plan(&self, id: &str) -> Result<Plan> {
        validate_id(id, "plan")?;
        let plan = if let Some(plan) = self.state.plans.get(id) {
            plan.clone()
        } else {
            read_json::<Plan>(&self.root.join("plans").join(format!("{id}.json")))?
        };
        // Every plan kind must execute exactly the input that was previewed.
        // Checking only the stored digest field would accept a stale digest
        // paired with modified local purge/split input after restart.
        if plan.plan_id != id || object_hash("plan", &plan.input)? != plan.digest {
            return Err(Error::new(
                "PLAN_INVALID",
                "plan identity or input digest differs",
            ));
        }
        Ok(plan)
    }
    pub fn commit_plan(
        &mut self,
        id: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        if let Some(op) = self.state.operations.get(operation) {
            if matches!(
                op.kind.as_str(),
                "content.purge" | "history.split" | "history.historical_save"
            ) && op.result["plan_id"].as_str() == Some(id)
                && self.operation_owner(operation).as_deref() == Some(principal)
            {
                return self.operation(operation);
            }
            return Err(Error::new(
                "IDEMPOTENCY_KEY_REUSED",
                "operation already used",
            ));
        }
        let plan = self.plan(id)?;
        if plan.kind == "historical_save" {
            return self.commit_historical_save_plan(id, principal, operation);
        }
        if plan.principal != principal {
            return Err(Error::new(
                "PERMISSION_DENIED",
                "plan belongs to another principal",
            ));
        }
        if plan.state != "ready" {
            return Err(Error::new("PLAN_CLOSED", "plan is no longer ready"));
        }
        if plan.expected_sequence != self.state.business_generation {
            return Err(Error::new(
                "PLAN_STALE",
                "published state changed; preview a new plan",
            ));
        }
        let history = plan.input["history_tree_id"]
            .as_str()
            .ok_or_else(|| Error::new("PLAN_INVALID", "history missing"))?
            .to_string();
        self.require_grant(
            self.repository_for_history(&history)?,
            principal,
            zxs_contract::RepositoryAccess::Manage,
        )?;
        let result = match plan.kind.as_str() {
            "purge" => {
                let ids = plan.input["version_ids"]
                    .as_array()
                    .ok_or_else(|| Error::new("PLAN_INVALID", "versions missing"))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(str::to_string)
                            .ok_or_else(|| Error::new("PLAN_INVALID", "version id is not a string"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                self.check_purge(&history, &ids)?;
                self.mutate(
                    "content.purge",
                    principal,
                    operation,
                    json!({"plan_id":id,"digest":plan.digest}),
                    |_, state, _| {
                        for version in &ids {
                            state
                                .content_states
                                .insert(version.clone(), "PURGED".into());
                        }
                        let mut p = plan.clone();
                        p.state = "committed".into();
                        state.plans.insert(id.into(), p);
                        Ok(json!({"plan_id":id,"history_tree_id":history,"purged_versions":ids}))
                    },
                )?
            }
            "split" => {
                let cut = plan.input["cut_commit_id"]
                    .as_str()
                    .ok_or_else(|| Error::new("PLAN_INVALID", "cut missing"))?
                    .to_string();
                self.split_history(&history, &cut, principal, operation, &plan)?
            }
            _ => return Err(Error::new("PLAN_UNSUPPORTED", "unknown plan kind")),
        };
        self.cleanup_purged()?;
        Ok(result)
    }
    fn check_purge(&self, history: &str, versions: &[String]) -> Result<()> {
        let selected = versions.iter().collect::<BTreeSet<_>>();
        let reads = self
            .reads
            .lock()
            .map_err(|_| Error::new("INTERNAL_ERROR", "read lease lock poisoned"))?;
        for id in versions {
            let v = self
                .state
                .versions
                .get(id)
                .ok_or_else(|| Error::new("VERSION_NOT_FOUND", "selected version missing"))?;
            if v.history_tree_id != history {
                return Err(Error::new(
                    "HISTORY_MISMATCH",
                    "purge version belongs to another history",
                ));
            }
            if reads.get(id).copied().unwrap_or(0) > 0 {
                return Err(Error::new(
                    "CONTENT_PINNED",
                    "version has an active read lease",
                ));
            }
            for b in self.state.branches.values().filter(|b| b.state == "active") {
                if self
                    .state
                    .commits
                    .get(&b.head_commit_id)
                    .is_some_and(|c| c.entries.iter().any(|e| e.version_id == *id))
                {
                    return Err(Error::new(
                        "CONTENT_PINNED",
                        "version is used by an active branch HEAD",
                    ));
                }
            }
            for branch in self.state.directory_branches.values() {
                if branch
                    .entries
                    .iter()
                    .any(|entry| entry.kind == "file" && entry.version_id == *id)
                {
                    return Err(Error::new(
                        "CONTENT_PINNED",
                        "version is referenced by a retained directory branch HEAD",
                    ));
                }
            }
        }
        for w in self
            .workspaces
            .values()
            .filter(|w| matches!(w.state.as_str(), "open" | "sealed"))
        {
            if self
                .state
                .commits
                .get(&w.base_head)
                .is_some_and(|c| c.entries.iter().any(|e| selected.contains(&e.version_id)))
            {
                return Err(Error::new(
                    "CONTENT_PINNED",
                    "an open workspace needs a selected version",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn cleanup_purged(&self) -> Result<()> {
        self.require_no_commit_preparation()?;
        if !self
            .state
            .content_states
            .values()
            .any(|s| s == "PURGED" || s == "MIGRATED")
        {
            return Ok(());
        }
        // Embed the hashed state page and its publication closure in each local
        // tombstone, so an offline disk can distinguish intentional removal
        // from lost bytes without consulting another disk or SQLite.
        let mut published = None;
        for member in self.members() {
            if validate_seal(&self.config, &member).is_err() {
                continue;
            }
            let scanned = scan(&member)?;
            if let Some((record, _)) =
                scanned.records.iter().rev().find(|(r, _)| {
                    r.sequence == self.state.sequence && r.record_type == "PUBLICATION"
                })
            {
                let candidate: Candidate =
                    load_object(&member, "candidate", &record.candidate_hash)?;
                let publication: Publication =
                    load_object(&member, "publication", &record.object_hash)?;
                let seal: PublicationSeal =
                    read_json(&publication_seal_path(&member, &record.candidate_hash))?;
                verify_publication_seal(&candidate, &publication, &seal)?;
                let snapshot: Snapshot =
                    load_object(&member, "snapshot", &candidate.snapshot_hash)?;
                published = Some((member, candidate, publication, seal, snapshot));
                break;
            }
        }
        let (member, candidate, publication, seal, snapshot) = published.ok_or_else(|| {
            Error::new(
                "RECOVERY_REQUIRED",
                "published tombstone evidence is unavailable",
            )
        })?;
        for (id, state) in &self.state.content_states {
            if state != "PURGED" && state != "MIGRATED" {
                continue;
            }
            let Some(v) = self.state.versions.get(id) else {
                continue;
            };
            let Some(p) = self.state.placements.get(id) else {
                continue;
            };
            let mut inclusion = None;
            for hash in snapshot.pages.get("content_states").into_iter().flatten() {
                let page: serde_json::Map<String, serde_json::Value> =
                    load_object(&member, "state-page", hash)?;
                if page.get(id).and_then(|v| v.as_str()) == Some(state.as_str()) {
                    inclusion = Some(page);
                    break;
                }
            }
            let content_state_page = inclusion.ok_or_else(|| {
                Error::new(
                    "TOMBSTONE_INVALID",
                    "published state page does not contain the tombstone",
                )
            })?;
            let tombstone = ContentTombstone {
                version_id: id.clone(),
                state: state.clone(),
                candidate: candidate.clone(),
                publication: publication.clone(),
                publication_seal: seal.clone(),
                snapshot: snapshot.clone(),
                content_state_page,
            };
            for slot in &p.route_slot_ids {
                let d = self.disk(slot)?;
                if validate_seal(&self.config, d).is_err() {
                    continue;
                }
                let dir = version_dir(d, &v.history_tree_id, id);
                if !dir.exists() {
                    continue;
                }
                write_json(&dir.join("purged.json"), &tombstone, true)?;
                let content = crate::container_storage::content_path(&dir, v)?;
                if content.exists() {
                    fs::remove_file(&content)?;
                    sync_dir(content.parent().expect("content parent"))?;
                }
            }
        }
        Ok(())
    }
    fn split_history(
        &mut self,
        history: &str,
        cut: &str,
        principal: &str,
        operation: &str,
        plan: &Plan,
    ) -> Result<OperationResult> {
        self.check_directory_split(history)?;
        self.mutate(
            "history.split", principal, operation,
            json!({"plan_id":plan.plan_id,"digest":plan.digest}),
            |engine, state, pending| {
                let original = state.histories.get(history).cloned()
                    .ok_or_else(|| Error::new("HISTORY_NOT_FOUND", "source history missing"))?;
                let original_repo = state.repositories.get(&original.repository_id).cloned()
                    .ok_or_else(|| Error::new("REPOSITORY_NOT_FOUND", "source repository missing"))?;
                let cut_commit = state.commits.get(cut)
                    .filter(|c| c.history_tree_id == history).cloned()
                    .ok_or_else(|| Error::new("COMMIT_NOT_FOUND", "cut commit missing"))?;
                if cut_commit.commit_id != cut {
                    return Err(Error::new("STATE_INVALID", "split cut commit identity differs from its key"));
                }
                if original.root_commit_id == cut || cut_commit.parent_commit_ids.len() != 1 {
                    return Err(Error::new("SPLIT_ROOT_FORBIDDEN", "split must preserve the original root and prefix"));
                }
                let prefix_head = cut_commit.parent_commit_ids[0].clone();
                let prefix = state.commits.get(&prefix_head)
                    .filter(|commit| commit.commit_id == prefix_head && commit.history_tree_id == history)
                    .ok_or_else(|| Error::new("STATE_INVALID", "split prefix commit missing from source history"))?;
                let prefix_pending = prefix.pending_file_versions.clone();
                let selected = crate::business::descendant_commits(
                    state.commits.values().filter(|commit| commit.history_tree_id == history), cut,
                );
                if selected.contains(&prefix_head) {
                    return Err(Error::new("STATE_INVALID", "split prefix is part of a cyclic selected history"));
                }
                if engine.workspaces.values().any(|w| matches!(w.state.as_str(), "open" | "sealed") && state.branches
                    .get(&w.branch_id).is_some_and(|b| b.history_tree_id == history)) {
                    return Err(Error::new("WORKSPACE_BUSY", "split input has an open workspace"));
                }
                let mut commits = selected.iter().map(|id| state.commits.get(id)
                    .filter(|commit| commit.commit_id == *id && commit.history_tree_id == history).cloned()
                    .ok_or_else(|| Error::new("STATE_INVALID", "split commit identity missing from source history")))
                    .collect::<Result<Vec<_>>>()?;
                commits.sort_by_key(|c| c.sequence);
                let selected_versions = commits.iter().flat_map(|c| &c.entries)
                    .filter(|e| e.kind == "file").map(|e| e.version_id.clone())
                    .collect::<BTreeSet<_>>();
                let reads = engine.reads.lock()
                    .map_err(|_| Error::new("INTERNAL_ERROR", "read lease lock poisoned"))?;
                if selected_versions.iter().any(|id| reads.get(id).copied().unwrap_or(0) != 0) {
                    return Err(Error::new("CONTENT_PINNED", "split input has an active read lease"));
                }
                drop(reads);
                let history_id = new_id("history");
                let repository_id = new_id("repo");
                let commit_map = commits.iter().map(|c| (c.commit_id.clone(), new_id("commit")))
                    .collect::<BTreeMap<_,_>>();
                // Map identities once for the whole history, rather than once per snapshot.
                let entry_map = commits.iter().flat_map(|c| &c.entries)
                    .map(|e| e.entry_id.clone()).collect::<BTreeSet<_>>().into_iter()
                    .map(|id| (id, new_id("entry"))).collect::<BTreeMap<_,_>>();
                let file_map = commits.iter().flat_map(|c| &c.entries).filter(|e| e.kind == "file")
                    .map(|e| e.file_id.clone()).collect::<BTreeSet<_>>().into_iter()
                    .map(|id| (id, new_id("file"))).collect::<BTreeMap<_,_>>();
                let source_branches = state.branches.iter().filter(|(_, branch)| branch.history_tree_id == history)
                    .map(|(id, branch)| {
                        if id != &branch.branch_id {
                            return Err(Error::new("STATE_INVALID", "split source branch identity differs from its key"));
                        }
                        Ok(branch.clone())
                    }).collect::<Result<Vec<_>>>()?;
                let moved = source_branches.iter().filter(|b| selected.contains(&b.head_commit_id))
                    .map(|b| b.branch_id.clone()).collect::<BTreeSet<_>>();
                let links = state.merge_links.iter().filter(|(_, link)| link.history_tree_id == history)
                    .map(|(id, link)| {
                        if id != &link.merge_link_id {
                            return Err(Error::new("STATE_INVALID", "split merge link identity differs from its key"));
                        }
                        Ok(link.clone())
                    }).collect::<Result<Vec<_>>>()?;
                let mut branch_map = moved.iter().map(|id| (id.clone(), new_id("br")))
                    .collect::<BTreeMap<_,_>>();
                // A link may name a branch whose current HEAD is outside the cut.
                // Preserve that branch's local identity with an archived endpoint stub.
                for link in &links {
                    for (branch, commit) in [(&link.source_branch_id, &link.source_commit_id),
                        (&link.target_branch_id, &link.target_commit_id)] {
                        // Even the endpoint left in the prefix becomes a local
                        // audit boundary. Both references must remain valid;
                        // legacy links need not match the branch's current HEAD.
                        if !state.branches.get(branch).is_some_and(|b|
                            b.branch_id == *branch && b.history_tree_id == history)
                        {
                            return Err(Error::new("STATE_INVALID", "merge link branch missing from source history"));
                        }
                        if !state.commits.get(commit).is_some_and(|c|
                            c.commit_id == *commit && c.history_tree_id == history)
                        {
                            return Err(Error::new("STATE_INVALID", "merge link commit missing from source history"));
                        }
                        if selected.contains(commit) {
                            branch_map.entry(branch.clone()).or_insert_with(|| new_id("br"));
                        }
                    }
                }
                let mut version_map = BTreeMap::new();
                for c in &commits {
                    for entry in c.entries.iter().filter(|e| e.kind == "file") {
                        let old = state.versions.get(&entry.version_id)
                            .filter(|version| version.version_id == entry.version_id
                                && version.file_id == entry.file_id && version.history_tree_id == history)
                            .ok_or_else(|| Error::new("STATE_INVALID", "split version identity differs from its entry"))?;
                        if version_map.contains_key(&entry.version_id) { continue; }
                        let old = old.clone();
                        let placement = state.placements.get(&entry.version_id).cloned()
                            .ok_or_else(|| Error::new("PLACEMENT_NOT_FOUND", "split placement missing"))?;
                        let file_id = file_map.get(&old.file_id)
                            .ok_or_else(|| Error::new("STATE_INVALID", "entry and version file identity differ"))?;
                        let new = if state.content_states.get(&old.version_id).is_some_and(|s| s == "PURGED") {
                            let mut new = old.clone();
                            new.version_id = new_id("ver");
                            new.file_id = file_id.clone();
                            new.history_tree_id = history_id.clone();
                            new.source_version_ids.clear();
                            new.created_as_full_checkpoint = true;
                            let mut placement = placement;
                            placement.version_id = new.version_id.clone();
                            state.placements.insert(new.version_id.clone(), placement);
                            state.content_states.insert(new.version_id.clone(), "PURGED".into());
                            state.versions.insert(new.version_id.clone(), new.clone());
                            new
                        } else {
                            let (path, _) = engine.valid_replica(&old.version_id)?;
                            engine.build_version(state, pending, &history_id, file_id, &entry.path,
                                &path, &placement.primary_slot_id, placement.target_copies, Vec::new(), true)?
                        };
                        version_map.insert(old.version_id, new.version_id);
                    }
                }
                for (old,new) in &file_map {
                    if let Some(name)=state.file_names.get(old).cloned(){state.file_names.insert(new.clone(),name);}
                    if let Some(location)=state.file_locations.get(old).cloned(){state.file_locations.insert(new.clone(),location);}
                    if let Some(note)=state.file_notes.get(old).cloned(){state.file_notes.insert(new.clone(),note);}
                    if let Some(paths)=state.file_backup_paths.get(old).cloned(){state.file_backup_paths.insert(new.clone(),paths);}
                }
                for (old,new) in &version_map {
                    if let Some(note)=state.version_notes.get(old).cloned(){state.version_notes.insert(new.clone(),note);}
                }
                for commit in &commits {
                    let mut new = commit.clone();
                    new.commit_id = commit_map[&commit.commit_id].clone();
                    new.history_tree_id = history_id.clone();
                    new.parent_commit_ids = if commit.commit_id == cut { Vec::new() } else {
                        commit.parent_commit_ids.iter().map(|p| commit_map.get(p).cloned()
                            .ok_or_else(|| Error::new("SPLIT_INVALID", "selected closure has an external parent")))
                            .collect::<Result<_>>()?
                    };
                    new.tree_id = new_id("tree");
                    if commit.commit_id == cut { new.kind = "split_root".into(); }
                    new.provenance = Some(commit.commit_id.clone());
                    for entry in &mut new.entries {
                        entry.entry_id = entry_map[&entry.entry_id].clone();
                        if entry.kind == "file" {
                            entry.file_id = file_map[&entry.file_id].clone();
                            entry.version_id = version_map[&entry.version_id].clone();
                        }
                    }
                    new.pending_file_versions = remap_pending_versions(
                        &commit.pending_file_versions, &file_map, &version_map,
                    )?;
                    sorted(&mut new.entries);
                    state.commits.insert(new.commit_id.clone(), new);
                    // Migrated history must not appear as empty-file commits in the old graph.
                    state.commits.remove(&commit.commit_id);
                }
                for branch in &source_branches {
                    let Some(new_id) = branch_map.get(&branch.branch_id) else { continue; };
                    let mut new = branch.clone();
                    new.branch_id = new_id.clone();
                    new.history_tree_id = history_id.clone();
                    new.file_id = branch.file_id.as_ref().map(|file|file_map.get(file).cloned().ok_or_else(||Error::new("SPLIT_INVALID","file branch identity is outside the selected history"))).transpose()?;
                    new.generation = 1;
                    new.fork_commit_id = commit_map.get(&branch.fork_commit_id)
                        .cloned().unwrap_or_else(|| commit_map[cut].clone());
                    if moved.contains(&branch.branch_id) {
                        new.pending_file_versions = remap_pending_versions(
                            &branch.pending_file_versions, &file_map, &version_map,
                        )?;
                        new.head_commit_id = commit_map[&branch.head_commit_id].clone();
                        let old = state.branches.get_mut(&branch.branch_id)
                            .ok_or_else(|| Error::new("STATE_INVALID", "split source branch identity missing"))?;
                        old.pending_file_versions = prefix_pending.clone();
                        old.head_commit_id = prefix_head.clone();
                        if selected.contains(&old.fork_commit_id) { old.fork_commit_id = prefix_head.clone(); }
                        old.state = "migrated".into();
                        old.generation = old.generation.checked_add(1)
                            .ok_or_else(|| Error::new("RESOURCE_LIMIT", "branch generation exhausted"))?;
                    } else {
                        let endpoint = links.iter().find_map(|l| {
                            if l.source_branch_id == branch.branch_id && selected.contains(&l.source_commit_id) {
                                Some(&l.source_commit_id)
                            } else if l.target_branch_id == branch.branch_id && selected.contains(&l.target_commit_id) {
                                Some(&l.target_commit_id)
                            } else { None }
                        }).ok_or_else(|| Error::new("STATE_INVALID", "split branch endpoint missing"))?;
                        new.head_commit_id = commit_map[endpoint].clone();
                        new.state = "archived".into();
                        // The audit endpoint keeps its own snapshot state,
                        // never the original branch's later reservation map.
                        new.pending_file_versions = state.commits[&new.head_commit_id]
                            .pending_file_versions.clone();
                    }
                    state.branches.insert(new.branch_id.clone(), new);
                }
                for (file,selected_branch) in state.file_selections.clone() {
                    if moved.contains(&selected_branch)
                        && let (Some(new_file),Some(new_branch))=(file_map.get(&file),branch_map.get(&selected_branch)) {
                            state.file_selections.insert(new_file.clone(),new_branch.clone());
                            state.file_selections.remove(&file);
                        }
                }
                let new_main = original.main_branch_id.as_ref().filter(|id| moved.contains(*id))
                    .and_then(|id| branch_map.get(id)).cloned()
                    .or_else(|| moved.iter().find_map(|id| branch_map.get(id).cloned()))
                    .unwrap_or_else(|| {
                        let id = new_id("br");
                        let pending_file_versions = state.commits[&commit_map[cut]]
                            .pending_file_versions.clone();
                        state.branches.insert(id.clone(), Branch {
                            branch_id: id.clone(), history_tree_id: history_id.clone(), name: "main".into(),
                            head_commit_id: commit_map[cut].clone(), fork_commit_id: commit_map[cut].clone(),
                            branch_kind: "full".into(), file_id: None, file_node_id:None, pending_file_versions, generation: 1, state: "active".into(),
                        });
                        id
                    });
                state.histories.insert(history_id.clone(), HistoryTree {
                    history_tree_id: history_id.clone(), repository_id: repository_id.clone(),
                    root_commit_id: commit_map[cut].clone(), main_branch_id: Some(new_main.clone()),
                    generation: 1, state: "active".into(), provenance: Some(format!("{history}:{cut}")),
                });
                let old = state.histories.get_mut(history).expect("source history");
                old.generation = old.generation.checked_add(1)
                    .ok_or_else(|| Error::new("RESOURCE_LIMIT", "history generation exhausted"))?;
                let main_moved = old.main_branch_id.as_ref().is_some_and(|id| moved.contains(id));
                if main_moved { old.main_branch_id = None; old.state = "archived".into(); }
                if main_moved {
                    state.repositories.get_mut(&original.repository_id).expect("source repository").smb_enabled = false;
                }
                let mut new_repo = original_repo.clone();
                new_repo.repository_id = repository_id.clone();
                new_repo.history_tree_id = history_id.clone();
                new_repo.main_branch_id = new_main.clone();
                new_repo.account_scope_repository_id = Some(original_repo.account_scope_repository_id
                    .clone().unwrap_or_else(|| original_repo.repository_id.clone()));
                new_repo.smb_enabled = false;
                new_repo.file_manager_root = false;
                loop {
                    new_repo.name = format!("拆分-{}", &new_id("repo")[5..13]);
                    if !state.repositories.values().any(|r| zxs_naming::lookup_key(&r.name) == zxs_naming::lookup_key(&new_repo.name)) { break; }
                }
                zxs_naming::validate_name(&new_repo.name)?;
                state.repositories.insert(repository_id.clone(), new_repo);
                let rules = state.placement_rules.values().filter(|p| p.repository_id == original.repository_id).cloned().collect::<Vec<_>>();
                for mut rule in rules {
                    rule.rule_id = new_id("rule"); rule.repository_id = repository_id.clone(); rule.generation = 1;
                    state.placement_rules.insert(rule.rule_id.clone(), rule);
                }
                let policies = state.protection_policies.values().filter(|p| p.repository_id == original.repository_id).cloned().collect::<Vec<_>>();
                let mut policy_map=BTreeMap::new();
                for mut policy in policies {
                    let old=policy.policy_id.clone();
                    if let Some(file)=&policy.scope_file_id {let Some(mapped)=file_map.get(file) else {continue;};policy.scope_file_id=Some(mapped.clone());}
                    if let Some(directory)=&policy.scope_directory_id {policy.scope_directory_id=Some(if directory==&original.repository_id {repository_id.clone()}else{let Some(mapped)=entry_map.get(directory) else{continue;};mapped.clone()});}
                    policy.policy_id = new_id("policy"); policy.repository_id = repository_id.clone(); policy.generation = 1;
                    policy_map.insert(old,policy.policy_id.clone());
                    state.protection_policies.insert(policy.policy_id.clone(), policy);
                }
                for (old,new) in &file_map {if let Some(policy)=state.file_backup_policies.get(old).and_then(|policy|policy_map.get(policy)).cloned(){state.file_backup_policies.insert(new.clone(),policy);}}
                // The cut edge and each crossing merge edge are explicit audit boundaries.
                insert_boundary(state, history, &prefix_head, None, "split_out", &history_id, &commit_map[cut], None);
                insert_boundary(state, &history_id, &commit_map[cut], None, "split_origin", history, &prefix_head, None);
                for link in links {
                    let source = selected.contains(&link.source_commit_id);
                    let target = selected.contains(&link.target_commit_id);
                    if !source && !target { continue; }
                    state.merge_links.remove(&link.merge_link_id);
                    if source && target {
                        let mut new = link.clone();
                        new.merge_link_id = new_id("link"); new.history_tree_id = history_id.clone();
                        new.source_commit_id = commit_map[&link.source_commit_id].clone();
                        new.target_commit_id = commit_map[&link.target_commit_id].clone();
                        new.source_branch_id = branch_map[&link.source_branch_id].clone();
                        new.target_branch_id = branch_map[&link.target_branch_id].clone();
                        state.merge_links.insert(new.merge_link_id.clone(), new);
                    } else {
                        let (local_commit, local_branch, external_commit, moved_branch, relation) = if source {
                            (&link.target_commit_id, &link.target_branch_id, &link.source_commit_id, &link.source_branch_id, "merge_source_migrated")
                        } else {
                            (&link.source_commit_id, &link.source_branch_id, &link.target_commit_id, &link.target_branch_id, "merge_target_migrated")
                        };
                        insert_boundary(state, history, local_commit, Some(local_branch.clone()), relation,
                            &history_id, &commit_map[external_commit], Some(link.merge_link_id.clone()));
                        insert_boundary(state, &history_id, &commit_map[external_commit], Some(branch_map[moved_branch].clone()),
                            "merge_external", history, local_commit, Some(link.merge_link_id));
                    }
                }
                // Existing audit boundaries whose local endpoint moved follow that endpoint.
                for boundary in state.history_boundaries.values_mut() {
                    if boundary.history_tree_id == history && selected.contains(&boundary.local_commit_id) {
                        boundary.history_tree_id = history_id.clone();
                        boundary.local_commit_id = commit_map[&boundary.local_commit_id].clone();
                        boundary.local_branch_id = boundary.local_branch_id.as_ref().and_then(|b| branch_map.get(b)).cloned();
                    }
                }
                for old_version in version_map.keys() {
                    let used = state.commits.values().filter(|c| c.history_tree_id == history)
                        .any(|c| c.entries.iter().any(|e| &e.version_id == old_version));
                    if !used && state.content_states.get(old_version).is_none_or(|s| s != "PURGED") {
                        state.content_states.insert(old_version.clone(), "MIGRATED".into());
                    }
                }
                let mut completed = plan.clone(); completed.state = "committed".into();
                state.plans.insert(plan.plan_id.clone(), completed);
                Ok(json!({"plan_id":plan.plan_id,"repository_id":repository_id,"history_tree_id":history_id,
                    "source_history_tree_id":history,"commit_id":commit_map[cut],"branch_id":new_main,
                    "commit_mapping":commit_map,"branch_mapping":branch_map,"version_mapping":version_map,
                    "entry_mapping":entry_map,"file_mapping":file_map,"smb_rebind_required":true,"created_at":now().to_string()}))
            },
        )
    }
}

fn remap_pending_versions(
    pending: &BTreeMap<String, String>,
    files: &BTreeMap<String, String>,
    versions: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    pending
        .iter()
        .map(|(file, version)| {
            Ok((
                files.get(file).cloned().ok_or_else(|| {
                    Error::new(
                        "SPLIT_INVALID",
                        "pending file is outside the selected history",
                    )
                })?,
                versions.get(version).cloned().ok_or_else(|| {
                    Error::new(
                        "SPLIT_INVALID",
                        "pending source version is outside the selected history",
                    )
                })?,
            ))
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "Boundary construction records distinct provenance fields; tracked request-type migration in .github/QUALITY.md"
)]
fn insert_boundary(
    state: &mut State,
    history: &str,
    commit: &str,
    branch: Option<String>,
    relation: &str,
    external_history: &str,
    external_commit: &str,
    source_link: Option<String>,
) {
    let id = new_id("boundary");
    state.history_boundaries.insert(
        id.clone(),
        HistoryBoundary {
            boundary_id: id,
            history_tree_id: history.into(),
            local_commit_id: commit.into(),
            local_branch_id: branch,
            relation: relation.into(),
            external_history_tree_id: external_history.into(),
            external_commit_id: external_commit.into(),
            source_merge_link_id: source_link,
        },
    );
}

#[cfg(test)]
mod split_tests {
    use super::*;
    use std::io::Read;

    fn fixture() -> (tempfile::TempDir, Engine, Repository) {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::development([
            temp.path().join("p"),
            temp.path().join("b"),
            temp.path().join("c"),
        ]);
        let mut engine = Engine::initialize(&temp.path().join("state"), config).unwrap();
        let repo = engine
            .create_repository("项目", "P01", 3, "alice", &new_id("op"))
            .unwrap();
        (temp, engine, repo)
    }
    fn save(engine: &mut Engine, branch: &str, bytes: &[u8]) -> String {
        let workspace = engine.create_workspace(branch, "alice").unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "资料/文件.txt", bytes)
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
    fn assert_graph_closed(engine: &Engine, history: &str) {
        let graph = engine.graph(history).unwrap();
        let commits = graph
            .commits
            .iter()
            .map(|c| c.commit_id.as_str())
            .collect::<BTreeSet<_>>();
        let branches = graph
            .branches
            .iter()
            .map(|b| b.branch_id.as_str())
            .collect::<BTreeSet<_>>();
        for commit in &graph.commits {
            for parent in &commit.parent_commit_ids {
                assert!(commits.contains(parent.as_str()));
            }
        }
        for branch in &graph.branches {
            assert!(commits.contains(branch.head_commit_id.as_str()));
            assert!(commits.contains(branch.fork_commit_id.as_str()));
        }
        for link in &graph.merge_links {
            assert!(commits.contains(link.source_commit_id.as_str()));
            assert!(commits.contains(link.target_commit_id.as_str()));
            assert!(branches.contains(link.source_branch_id.as_str()));
            assert!(branches.contains(link.target_branch_id.as_str()));
        }
        for boundary in &graph.boundary_marks {
            assert!(commits.contains(boundary.local_commit_id.as_str()));
            if let Some(branch) = &boundary.local_branch_id {
                assert!(branches.contains(branch.as_str()));
            }
            assert!(!boundary.external_commit_id.is_empty());
        }
    }

    #[test]
    fn all_named_directory_heads_pin_sources_until_every_head_advances() {
        let (_temporary, mut engine, repo) = fixture();
        let source_commit = save(&mut engine, &repo.main_branch_id, b"frozen source");
        let source = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"current main");
        let shared = engine
            .create_file_branch(
                &source.file_id,
                &source.version_id,
                "shared",
                "alice",
                &new_id("op"),
            )
            .unwrap()
            .branch_id
            .unwrap();
        save(&mut engine, &shared, b"shared own content");
        // Only the frozen composition still needs the original body; both
        // ordinary branch HEADs have moved to their own newer versions.
        engine
            .create_purge_plan(
                &repo.history_tree_id,
                vec![source.version_id.clone()],
                "alice",
            )
            .unwrap();
        let record = new_id("dbr");
        let parent_record = new_id("dbr");
        let parent_tree = new_id("dtree");
        engine
            .mutate(
                "fixture.directory_composition",
                "alice",
                &new_id("op"),
                json!({}),
                |_, state, _| {
                    let parent = DirectoryBranch {
                        directory_branch_id: parent_record.clone(),
                        repository_id: repo.repository_id.clone(),
                        directory_id: repo.repository_id.clone(),
                        path: String::new(),
                        name: "frozen".into(),
                        parent_branch_id: None,
                        head_tree_id: parent_tree.clone(),
                        commit_id: source_commit.clone(),
                        entries: vec![source.clone()],
                        file_branches: BTreeMap::from([(source.file_id.clone(), shared.clone())]),
                        file_contexts: BTreeMap::new(),
                        inherited_files: BTreeMap::new(),
                        directory_refs: BTreeMap::new(),
                        materialized: true,
                        sequence: state.sequence,
                    };
                    state.directory_trees.insert(
                        parent_tree.clone(),
                        DirectoryTree {
                            tree_id: parent_tree.clone(),
                            directory_branch_id: parent_record.clone(),
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
                    let mut pending = parent.clone();
                    pending.directory_branch_id = record.clone();
                    pending.name = "pending".into();
                    pending.parent_branch_id = Some(parent_record.clone());
                    pending.materialized = false;
                    state
                        .directory_branches
                        .insert(parent_record.clone(), parent);
                    state.directory_branches.insert(record.clone(), pending);
                    Ok(json!({}))
                },
            )
            .unwrap();
        assert_eq!(
            engine
                .create_purge_plan(
                    &repo.history_tree_id,
                    vec![source.version_id.clone()],
                    "alice"
                )
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        assert_eq!(
            engine
                .create_split_plan(&repo.history_tree_id, &source_commit, "alice")
                .unwrap_err()
                .code(),
            "DIRECTORY_HISTORY_SPLIT_UNSUPPORTED"
        );
        engine
            .mutate(
                "fixture.directory_complete",
                "alice",
                &new_id("op"),
                json!({}),
                |_, state, _| {
                    let branch = state.directory_branches.get_mut(&record).unwrap();
                    let tree = new_id("dtree");
                    state.directory_trees.insert(
                        tree.clone(),
                        DirectoryTree {
                            tree_id: tree.clone(),
                            directory_branch_id: record.clone(),
                            parent_tree_id: Some(branch.head_tree_id.clone()),
                            commit_id: branch.commit_id.clone(),
                            path: branch.path.clone(),
                            entries: branch.entries.clone(),
                            file_branches: branch.file_branches.clone(),
                            file_contexts: branch.file_contexts.clone(),
                            inherited_files: branch.inherited_files.clone(),
                            directory_refs: branch.directory_refs.clone(),
                            sequence: state.sequence,
                        },
                    );
                    branch.head_tree_id = tree;
                    branch.materialized = true;
                    branch.sequence = state.sequence;
                    Ok(json!({}))
                },
            )
            .unwrap();
        assert_eq!(
            engine
                .create_purge_plan(
                    &repo.history_tree_id,
                    vec![source.version_id.clone()],
                    "alice"
                )
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        engine
            .mutate(
                "fixture.directory_select",
                "alice",
                &new_id("op"),
                json!({}),
                |_, state, _| {
                    state
                        .directory_selections
                        .insert(repo.repository_id.clone(), record.clone());
                    Ok(json!({}))
                },
            )
            .unwrap();
        assert_eq!(
            engine
                .create_purge_plan(
                    &repo.history_tree_id,
                    vec![source.version_id.clone()],
                    "alice"
                )
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        engine
            .mutate(
                "fixture.directory_unselect",
                "alice",
                &new_id("op"),
                json!({}),
                |_, state, _| {
                    state.directory_selections.remove(&repo.repository_id);
                    Ok(json!({}))
                },
            )
            .unwrap();
        assert_eq!(
            engine
                .create_purge_plan(
                    &repo.history_tree_id,
                    vec![source.version_id.clone()],
                    "alice"
                )
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        // A branch remains a readable snapshot when it is not selected. Only
        // advancing every retained named HEAD releases its former content.
        let latest = engine.state.commits[&engine.state.branches[&shared].head_commit_id].clone();
        engine
            .mutate(
                "fixture.directory_advance",
                "alice",
                &new_id("op"),
                json!({}),
                |_, state, _| {
                    for id in [&parent_record, &record] {
                        let branch = state.directory_branches.get_mut(id).unwrap();
                        let tree = new_id("dtree");
                        state.directory_trees.insert(
                            tree.clone(),
                            DirectoryTree {
                                tree_id: tree.clone(),
                                directory_branch_id: id.clone(),
                                parent_tree_id: Some(branch.head_tree_id.clone()),
                                commit_id: latest.commit_id.clone(),
                                path: String::new(),
                                entries: latest.entries.clone(),
                                file_branches: branch.file_branches.clone(),
                                file_contexts: branch.file_contexts.clone(),
                                inherited_files: BTreeMap::new(),
                                directory_refs: BTreeMap::new(),
                                sequence: state.sequence,
                            },
                        );
                        branch.head_tree_id = tree;
                        branch.commit_id = latest.commit_id.clone();
                        branch.entries = latest.entries.clone();
                        branch.materialized = true;
                        branch.sequence = state.sequence;
                    }
                    Ok(json!({}))
                },
            )
            .unwrap();
        let plan = engine
            .create_purge_plan(
                &repo.history_tree_id,
                vec![source.version_id.clone()],
                "alice",
            )
            .unwrap();
        engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            engine
                .open_content(&source.version_id)
                .err()
                .unwrap()
                .code(),
            "CONTENT_PURGED"
        );
        assert_eq!(
            engine.state.directory_trees[&parent_tree].entries[0].version_id,
            source.version_id
        );
        let own = engine
            .list_tree(&shared)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        assert!(engine.open_content(&own.version_id).is_ok());
    }

    #[test]
    fn inherited_branches_and_their_workspaces_pin_sources_until_first_own_content() {
        let (_temporary, mut engine, repo) = fixture();
        let source = save(&mut engine, &repo.main_branch_id, b"old source");
        let old = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        let pending = engine
            .create_branch(&source, "pending", "alice", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&pending.branch_id, "alice")
            .unwrap();
        // Inheritance is frozen at registration; main advancing does not change
        // this branch's visible source or release its active read dependency.
        save(&mut engine, &repo.main_branch_id, b"current main");
        assert_eq!(
            engine
                .create_purge_plan(&repo.history_tree_id, vec![old.version_id.clone()], "alice")
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        let inherited = engine
            .list_tree(&pending.branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        assert_eq!(inherited.version_id, old.version_id);
        assert_eq!(inherited.size, old.size);
        engine
            .stage_bytes(&workspace.workspace_id, &old.path, b"first own body")
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "first own content releases inherited source".into(),
                request_digest: None,
            })
            .unwrap();
        assert!(
            engine.state.branches[&pending.branch_id]
                .pending_file_versions
                .is_empty()
        );
        let current = engine
            .list_tree(&pending.branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        assert_eq!(
            engine.state.versions[&current.version_id].source_version_ids,
            vec![old.version_id.clone()]
        );
        let plan = engine
            .create_purge_plan(&repo.history_tree_id, vec![old.version_id.clone()], "alice")
            .unwrap();
        engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            engine.open_content(&old.version_id).err().unwrap().code(),
            "CONTENT_PURGED"
        );
        assert!(engine.open_content(&current.version_id).is_ok());
    }

    #[test]
    fn selected_inherited_file_pins_source_through_directory_main_until_materialized() {
        let (_temporary, mut engine, repo) = fixture();
        save(&mut engine, &repo.main_branch_id, b"old source");
        let old = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"owned main body");
        let created = engine
            .create_file_branch(
                &old.file_id,
                &old.version_id,
                "pending",
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let branch = created.branch_id.unwrap();
        engine
            .select_file_branch(&old.file_id, &branch, "alice", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "alice")
            .unwrap();
        assert_eq!(
            engine
                .create_purge_plan(&repo.history_tree_id, vec![old.version_id.clone()], "alice")
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        engine
            .stage_bytes(
                &workspace.workspace_id,
                &old.path,
                b"first selected branch bytes",
            )
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "activate selected pending".into(),
                request_digest: None,
            })
            .unwrap();
        assert!(
            engine.state.branches[&branch]
                .pending_file_versions
                .is_empty()
        );
        let current = engine
            .list_tree(&branch)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        assert_eq!(
            engine.state.versions[&current.version_id].source_version_ids,
            vec![old.version_id.clone()]
        );
        let plan = engine
            .create_purge_plan(&repo.history_tree_id, vec![old.version_id.clone()], "alice")
            .unwrap();
        engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        assert_eq!(
            engine.open_content(&old.version_id).err().unwrap().code(),
            "CONTENT_PURGED"
        );
        assert!(engine.open_content(&current.version_id).is_ok());
    }

    #[test]
    fn split_remaps_inherited_references_without_creating_placeholder_files() {
        let (_temporary, mut engine, repo) = fixture();
        let cut = save(&mut engine, &repo.main_branch_id, b"first");
        let original = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        let pending = engine
            .create_branch(&cut, "pending", "alice", &new_id("op"))
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"second");
        let plan = engine
            .create_split_plan(&repo.history_tree_id, &cut, "alice")
            .unwrap();
        let result = engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        let history = result.history_tree_id.unwrap();
        let moved = engine
            .state
            .branches
            .values()
            .find(|branch| branch.history_tree_id == history && branch.name == "pending")
            .unwrap();
        assert_eq!(moved.pending_file_versions.len(), 1);
        let (file, source) = moved.pending_file_versions.first_key_value().unwrap();
        assert_ne!(file, &original.file_id);
        assert_ne!(source, &original.version_id);
        assert_eq!(engine.state.versions[source].file_id, *file);
        assert_eq!(engine.state.versions[source].history_tree_id, history);
        assert!(
            engine.state.commits[&moved.head_commit_id]
                .entries
                .iter()
                .any(|entry| &entry.file_id == file && &entry.version_id == source)
        );
        assert_eq!(
            engine.state.commits[&moved.head_commit_id].pending_file_versions,
            moved.pending_file_versions
        );
        assert!(
            engine.state.branches[&pending.branch_id]
                .pending_file_versions
                .is_empty()
        );
        for disk in &engine.config.disks {
            assert!(
                !disk
                    .root
                    .join("histories")
                    .join(&repo.history_tree_id)
                    .join("file-containers")
                    .join(&original.file_id)
                    .join(&original.name)
                    .join("pending-branches")
                    .join(format!("{}.pending", pending.branch_id))
                    .exists()
            );
            assert!(
                !disk
                    .root
                    .join("histories")
                    .join(&history)
                    .join("file-containers")
                    .join(file)
                    .join(&original.name)
                    .join("pending-branches")
                    .exists()
            );
            let cache = disk
                .root
                .join("histories")
                .join(&history)
                .join("file-containers")
                .join(file)
                .join("namespace")
                .join(&engine.state.file_locations[file])
                .join("container.json");
            assert!(!cache.parent().unwrap().join("pending-branches").exists());
            let metadata: serde_json::Value = read_json(&cache).unwrap();
            assert_eq!(
                metadata["inherited_branches"][0]["branch_id"],
                moved.branch_id
            );
            assert_eq!(
                metadata["inherited_branches"][0]["source_version_id"],
                *source
            );
            assert_eq!(metadata["inherited_branches"][0]["version_id"], *source);
            assert!(metadata["inherited_branches"][0]["own_version_id"].is_null());
        }
        assert_graph_closed(&engine, &history);
        assert_graph_closed(&engine, &repo.history_tree_id);
    }

    #[test]
    fn split_keeps_pending_prefix_snapshots_and_remaps_mixed_snapshot_reservations() {
        let (_temporary, mut engine, repo) = fixture();
        let source = save(&mut engine, &repo.main_branch_id, b"lineage source");
        let original = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|entry| entry.kind == "file")
            .unwrap();
        let pending = engine
            .create_branch(&source, "pending", "alice", &new_id("op"))
            .unwrap();
        let prefix = pending.head_commit_id.clone();
        let workspace = engine
            .create_workspace(&pending.branch_id, "alice")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "other.txt", b"only real new file")
            .unwrap();
        let cut = engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "alice".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "mixed snapshot".into(),
                request_digest: None,
            })
            .unwrap()
            .commit_id
            .unwrap();
        let plan = engine
            .create_split_plan(&repo.history_tree_id, &cut, "alice")
            .unwrap();
        let result = engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        let history = result.history_tree_id.unwrap();
        let moved = engine
            .state
            .branches
            .values()
            .find(|branch| branch.history_tree_id == history && branch.name == "pending")
            .unwrap();
        let remapped = &engine.state.commits[&moved.head_commit_id];
        assert_eq!(remapped.pending_file_versions.len(), 1);
        assert_eq!(remapped.pending_file_versions, moved.pending_file_versions);
        let (file, source) = remapped.pending_file_versions.first_key_value().unwrap();
        assert_ne!(file, &original.file_id);
        assert_ne!(source, &original.version_id);
        assert!(
            remapped
                .entries
                .iter()
                .any(|entry| &entry.file_id == file && &entry.version_id == source)
        );
        let old = &engine.state.branches[&pending.branch_id];
        assert_eq!(old.head_commit_id, prefix);
        assert_eq!(
            old.pending_file_versions[&original.file_id],
            original.version_id
        );
        assert_eq!(
            engine.state.commits[&prefix].pending_file_versions,
            old.pending_file_versions
        );
        assert_graph_closed(&engine, &history);
        assert_graph_closed(&engine, &repo.history_tree_id);
    }

    #[test]
    fn split_preserves_identity_per_file_and_independent_acl_without_rebinding() {
        let (temp, mut engine, repo) = fixture();
        let cut = save(&mut engine, &repo.main_branch_id, b"first");
        let original = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|e| e.kind == "file")
            .unwrap();
        save(&mut engine, &repo.main_branch_id, b"second");
        let plan = engine
            .create_split_plan(&repo.history_tree_id, &cut, "alice")
            .unwrap();
        let operation = new_id("op");
        let result = engine
            .commit_plan(&plan.plan_id, "alice", &operation)
            .unwrap();
        assert_eq!(
            engine
                .commit_plan(&plan.plan_id, "alice", &operation)
                .unwrap()
                .sequence,
            result.sequence
        );
        let history = result.history_tree_id.as_ref().unwrap();
        let new_repo_id = result.result["repository_id"].as_str().unwrap();
        let new_repo = &engine.state.repositories[new_repo_id];
        assert_ne!(new_repo_id, repo.repository_id);
        assert_eq!(
            new_repo.account_scope_repository_id.as_deref(),
            Some(repo.repository_id.as_str())
        );
        assert!(!new_repo.smb_enabled);
        assert!(!engine.state.repositories[&repo.repository_id].smb_enabled);
        assert_eq!(
            engine.state.repositories[&repo.repository_id].history_tree_id,
            repo.history_tree_id
        );
        assert_eq!(
            engine.state.branches[&repo.main_branch_id].state,
            "migrated"
        );
        let graph = engine.graph(history).unwrap();
        let files = graph
            .commits
            .iter()
            .flat_map(|c| &c.entries)
            .filter(|e| e.kind == "file")
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].entry_id, files[1].entry_id);
        assert_eq!(files[0].file_id, files[1].file_id);
        assert_ne!(files[0].file_id, original.file_id);
        assert_ne!(files[0].entry_id, original.entry_id);
        for file in files {
            let version = &engine.state.versions[&file.version_id];
            assert_eq!(version.file_id, file.file_id);
            assert!(version.source_version_ids.is_empty());
            let mut content = engine.open_content(&file.version_id).unwrap();
            let mut bytes = Vec::new();
            content.file.read_to_end(&mut bytes).unwrap();
            assert!([b"first".as_slice(), b"second".as_slice()].contains(&bytes.as_slice()));
        }
        assert_eq!(
            engine.graph(&repo.history_tree_id).unwrap().commits.len(),
            1
        );
        engine
            .set_grants(new_repo_id, Grants::owner("bob"), "alice", &new_id("op"))
            .unwrap();
        assert!(
            engine.state.repositories[&repo.repository_id]
                .grants
                .manage
                .contains("alice")
        );
        assert!(
            !engine.state.repositories[new_repo_id]
                .grants
                .manage
                .contains("alice")
        );
        assert_graph_closed(&engine, &repo.history_tree_id);
        assert_graph_closed(&engine, history);
        drop(engine);
        let reopened = Engine::open(&temp.path().join("state")).unwrap();
        assert_graph_closed(&reopened, &repo.history_tree_id);
        assert_graph_closed(&reopened, history);
    }

    #[test]
    fn split_merge_links_have_local_branch_stubs_and_explicit_crossing_boundaries() {
        let (_temp, mut engine, repo) = fixture();
        let root = engine.state.histories[&repo.history_tree_id]
            .root_commit_id
            .clone();
        let outside = engine
            .create_branch(&root, "前缀分支", "alice", &new_id("op"))
            .unwrap();
        let cut = save(&mut engine, &repo.main_branch_id, b"first");
        let last = save(&mut engine, &repo.main_branch_id, b"second");
        // Legacy metadata could name an endpoint outside the recorded branch.
        // New requests reject it; split still closes all references when such
        // old data is encountered instead of emitting an empty branch id.
        let legacy = MergeLink {
            merge_link_id: new_id("link"),
            history_tree_id: repo.history_tree_id.clone(),
            source_branch_id: outside.branch_id.clone(),
            source_commit_id: cut.clone(),
            target_branch_id: repo.main_branch_id.clone(),
            target_commit_id: last.clone(),
            author: "alice".into(),
            operation_id: new_id("op"),
        };
        engine
            .state
            .merge_links
            .insert(legacy.merge_link_id.clone(), legacy);
        engine
            .add_merge_link(
                &outside.branch_id,
                &outside.head_commit_id,
                &repo.main_branch_id,
                &cut,
                "alice",
                &new_id("op"),
            )
            .unwrap();
        let plan = engine
            .create_split_plan(&repo.history_tree_id, &cut, "alice")
            .unwrap();
        let result = engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
        let history = result.history_tree_id.unwrap();
        assert_graph_closed(&engine, &repo.history_tree_id);
        assert_graph_closed(&engine, &history);
        let graph = engine.graph(&history).unwrap();
        assert_eq!(graph.merge_links.len(), 1);
        assert!(graph.branches.iter().any(|b| b.state == "archived"));
        assert!(
            graph
                .boundary_marks
                .iter()
                .any(|b| b.source_merge_link_id.is_some())
        );
        assert!(
            engine
                .graph(&repo.history_tree_id)
                .unwrap()
                .merge_links
                .is_empty()
        );
    }

    #[test]
    fn split_rejects_root_and_rechecks_active_read_before_publication() {
        let (_temp, mut engine, repo) = fixture();
        let root = engine.state.histories[&repo.history_tree_id]
            .root_commit_id
            .clone();
        assert_eq!(
            engine
                .create_split_plan(&repo.history_tree_id, &root, "alice")
                .unwrap_err()
                .code(),
            "SPLIT_ROOT_FORBIDDEN"
        );
        let cut = save(&mut engine, &repo.main_branch_id, b"first");
        let plan = engine
            .create_split_plan(&repo.history_tree_id, &cut, "alice")
            .unwrap();
        let version = engine
            .list_tree(&repo.main_branch_id)
            .unwrap()
            .into_iter()
            .find(|e| e.kind == "file")
            .unwrap()
            .version_id;
        let read = engine.open_content(&version).unwrap();
        let generation = engine.state.business_generation;
        assert_eq!(
            engine
                .commit_plan(&plan.plan_id, "alice", &new_id("op"))
                .unwrap_err()
                .code(),
            "CONTENT_PINNED"
        );
        assert_eq!(engine.state.business_generation, generation);
        drop(read);
        engine
            .commit_plan(&plan.plan_id, "alice", &new_id("op"))
            .unwrap();
    }
}

#[cfg(test)]
#[path = "maintenance_corruption_tests.rs"]
mod corruption_tests;
