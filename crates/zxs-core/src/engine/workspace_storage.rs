//! Durable workspace manifests and restart reconciliation.
use super::Engine;
use crate::{disk::*, *};
use std::{fs, path::PathBuf};

impl Engine {
    pub(crate) fn persist_workspace(&self, w: &Workspace) -> Result<()> {
        let primary = self.workspace_data_directory(w)?;
        write_json(&primary.join("workspace.json"), w, true)?;
        write_json(
            &self
                .root
                .join("workspaces")
                .join(&w.workspace_id)
                .join("workspace.json"),
            w,
            true,
        )
    }
    pub(crate) fn workspace_data_directory(&self, w: &Workspace) -> Result<PathBuf> {
        let repo = self.repository_for_branch(&w.branch_id)?;
        let disk = self.disk(&repo.primary_slot_id)?;
        validate_seal(&self.config, disk)?;
        Ok(disk.root.join("staging/workspaces").join(&w.workspace_id))
    }
    pub(crate) fn load_workspaces(&mut self) -> Result<()> {
        let mut roots = vec![self.root.join("workspaces")];
        roots.extend(
            self.config
                .disks
                .iter()
                .filter(|d| d.level == 1)
                .map(|d| d.root.join("staging/workspaces")),
        );
        for root in roots {
            if !root.exists() {
                continue;
            }
            for e in fs::read_dir(root)? {
                let e = e?;
                if !e.file_type()?.is_dir() {
                    continue;
                }
                let p = e.path().join("workspace.json");
                if p.exists() {
                    let mut w: Workspace = read_json(&p)?;
                    validate_id(&w.workspace_id, "ws")?;
                    for f in w.files.values() {
                        let owned = self.config.disks.iter().any(|d| {
                            f.source_path.starts_with(
                                d.root.join("staging/workspaces").join(&w.workspace_id),
                            )
                        }) || f
                            .source_path
                            .starts_with(self.root.join("workspaces").join(&w.workspace_id));
                        if !owned {
                            return Err(Error::new(
                                "UNSAFE_PATH",
                                "workspace staging path escapes owned directory",
                            ));
                        }
                    }
                    let local = self
                        .root
                        .join("workspaces")
                        .join(&w.workspace_id)
                        .join("commit-request.json");
                    let data = e.path().join("commit-request.json");
                    let request = if local.exists() {
                        read_json::<CommitRequest>(&local).ok()
                    } else if data.exists() {
                        read_json::<CommitRequest>(&data).ok()
                    } else {
                        None
                    };
                    if let Some(request) = request {
                        if self.state.operations.contains_key(&request.operation_id) {
                            w.state = "committed".into();
                            w.fencing_token = w.fencing_token.max(
                                request.fencing_token.checked_add(1).ok_or_else(|| {
                                    Error::new(
                                        "RECOVERY_REQUIRED",
                                        "workspace fencing generation exhausted",
                                    )
                                })?,
                            );
                        }
                    }
                    if self
                        .workspaces
                        .get(&w.workspace_id)
                        .is_some_and(|existing| existing.fencing_token > w.fencing_token)
                    {
                        continue;
                    }
                    self.workspaces.insert(w.workspace_id.clone(), w);
                }
            }
        }
        Ok(())
    }
}
