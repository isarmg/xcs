//! Local administration and engine operations. Remote API calls live separately.
use crate::{
    Result,
    args::{Args, Command},
    credentials::password,
    op,
    pending::save_json_private,
    print, private_io,
};
use serde_json::json;
use std::{path::Path, sync::Arc};
use xcs_core::{CommitRequest, Config, Engine, NamespaceOperation};
use xcs_server::{ServerConfig, auth::AuthStore, engine_backend::EngineBackend};

pub(crate) async fn run(args: Args) -> Result<()> {
    let root = args.root;
    match args.command {
        Command::RecoveryPreview { operation, reason } => {
            let mut engine = Engine::open_for_recovery(&root)?;
            print(engine.prepare_degraded_recovery(&operation, &args.principal, &reason)?)
        }
        Command::RecoveryCommit {
            recovery,
            digest,
            accept_permanent_loss,
        } => {
            if !accept_permanent_loss {
                return Err(
                    "review the recovery preview and explicitly pass --accept-permanent-loss"
                        .into(),
                );
            }
            let mut engine = Engine::open_for_recovery(&root)?;
            print(engine.commit_degraded_recovery(&recovery, &digest, &args.principal, true)?)
        }
        Command::CheckHistoryPackage { package } => {
            print(xcs_core::offline_check_history_package(&package)?)
        }
        Command::ConfigTemplate {
            production,
            disks,
            output,
        } => {
            if disks.iter().any(|path| !path.is_absolute())
                || disks
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != 3
            {
                return Err("provide three distinct absolute disk roots".into());
            }
            let mut config = Config::development(disks.try_into().unwrap());
            if production {
                config.mode = xcs_core::Mode::Production;
            }
            if let Some(path) = output {
                save_json_private(&path, &serde_json::to_value(config)?)?;
                print(json!({"configuration_draft":path,"devices_initialized":false}))
            } else {
                print(config)
            }
        }

        Command::RestoreSystem { disks } => {
            let engine = Engine::restore_from_disks(&root, &disks)?;
            let pool = engine.config().pool_id.clone();
            let repositories = engine.state().repositories.len();
            let backend = Arc::new(EngineBackend::new(engine));
            let _auth = backend.open_auth(&root)?;
            print(
                json!({"restored":true,"pool_id":pool,"repositories":repositories,"credentials_restored":true}),
            )
        }
        Command::InitAuth {
            username,
            password_file,
        } => {
            let engine = Engine::open(&root)?;
            if engine.auth_state().is_some() {
                return Err("authoritative credentials already exist; start the server to restore the protected credential state".into());
            }
            let secret = password(password_file.as_deref())?;
            let backend = Arc::new(EngineBackend::new(engine));
            backend.protect_auth(AuthStore::initialize(&root, &username, &secret)?)?;
            print(json!({"administrator":username,"initialized":true}))
        }
        Command::OfflineCheck { disk } => {
            let report = xcs_core::offline_check(&disk)?;
            let healthy = report.issues.is_empty();
            print(report)?;
            if healthy {
                Ok(())
            } else {
                Err("offline integrity check reported issues".into())
            }
        }
        Command::OfflineExport {
            disk,
            version,
            output,
        } => {
            xcs_core::offline_export(&disk, &version, &output)?;
            print(json!({"version_id":version,"exported":output,"mode":"offline-isolated"}))
        }
        Command::Init {
            config,
            development_disks,
            username,
            password_file,
        } => {
            let config = if let Some(path) = config {
                xcs_core::decode_strict::<Config>(&std::fs::read(path)?)?
            } else if development_disks.len() == 3 {
                Config::development(development_disks.try_into().unwrap())
            } else {
                return Err("provide --config or exactly three --development-disks paths".into());
            };
            let secret = password(password_file.as_deref())?;
            xcs_server::auth_primitives::normalize_administrator_username(&username)?;
            if root.join("auth.json").exists() {
                return Err("credentials already initialized".into());
            }
            let engine = Engine::initialize(&root, config)?;
            let pool_id = engine.config().pool_id.clone();
            let mode = engine.config().mode.clone();
            let backend = Arc::new(EngineBackend::new(engine));
            backend.protect_auth(AuthStore::initialize(&root, &username, &secret)?)?;
            print(
                json!({"pool_id":pool_id,"mode":mode,"initialized":true,"administrator":username}),
            )
        }
        Command::Serve {
            listen,
            external_origin,
            web_dir,
            max_upload_bytes,
            smb_socket,
            smb_peer_uid,
            smb_user,
        } => {
            let engine = Engine::open(&root)?;
            let backend = Arc::new(EngineBackend::new(engine));
            let auth = backend.open_auth(&root)?;
            let mut config = ServerConfig::loopback(&root);
            config.listen = listen;
            config.external_origin = external_origin;
            config.web_dir = web_dir;
            config.max_upload_bytes = max_upload_bytes;
            if let Some(socket) = smb_socket {
                let allowed_uid =
                    smb_peer_uid.ok_or("--smb-peer-uid is required with --smb-socket")?;
                let mut users = std::collections::BTreeMap::new();
                for mapping in smb_user {
                    let (alias, principal) = mapping
                        .split_once('=')
                        .ok_or("--smb-user must be SambaUser=CoreAccount")?;
                    if alias.is_empty()
                        || users
                            .insert(alias.to_owned(), principal.to_owned())
                            .is_some()
                    {
                        return Err("SMB user mappings must be nonempty and unique".into());
                    }
                    xcs_server::auth_primitives::require_canonical_administrator_username(
                        principal,
                    )?;
                }
                config.smb = Some(xcs_server::SmbConfig {
                    socket,
                    allowed_uid,
                    users,
                });
            }
            eprintln!("xcs API listening on {listen}");
            xcs_server::serve(backend, auth, config).await
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
            let backend = Arc::new(EngineBackend::new(Engine::open(&root)?));
            backend.open_auth(&root)?.upsert_account(
                &username,
                secret.as_deref(),
                admin,
                repository.into_iter().collect(),
                disabled,
            )?;
            print(json!({"username":username,"updated":true,"sessions_revoked":true}))
        }
        Command::RegisterDisk {
            config,
            slot,
            disk_id,
            path,
            level,
            upstream,
            metadata_member,
        } => {
            if root.join("auth.json").exists() {
                return Err("disk registration modifies only an uninitialized configuration draft; live migration requires a protected topology plan".into());
            }
            let original = std::fs::read(&config)?;
            let mut draft: Config = xcs_core::decode_strict(&original)?;
            if draft
                .disks
                .iter()
                .any(|disk| disk.slot_id == slot || disk.disk_id == disk_id || disk.root == path)
            {
                return Err("slot, disk identity and root must be unique".into());
            }
            if level == 0 || (level == 1 && upstream.is_some()) || (level > 1 && upstream.is_none())
            {
                return Err(
                    "primary disks have no upstream; higher levels require an upstream".into(),
                );
            }
            if let Some(source) = &upstream {
                if !draft
                    .disks
                    .iter()
                    .any(|disk| &disk.slot_id == source && disk.level.checked_add(1) == Some(level))
                {
                    return Err(
                        "upstream must name a disk in the immediately previous level".into(),
                    );
                }
            }
            draft.disks.push(xcs_core::DiskConfig {
                slot_id: slot.clone(),
                disk_id,
                root: path,
                level,
                upstream_slot_id: upstream,
                metadata_member,
                binding_generation: 1,
                capacity_bytes: None,
            });
            let backup = config.with_extension("json.before-registration");
            if backup.symlink_metadata().is_ok() {
                private_io::replace_private(&backup, &original)?;
            } else {
                private_io::write_new(&backup, &original)?;
            }
            private_io::replace_private(&config, &serde_json::to_vec_pretty(&draft)?)?;
            print(
                json!({"slot_id":slot,"draft":config,"backup":backup,"live_binding_changed":false}),
            )
        }
        Command::Login { .. } | Command::Logout | Command::Whoami => {
            Err("login/logout/whoami require --server".into())
        }
        command => local_operation(&root, &args.principal, command),
    }
}

