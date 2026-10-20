//! Conditional policy updates affect future full versions only.
use crate::*;
use serde_json::json;
mod backup_paths;
pub(crate) use backup_paths::apply_backup_paths;

fn applies(prefix: &str, path: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    let prefix = zxs_naming::path_key(prefix);
    let path = zxs_naming::path_key(path);
    path == prefix || path.starts_with(&format!("{prefix}/"))
}
impl Engine {
    pub fn new_file_placement(&self, repository: &str, path: &str) -> Result<(String, u32)> {
        self.new_file_placement_for_file(repository, None, path)
    }
    pub fn new_file_placement_for_file(
        &self,
        repository: &str,
        file_id: Option<&str>,
        path: &str,
    ) -> Result<(String, u32)> {
        let repo = self
            .state
            .repositories
            .get(repository)
            .ok_or_else(|| Error::new("REPOSITORY_NOT_FOUND", "repository does not exist"))?;
        let primary = self
            .state
            .placement_rules
            .values()
            .filter(|r| r.repository_id == repository && applies(&r.path_prefix, path))
            .max_by_key(|r| r.path_prefix.len())
            .map(|r| r.primary_slot_id.clone())
            .unwrap_or_else(|| repo.primary_slot_id.clone());
        let copies = backup_paths::effective_policy(&self.state, repository, file_id, path)
            .map(|p| p.target_copies)
            .unwrap_or(repo.target_copies);
        self.route(&primary, copies)?;
        Ok((primary, copies))
    }
    pub fn update_placement_rule(
        &mut self,
        id: &str,
        repository: &str,
        path_prefix: &str,
        primary: &str,
        expected_generation: u64,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        validate_id(id, "rule")?;
        if !path_prefix.is_empty() {
            zxs_naming::validate_path(path_prefix)?;
        }
        self.mutate("placement-rule.update",principal,operation,json!({"rule_id":id,"repository_id":repository,"path_prefix":path_prefix,"primary_slot_id":primary,"expected_generation":expected_generation.to_string()}),|engine,state,_|{
            let repo=engine.state.repositories.get(repository).ok_or_else(||Error::new("REPOSITORY_NOT_FOUND","repository does not exist"))?;
            engine.require_grant(repo,principal,"manage")?;
            let actual=state.placement_rules.get(id).map_or(0,|r|r.generation);
            if actual!=expected_generation { return Err(Error::new("GENERATION_MISMATCH","placement rule changed; reread before updating")); }
            if state.placement_rules.get(id).is_some_and(|r|r.repository_id!=repository) { return Err(Error::new("REQUEST_BINDING_MISMATCH","rule is bound to another repository")); }
            if state.placement_rules.values().any(|r|r.rule_id!=id&&r.repository_id==repository&&zxs_naming::path_key(&r.path_prefix)==zxs_naming::path_key(path_prefix)) { return Err(Error::new("NAME_COLLISION","an equivalent placement prefix already has a rule")); }
            engine.route(primary,engine.new_file_placement(repository,path_prefix)?.1)?;
            let rule=PlacementRule{rule_id:id.into(),repository_id:repository.into(),path_prefix:path_prefix.into(),primary_slot_id:engine.disk(primary)?.slot_id.clone(),generation:actual.checked_add(1).ok_or_else(||Error::new("RESOURCE_LIMIT","rule generation exhausted"))?};
            state.placement_rules.insert(id.into(),rule.clone());
            Ok(json!({"rule":rule}))
        })
    }
    pub fn update_protection_policy(
        &mut self,
        id: &str,
        repository: &str,
        path_prefix: &str,
        copies: u32,
        expected_generation: u64,
        principal: &str,
        operation: &str,
    ) -> Result<OperationResult> {
        validate_id(id, "policy")?;
        if !path_prefix.is_empty() {
            zxs_naming::validate_path(path_prefix)?;
        }
        self.mutate("protection-policy.update",principal,operation,json!({"policy_id":id,"repository_id":repository,"path_prefix":path_prefix,"target_copies":copies,"expected_generation":expected_generation.to_string()}),|engine,state,_|{
            let repo=engine.state.repositories.get(repository).ok_or_else(||Error::new("REPOSITORY_NOT_FOUND","repository does not exist"))?;
            engine.require_grant(repo,principal,"manage")?;
            let actual=state.protection_policies.get(id).map_or(0,|p|p.generation);
            if actual!=expected_generation { return Err(Error::new("GENERATION_MISMATCH","protection policy changed; reread before updating")); }
            if state.protection_policies.get(id).is_some_and(|p|p.repository_id!=repository) { return Err(Error::new("REQUEST_BINDING_MISMATCH","policy is bound to another repository")); }
            if state.protection_policies.values().any(|p|p.policy_id!=id&&p.repository_id==repository&&zxs_naming::path_key(&p.path_prefix)==zxs_naming::path_key(path_prefix)) { return Err(Error::new("NAME_COLLISION","an equivalent protection prefix already has a policy")); }
            engine.route(&engine.new_file_placement(repository,path_prefix)?.0,copies)?;
            let previous=state.protection_policies.get(id);
            let policy=ProtectionPolicy{policy_id:id.into(),repository_id:repository.into(),path_prefix:path_prefix.into(),target_copies:copies,scope_directory_id:previous.and_then(|policy|policy.scope_directory_id.clone()),scope_file_id:previous.and_then(|policy|policy.scope_file_id.clone()),backup_prefixes:previous.map(|policy|policy.backup_prefixes.clone()).unwrap_or_default(),generation:actual.checked_add(1).ok_or_else(||Error::new("RESOURCE_LIMIT","policy generation exhausted"))?};
            state.protection_policies.insert(id.into(),policy.clone());
            Ok(json!({"policy":policy}))
        })
    }
}
