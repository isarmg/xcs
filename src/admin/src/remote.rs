//! Authenticated remote commands; pending commit requests remain explicit files.
use crate::{
    Result,
    args::{Args, Command},
    credentials::{load_token, password, save_token},
    op,
    pending::save_pending,
    print,
};
use json_codec::{Value, json};
pub(crate) async fn run(args: &Args, address: &str) -> Result<()> {
    let mut client = storage_client::Client::connect(address)?;
    if let Some(file) = &args.token_file {
        client.set_token(Some(load_token(file)?));
    }
    let request = |method: &'static str, path: String, body: Option<Value>| {
        let client = client.clone();
        async move { client.request(method, &path, body.as_ref()).await }
    };
    let segment = storage_client::segment;
    let command = &args.command;
    let value = match command {
        Command::Login {
            username,
            password_file,
            output,
        } => {
            let secret = password(password_file.as_deref())?;
            let session = client.login(username, &secret).await?;
            if let Err(error) = save_token(output, &session.token) {
                // Try to revoke a new session whose private output could not
                // be published. Preserve the original local publication error.
                let _ = client.logout().await;
                return Err(error);
            }
            return print(login_summary(&session, output));
        }
        Command::RetryCommit { request_file } => {
            let prepared: Value = storage_engine::strict_json(&std::fs::read(request_file)?)?;
            client.commit_prepared(&prepared).await?
        }
        Command::Logout => {
            client.logout().await?;
            json!({ "logged_out" : true })
        }
        Command::Whoami => request("GET", "/api/auth/me".into(), None).await?,
        Command::Repositories => request("GET", "/api/repositories".into(), None).await?,
        Command::Disks => request("GET", "/api/admin/disks".into(), None).await?,
        Command::Topology => request("GET", "/api/backup-topologies/default".into(), None).await?,
        Command::CreateRepo {
            name,
            primary_slot,
            copies,
            operation_id,
        } => {
            request(
                "POST",
                "/api/repositories".into(),
                Some(json!(
                    { "name" : name, "primary_slot_id" : primary_slot,
                    "target_copies" : copies, "operation_id" : op(operation_id
                    .clone()) }
                )),
            )
            .await?
        }
        Command::List { branch } => {
            request(
                "GET",
                format!("/api/branches/{}/tree", segment(branch)),
                None,
            )
            .await?
        }
        Command::History { file } => {
            request("GET", format!("/api/files/{}/history", segment(file)), None).await?
        }
        Command::Graph { history } => {
            request(
                "GET",
                format!("/api/history-trees/{}/graph", segment(history)),
                None,
            )
            .await?
        }
        Command::Operation { id } => {
            request("GET", format!("/api/operations/{}", segment(id)), None).await?
        }
        Command::Get { version, output } | Command::Export { version, output } => {
            client.download_file(version, output).await?;
            json!({ "version_id" : version, "exported" : output })
        }
        Command::Put {
            branch,
            path,
            source,
            expected_head,
            message,
            operation_id,
        } => {
            let workspace = request(
                "POST",
                "/api/workspaces".into(),
                Some(json!({ "branch_id" : branch })),
            )
            .await?;
            let id = workspace["workspace_id"]
                .as_str()
                .ok_or("missing workspace identity")?;
            let staged = client
                .upload_file(id, &storage_engine::new_id("entry"), path, source)
                .await?;
            eprintln!("uploaded {} bytes; workspace {id}", staged["size"]);
            let operation_id = op(operation_id.clone());
            let prepared = client
                .prepare_commit(
                    id,
                    &operation_id,
                    &message.clone().unwrap_or_else(|| format!("Save {path}")),
                    expected_head.as_deref(),
                )
                .await?;
            let pending = save_pending(&args.root, &operation_id, &prepared)?;
            eprintln!(
                "commit request preserved at {}; operation {}",
                pending.display(),
                operation_id
            );
            client.commit_prepared(&prepared).await?
        }
        Command::Branch {
            source_commit,
            name,
            operation_id,
        } => {
            let commit = request(
                "GET",
                format!("/api/commits/{}", segment(source_commit)),
                None,
            )
            .await?;
            let history = commit["history_tree_id"]
                .as_str()
                .ok_or("missing history")?;
            let graph = request(
                "GET",
                format!("/api/history-trees/{}/graph", segment(history)),
                None,
            )
            .await?;
            let repo = graph["history"]["repository_id"]
                .as_str()
                .ok_or("missing repository")?;
            request(
                "POST",
                format!("/api/repositories/{}/branches", segment(repo)),
                Some(json!(
                    { "source_commit_id" : source_commit, "name" : name,
                    "operation_id" : op(operation_id.clone()) }
                )),
            )
            .await?
        }
        Command::Link {
            source_branch,
            source_commit,
            target_branch,
            target_commit,
            operation_id,
        } => {
            let commit = request(
                "GET",
                format!("/api/commits/{}", segment(source_commit)),
                None,
            )
            .await?;
            let history = commit["history_tree_id"]
                .as_str()
                .ok_or("missing history")?;
            request(
                "POST",
                format!("/api/history-trees/{}/merge-links", segment(history)),
                Some(json!(
                    { "source_branch_id" : source_branch, "source_commit_id" :
                    source_commit, "target_branch_id" : target_branch,
                    "target_commit_id" : target_commit, "operation_id" :
                    op(operation_id.clone()) }
                )),
            )
            .await?
        }
        Command::Namespace {
            branch,
            kind,
            path,
            destination,
            operation_id,
        } => {
            let workspace = request(
                "POST",
                "/api/workspaces".into(),
                Some(json!({ "branch_id" : branch })),
            )
            .await?;
            let id = workspace["workspace_id"]
                .as_str()
                .ok_or("missing workspace identity")?;
            request(
                "POST",
                format!("/api/workspaces/{}/namespace-operations", segment(id)),
                Some(json!(
                    { "kind" : kind, "path" : path, "destination" : destination }
                )),
            )
            .await?;
            let operation_id = op(operation_id.clone());
            let prepared = client
                .prepare_commit(id, &operation_id, "Namespace change", None)
                .await?;
            let pending = save_pending(&args.root, &operation_id, &prepared)?;
            eprintln!("commit request preserved at {}", pending.display());
            client.commit_prepared(&prepared).await?
        }
        Command::Check => request("GET", "/api/admin/check".into(), None).await?,
        Command::SmbBinding {
            repository,
            branch,
            enabled,
            expected_history_generation,
            operation_id,
        } => {
            request(
                "PUT",
                format!("/api/repositories/{}/smb-binding", segment(repository)),
                Some(json!(
                    { "branch_id" : branch, "enabled" : enabled,
                    "expected_history_generation" : expected_history_generation,
                    "operation_id" : op(operation_id.clone()) }
                )),
            )
            .await?
        }
        Command::Policy {
            id,
            repository,
            path_prefix,
            copies,
            expected_generation,
            operation_id,
        } => {
            let id = if id == "new" {
                storage_engine::new_id("policy")
            } else {
                id.clone()
            };
            request(
                "PUT",
                format!("/api/policies/{}", segment(&id)),
                Some(json!(
                    { "repository_id" : repository, "path_prefix" : path_prefix,
                    "target_copies" : copies, "expected_generation" :
                    expected_generation, "operation_id" : op(operation_id
                    .clone()) }
                )),
            )
            .await?
        }
        Command::Rule {
            id,
            repository,
            path_prefix,
            primary_slot,
            expected_generation,
            operation_id,
        } => {
            let id = if id == "new" {
                storage_engine::new_id("rule")
            } else {
                id.clone()
            };
            request(
                "PUT",
                format!("/api/primary-placement-rules/{}", segment(&id)),
                Some(json!(
                    { "repository_id" : repository, "path_prefix" : path_prefix,
                    "primary_slot_id" : primary_slot, "expected_generation" :
                    expected_generation, "operation_id" : op(operation_id
                    .clone()) }
                )),
            )
            .await?
        }
        Command::HistoricalSave {
            branch,
            source_commit,
            name,
            workspace,
            source,
            path,
            primary_slot,
            copies,
            execute,
            operation_id,
        } => {
            let branch_info =
                request("GET", format!("/api/branches/{}", segment(branch)), None).await?;
            let history = branch_info["history_tree_id"]
                .as_str()
                .ok_or("source history missing")?;
            let workspace = match workspace {
                Some(id) => {
                    request("GET", format!("/api/workspaces/{}", segment(id)), None).await?
                }
                None => {
                    request(
                        "POST",
                        "/api/workspaces".into(),
                        Some(json!(
                            { "branch_id" : branch, "source_commit_id" : source_commit }
                        )),
                    )
                    .await?
                }
            };
            let id = workspace["workspace_id"]
                .as_str()
                .ok_or("historical workspace identity missing")?;
            if let (Some(source), Some(path)) = (source.as_deref(), path.as_deref()) {
                client
                    .upload_file(id, &storage_engine::new_id("entry"), path, source)
                    .await?;
            }
            let plan = request(
                "POST",
                format!(
                    "/api/history-trees/{}/historical-save-plans",
                    segment(history)
                ),
                Some(json!(
                    { "source_commit_id" : source_commit, "name" : name,
                    "workspace_id" : id, "primary_slot_id" : primary_slot,
                    "target_copies" : copies }
                )),
            )
            .await?;
            remote_plan(&client, plan, *execute, operation_id.clone()).await?
        }
        Command::PlanCommit {
            id,
            digest,
            operation_id,
        } => {
            request(
                "POST",
                format!("/api/plans/{}/commit", segment(id)),
                Some(json!(
                    { "operation_id" : op(operation_id.clone()), "plan_digest" :
                    digest }
                )),
            )
            .await?
        }
        Command::PlanCancel { id, operation_id } => {
            request(
                "POST",
                format!("/api/plans/{}/cancel", segment(id)),
                Some(json!({ "operation_id" : op(operation_id.clone()) })),
            )
            .await?
        }
        Command::Purge {
            history,
            versions,
            execute,
            operation_id,
        } => {
            let plan = request(
                "POST",
                format!("/api/history-trees/{}/purge-plans", segment(history)),
                Some(json!({ "version_ids" : versions })),
            )
            .await?;
            remote_plan(&client, plan, *execute, operation_id.clone()).await?
        }
        Command::Split {
            history,
            cut_commit,
            execute,
            operation_id,
        } => {
            let plan = request(
                "POST",
                format!("/api/history-trees/{}/split-plans", segment(history)),
                Some(json!({ "cut_commit_id" : cut_commit })),
            )
            .await?;
            remote_plan(&client, plan, *execute, operation_id.clone()).await?
        }
        Command::Account {
            username,
            password_file,
            admin,
            disabled,
            repository,
        } => {
            let secret = password_file
                .as_deref()
                .map(|path| password(Some(path)))
                .transpose()?;
            request(
                "POST",
                "/api/admin/accounts".into(),
                Some(json!(
                    { "username" : username, "password" : secret, "admin" :
                    admin, "disabled" : disabled, "repositories" : repository }
                )),
            )
            .await?
        }
        _ => return Err("this command currently requires local pool access".into()),
    };
    if matches!(command, Command::Check)
        && value
            .get("issues")
            .and_then(Value::as_array)
            .is_some_and(|issues| !issues.is_empty())
    {
        print(value)?;
        return Err("remote integrity check reported issues".into());
    }
    print(value)
}
async fn remote_plan(
    client: &storage_client::Client,
    plan: Value,
    execute: bool,
    operation_id: Option<String>,
) -> Result<Value> {
    if !execute {
        return Ok(plan);
    }
    let id = plan["plan_id"].as_str().ok_or("missing plan identity")?;
    Ok(client
        .request(
            "POST",
            &format!("/api/plans/{}/commit", storage_client::segment(id)),
            Some(&json!(
                { "operation_id" : op(operation_id), "plan_digest" :
                plan["digest"] }
            )),
        )
        .await?)
}

fn login_summary(session: &storage_client::Session, output: &std::path::Path) -> Value {
    json!({"principal":session.principal,"admin":session.admin,
        "expires_at":session.expires_at.to_string(),"token_file":output})
}

#[cfg(test)]
#[path = "../../../test/admin/unit/remote/tests.rs"]
mod tests;
