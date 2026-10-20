//! Authenticated remote commands; pending commit requests remain explicit files.
use crate::{
    Result,
    args::{Args, Command},
    credentials::{load_token, password, save_token},
    op,
    pending::save_pending,
    print,
};
use serde_json::{Value, json};
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
mod tests {
    use super::*;
    #[test]
    fn login_output_preserves_u64_precision_and_excludes_all_credentials() {
        let session = storage_client::Session {
            token: "fixture-session-secret".into(),
            csrf_token: "fixture-csrf-secret".into(),
            principal: "admin".into(),
            admin: true,
            expires_at: u64::MAX,
        };
        let output = login_summary(&session, std::path::Path::new("token"));
        assert_eq!(output["expires_at"], u64::MAX.to_string());
        assert!(output.get("token").is_none());
        assert!(output.get("csrf_token").is_none());
        assert!(!output.to_string().contains("fixture-"));
    }
    #[tokio::test]
    async fn failed_token_publication_revokes_only_the_new_fixture_session() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let token = authentication::random_token().unwrap();
        let csrf = authentication::derive_csrf_token(&token).unwrap();
        let server = tokio::spawn(async move {
            let mut paths = Vec::new();
            for payload in [
                json!({"token":token,"csrf_token":csrf,"principal":"admin","admin":true,"expires_at":u64::MAX.to_string()}),
                json!({"logged_out":true}),
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let header_end = loop {
                    let mut buffer = [0_u8; 1024];
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0);
                    bytes.extend_from_slice(&buffer[..read]);
                    assert!(bytes.len() <= 8192);
                    if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                        break end + 4;
                    }
                };
                let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
                paths.push(
                    headers
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        .to_owned(),
                );
                let content_length = headers
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                while bytes.len() < header_end + content_length {
                    let mut buffer = [0_u8; 1024];
                    let read = stream.read(&mut buffer).await.unwrap();
                    assert!(read > 0);
                    bytes.extend_from_slice(&buffer[..read]);
                }
                let payload = serde_json::to_vec(&payload).unwrap();
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                stream.write_all(headers.as_bytes()).await.unwrap();
                stream.write_all(&payload).await.unwrap();
            }
            paths
        });
        let root = tempfile::tempdir().unwrap();
        let password = root.path().join("password");
        let output = root.path().join("already-exists");
        crate::private_io::write_new(&password, b"fixture-password").unwrap();
        crate::private_io::write_new(&output, b"preserved").unwrap();
        let args = Args {
            root: root.path().into(),
            principal: "admin".into(),
            server: Some(address.clone()),
            token_file: None,
            command: Command::Login {
                username: "admin".into(),
                password_file: Some(password),
                output: output.clone(),
            },
        };
        assert!(run(&args, &address).await.is_err());
        let paths = tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(paths, ["/api/auth/login", "/api/auth/logout"]);
        assert_eq!(std::fs::read(output).unwrap(), b"preserved");
    }
}
