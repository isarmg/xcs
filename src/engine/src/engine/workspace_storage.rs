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
    /// Match the exact requested receipt, never the first decodable JSON file.
    /// Its digest and owner are independently bound to the published operation
    /// by validate_workspace_commit_replay before this redundant storage lookup.
    pub(crate) fn require_saved_workspace_request(
        &self,
        workspace: &Workspace,
        request: &CommitRequest,
    ) -> Result<()> {
        let expected = canonical_json(request)?;
        let local = self
            .root
            .join("workspaces")
            .join(&workspace.workspace_id)
            .join("commit-request.json");
        let matches = |path: &std::path::Path| -> Result<bool> {
            match read_json::<CommitRequest>(path) {
                Ok(saved) => Ok(canonical_json(&saved)? == expected),
                Err(error) if error.code() == "UNSAFE_PATH" => Err(error),
                Err(_) => Ok(false),
            }
        };
        if matches(&local)? {
            return Ok(());
        }
        let data = self
            .workspace_data_directory(workspace)?
            .join("commit-request.json");
        if matches(&data)? {
            return Ok(());
        }
        Err(Error::new(
            "RECOVERY_REQUIRED",
            "no exact saved workspace commit request is available",
        ))
    }

    fn saved_request_digest_matches(
        &self,
        workspace: &Workspace,
        request: &CommitRequest,
        directories: &[PathBuf],
    ) -> bool {
        let Some(operation) = self.state.operations.get(&request.operation_id) else {
            return false;
        };
        if crate::business::native_commit_digest(workspace, request)
            .is_ok_and(|digest| digest == operation.request_digest)
        {
            return true;
        }
        // Completion advances the live fence, but the wire manifest commits
        // to the original request fence. Only restore that fence for hashing;
        // files, namespace operations, branch and base must remain unchanged.
        let mut frozen = workspace.clone();
        frozen.fencing_token = request.fencing_token;
        let Ok(manifest) = crate::wire::manifest_hash(&frozen) else {
            return false;
        };
        // Wire requests use a different envelope. Verify the original body,
        // not merely its caller-supplied digest string. Missing legacy body
        // copies leave the workspace untouched rather than guessing its state.
        directories.iter().any(|directory| {
            let Ok(body) = read_json::<serde_json::Value>(&directory.join("wire-request.json"))
            else {
                return false;
            };
            body["operation_id"].as_str() == Some(request.operation_id.as_str())
                && body["workspace_id"].as_str() == Some(workspace.workspace_id.as_str())
                && body["branch_id"].as_str() == Some(workspace.branch_id.as_str())
                && body["expected_head"].as_str() == Some(request.expected_head.as_str())
                && body["fencing_token"].as_str()
                    == Some(request.fencing_token.to_string().as_str())
                && body["message"].as_str() == Some(request.message.as_str())
                && body["request_digest"].as_str() == Some(operation.request_digest.as_str())
                && body["sealed_manifest_hash"].as_str() == Some(manifest.as_str())
                && crate::wire::commit_request_digest(&workspace.workspace_id, &body)
                    .is_ok_and(|digest| digest == operation.request_digest)
        })
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
                    // A request from another workspace (or a corrupt preferred
                    // copy) must never close this workspace's unsubmitted bytes.
                    let directories = [
                        local.parent().expect("workspace request parent").to_owned(),
                        data.parent().expect("workspace request parent").to_owned(),
                    ];
                    let request = [local, data].iter().find_map(|path| {
                        read_json::<CommitRequest>(path).ok().filter(|request| {
                            self.workspace_request_matches_operation(&w, request)
                                && self.saved_request_digest_matches(&w, request, &directories)
                        })
                    });
                    if let Some(request) = request {
                        w.state = "committed".into();
                        w.fencing_token =
                            w.fencing_token
                                .max(request.fencing_token.checked_add(1).ok_or_else(|| {
                                    Error::new(
                                        "RECOVERY_REQUIRED",
                                        "workspace fencing generation exhausted",
                                    )
                                })?);
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
