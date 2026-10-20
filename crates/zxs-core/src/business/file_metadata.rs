//! Public file-branch labels are metadata, independent of immutable content.
use super::*;
use serde_json::json;

impl Engine {
    pub fn rename_file_branch(
        &mut self,
        file_id: &str,
        branch_id: &str,
        name: &str,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        validate_branch_name(name)?;
        self.mutate(
            "file.branch.rename",
            principal,
            operation,
            json!({"file_id":file_id,"branch_id":branch_id,"name":name}),
            |engine, state, _| {
                let current = state.branches.get(branch_id)
                    .filter(|branch| branch.file_id.as_deref()==Some(file_id)&&branch.state!="migrated")
                    .ok_or_else(||Error::new("FILE_BRANCH_NOT_FOUND","file branch missing"))?;
                let repository=engine.repository_for_history(&current.history_tree_id)?;
                engine.require_grant(repository,principal,"write")?;
                if current.state!="active" || state.file_branch_aliases.contains_key(branch_id) {
                    return Err(Error::new("BRANCH_NOT_ACTIVE","merged file aliases are read-only"));
                }
                if state.branches.values().any(|branch|branch.file_id.as_deref()==Some(file_id)
                    && branch.branch_id!=branch_id
                    && zxs_naming::lookup_key(&branch.name)==zxs_naming::lookup_key(name)) {
                    return Err(Error::new("NAME_COLLISION","a public branch of this file already uses that name"));
                }
                let previous=current.name.clone();
                if previous!=name {
                    state.branches.get_mut(branch_id).expect("checked branch").name=name.into();
                }
                Ok(json!({"file_id":file_id,"branch_id":branch_id,"history_tree_id":repository.history_tree_id,
                    "previous_name":previous,"name":name,"no_change":previous==name}))
            },
        )
    }
}
