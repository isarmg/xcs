//! Exact HTTP contracts for daily administration. Never builds content transfers.
use json_codec::{Value, json};
use std::collections::BTreeMap;

fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
pub(super) type Fields = BTreeMap<String, String>;
pub(super) fn items(value: &str) -> Vec<String> {
    value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
pub(super) fn build(
    kind: &str,
    fields: &Fields,
    operation: &str,
) -> Result<(&'static str, String, Value), String> {
    let value = |key: &str| fields.get(key).cloned().unwrap_or_default();
    let required = |key: &str| {
        let v = value(key);
        if v.trim().is_empty() {
            Err(format!("{key} is required"))
        } else {
            Ok(v)
        }
    };
    let copies = || {
        required("copies")?
            .parse::<u32>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| "copies must be a positive integer".to_string())
    };
    let generation = || {
        let n = required("generation")?;
        storage_contract::parse_u64_decimal(&n).map_err(|e| e.to_string())?;
        Ok::<_, String>(n)
    };
    let path = |prefix: &str, key: &str, suffix: &str| {
        Ok::<_, String>(
            format!("{prefix}/{}/{suffix}", segment(&required(key)?))
                .trim_end_matches('/')
                .to_string(),
        )
    };
    if kind != "account" && operation.is_empty() {
        return Err("Secure operation ID unavailable".into());
    }
    Ok(match kind {
        "account" => {
            let password = value("password");
            (
                "POST",
                "/api/admin/accounts".into(),
                json!({"username":required("username")?,"password":if password.is_empty(){None}else{Some(password)},"admin":value("admin")=="true","disabled":value("disabled")=="true","repositories":items(&value("repositories"))}),
            )
        }
        "repository" => (
            "POST",
            "/api/repositories".into(),
            json!({"name":required("name")?,"primary_slot_id":required("slot")?,"target_copies":copies()?,"operation_id":operation}),
        ),
        "grants" => (
            "PUT",
            path("/api/repositories", "repository", "grants")?,
            json!({"grants":{"read":items(&value("read")),"write":items(&value("write")),"manage":items(&value("manage"))},"operation_id":operation}),
        ),
        "smb" => (
            "PUT",
            path("/api/repositories", "repository", "smb-binding")?,
            json!({"branch_id":required("branch")?,"enabled":value("enabled")=="true","expected_history_generation":generation()?,"operation_id":operation}),
        ),
        "policy" => (
            "PUT",
            path("/api/policies", "id", "")?,
            json!({"repository_id":required("repository")?,"path_prefix":value("prefix"),"target_copies":copies()?,"expected_generation":generation()?,"operation_id":operation}),
        ),
        "rule" => (
            "PUT",
            path("/api/primary-placement-rules", "id", "")?,
            json!({"repository_id":required("repository")?,"path_prefix":value("prefix"),"primary_slot_id":required("slot")?,"expected_generation":generation()?,"operation_id":operation}),
        ),
        "branch" => (
            "POST",
            path("/api/repositories", "repository", "branches")?,
            json!({"source_commit_id":required("commit")?,"name":required("name")?,"operation_id":operation}),
        ),
        "link" => (
            "POST",
            path("/api/history-trees", "history", "merge-links")?,
            json!({"source_branch_id":required("source_branch")?,"source_commit_id":required("source_commit")?,"target_branch_id":required("target_branch")?,"target_commit_id":required("target_commit")?,"operation_id":operation}),
        ),
        "purge" => {
            let versions = items(&required("versions")?);
            (
                "POST",
                path("/api/history-trees", "history", "purge-plans")?,
                json!({"version_ids":versions}),
            )
        }
        "split" => (
            "POST",
            path("/api/history-trees", "history", "split-plans")?,
            json!({"cut_commit_id":required("commit")?}),
        ),
        "save" => (
            "POST",
            path("/api/history-trees", "history", "historical-save-plans")?,
            json!({"source_commit_id":required("commit")?,"name":required("name")?,"workspace_id":if value("workspace").is_empty(){None}else{Some(value("workspace"))},"primary_slot_id":if value("slot").is_empty(){None}else{Some(value("slot"))},"target_copies":if value("copies").is_empty(){None}else{Some(copies()?)}}),
        ),
        _ => return Err("Unknown management action".into()),
    })
}
pub(super) fn plan_action(
    plan: &Value,
    commit: bool,
    acknowledged: bool,
    operation: &str,
) -> Result<(&'static str, String, Value), String> {
    let id = plan["plan_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("Load a plan first")?;
    if plan["state"] != "ready" {
        return Err("Only a ready plan may be committed or cancelled".into());
    }
    if operation.is_empty() {
        return Err("Secure operation ID unavailable".into());
    }
    if commit {
        if !acknowledged {
            return Err("Review the full plan and acknowledge its consequences first".into());
        }
        let digest = plan["digest"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("Plan digest missing")?;
        Ok((
            "POST",
            format!("/api/plans/{}/commit", segment(id)),
            json!({"plan_digest":digest,"operation_id":operation}),
        ))
    } else {
        Ok((
            "POST",
            format!("/api/plans/{}/cancel", segment(id)),
            json!({"operation_id":operation}),
        ))
    }
}