fn local_operation(root: &Path, principal: &str, command: Command) -> Result<()> {
    let mut engine = Engine::open(root)?;
    match command {
        Command::CreateRepo {
            name,
            primary_slot,
            copies,
            operation_id,
        } => print(engine.create_repository(
            &name,
            &primary_slot,
            copies,
            principal,
            &op(operation_id),
        )?),
        Command::Repositories => print(engine.state().repositories.values().collect::<Vec<_>>()),
        Command::Disks => print(engine.disk_status()?),
        Command::Topology => print(
            json!({"generation":engine.state().topology_generation.to_string(),"disks":engine.config().disks}),
        ),
        Command::Put {
            branch,
            path,
            source,
            expected_head,
            message,
            operation_id,
        } => {
            let workspace = engine.create_workspace(&branch, principal)?;
            let head = expected_head.unwrap_or_else(|| workspace.base_head.clone());
            engine.stage_file(&workspace.workspace_id, &path, &source)?;
            print(engine.commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: op(operation_id),
                principal: principal.into(),
                expected_head: head,
                fencing_token: workspace.fencing_token,
                message: message.unwrap_or_else(|| format!("Save {path}")),
                request_digest: None,
            })?)
        }
        Command::Get { version, output } | Command::Export { version, output } => {
            engine.export_version(&version, &output)?;
            print(json!({"version_id":version,"exported":output}))
        }
        Command::List { branch } => print(engine.list_tree(&branch)?),
        Command::History { file } => print(engine.file_history(&file)?),
        Command::Graph { history } => print(engine.graph(&history)?),
        Command::Branch {
            source_commit,
            name,
            operation_id,
        } => print(engine.create_branch(&source_commit, &name, principal, &op(operation_id))?),
        Command::Link {
            source_branch,
            source_commit,
            target_branch,
            target_commit,
            operation_id,
        } => print(engine.add_merge_link(
            &source_branch,
            &source_commit,
            &target_branch,
            &target_commit,
            principal,
            &op(operation_id),
        )?),
        Command::Namespace {
            branch,
            kind,
            path,
            destination,
            operation_id,
        } => {
            let workspace = engine.create_workspace(&branch, principal)?;
            engine.namespace_operation(
                &workspace.workspace_id,
                NamespaceOperation {
                    kind,
                    path,
                    destination,
                },
            )?;
            print(engine.commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: op(operation_id),
                principal: principal.into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: "Namespace change".into(),
                request_digest: None,
            })?)
        }
        Command::Purge {
            history,
            versions,
            execute,
            operation_id,
        } => {
            let plan = engine.create_purge_plan(&history, versions, principal)?;
            if execute {
                print(engine.commit_plan(&plan.plan_id, principal, &op(operation_id))?)
            } else {
                print(plan)
            }
        }
        Command::Split {
            history,
            cut_commit,
            execute,
            operation_id,
        } => {
            let plan = engine.create_split_plan(&history, &cut_commit, principal)?;
            if execute {
                print(engine.commit_plan(&plan.plan_id, principal, &op(operation_id))?)
            } else {
                print(plan)
            }
        }

        Command::SmbBinding {
            repository,
            branch,
            enabled,
            expected_history_generation,
            operation_id,
        } => {
            let generation = xcs_core::parse_u64_decimal(&expected_history_generation)?;
            print(engine.set_smb_binding(
                &repository,
                &branch,
                enabled,
                generation,
                principal,
                &op(operation_id),
            )?)
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
                xcs_core::new_id("policy")
            } else {
                id
            };
            let generation = xcs_core::parse_u64_decimal(&expected_generation)?;
            print(engine.update_protection_policy(
                &id,
                &repository,
                &path_prefix,
                copies,
                generation,
                principal,
                &op(operation_id),
            )?)
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
                xcs_core::new_id("rule")
            } else {
                id
            };
            let generation = xcs_core::parse_u64_decimal(&expected_generation)?;
            print(engine.update_placement_rule(
                &id,
                &repository,
                &path_prefix,
                &primary_slot,
                generation,
                principal,
                &op(operation_id),
            )?)
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
            let history = engine
                .state()
                .branches
                .get(&branch)
                .ok_or("source branch is missing")?
                .history_tree_id
                .clone();
            let workspace = match workspace {
                Some(id) => engine.workspace(&id)?,
                None => engine.create_historical_workspace(&branch, &source_commit, principal)?,
            };
            if let (Some(source), Some(path)) = (source.as_deref(), path.as_deref()) {
                engine.stage_file(&workspace.workspace_id, path, source)?;
            }
            let plan = engine.create_historical_save_plan(
                &history,
                xcs_core::HistoricalSaveOptions {
                    source_commit_id: source_commit,
                    name,
                    workspace_id: Some(workspace.workspace_id),
                    primary_slot_id: primary_slot,
                    target_copies: copies,
                },
                principal,
            )?;
            if execute {
                print(engine.commit_plan(&plan.plan_id, principal, &op(operation_id))?)
            } else {
                print(plan)
            }
        }
        Command::PlanCommit {
            id,
            digest,
            operation_id,
        } => {
            if engine.plan(&id)?.digest != digest {
                return Err("plan digest differs from the reviewed preview".into());
            }
            print(engine.commit_plan(&id, principal, &op(operation_id))?)
        }
        Command::PlanCancel { id, operation_id } => {
            print(engine.cancel_plan(&id, principal, &op(operation_id))?)
        }
        Command::Operation { id } => print(engine.operation(&id)?),
        Command::ExportHistory {
            history,
            destination,
        } => print(engine.export_history_package(&history, &destination, principal)?),
        Command::RestoreHistoryPackage {
            package,
            primary_slot,
            copies,
            operation_id,
        } => print(engine.restore_history_package(
            &package,
            &primary_slot,
            copies,
            principal,
            &op(operation_id),
        )?),
        Command::Check => {
            let report = engine.check()?;
            let healthy = report.issues.is_empty();
            print(report)?;
            if healthy {
                Ok(())
            } else {
                Err("integrity check reported issues".into())
            }
        }
        _ => Err("command requires another execution mode".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registration(root: &Path, config: std::path::PathBuf) -> Args {
        Args {
            root: root.join("system"),
            principal: "admin".into(),
            server: None,
            token_file: None,
            command: Command::RegisterDisk {
                config,
                slot: "D01".into(),
                disk_id: xcs_core::new_id("disk"),
                path: root.join("fourth"),
                level: 2,
                upstream: Some("P01".into()),
                metadata_member: false,
            },
        }
    }
    #[tokio::test]
    async fn registering_a_disk_keeps_an_exact_backup_and_atomically_replaces_the_draft() {
        let root = tempfile::tempdir().unwrap();
        let draft = Config::development([
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ]);
        let path = root.path().join("config.json");
        let original = serde_json::to_vec(&draft).unwrap();
        private_io::write_new(&path, &original).unwrap();
        run(registration(root.path(), path.clone())).await.unwrap();
        let saved: Config = xcs_core::decode_strict(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.disks.len(), 4);
        assert_eq!(saved.disks.last().unwrap().slot_id, "D01");
        assert_eq!(
            std::fs::read(path.with_extension("json.before-registration")).unwrap(),
            original
        );
    }
    #[tokio::test]
    async fn invalid_upstream_level_is_an_error_and_never_modifies_the_draft() {
        let root = tempfile::tempdir().unwrap();
        let mut draft = Config::development([
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ]);
        draft.disks[0].level = u32::MAX;
        let path = root.path().join("config.json");
        let original = serde_json::to_vec(&draft).unwrap();
        private_io::write_new(&path, &original).unwrap();
        assert!(run(registration(root.path(), path.clone())).await.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(!path.with_extension("json.before-registration").exists());
    }
    #[tokio::test]
    async fn unsafe_password_input_is_rejected_before_pool_or_disk_initialization() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let password_file = root.path().join("password");
        private_io::write_new(&password_file, b"fixture-password").unwrap();
        std::fs::set_permissions(&password_file, std::fs::Permissions::from_mode(0o644)).unwrap();
        let system = root.path().join("system");
        let disks = [
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ];
        let args = Args {
            root: system.clone(),
            principal: "admin".into(),
            server: None,
            token_file: None,
            command: Command::Init {
                config: None,
                development_disks: disks.to_vec(),
                username: "admin".into(),
                password_file: Some(password_file),
            },
        };
        assert!(run(args).await.is_err());
        assert!(!system.exists());
        assert!(disks.iter().all(|path| !path.exists()));
    }
}
