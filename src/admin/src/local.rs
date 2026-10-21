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
use storage_engine::{CommitRequest, Config, Engine, NamespaceOperation};
use storage_server::{ServerConfig, auth::AuthStore, engine_backend::EngineBackend};

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
            print(storage_engine::offline_check_history_package(&package)?)
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
                config.mode = storage_engine::Mode::Production;
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
            let report = storage_engine::offline_check(&disk)?;
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
            storage_engine::offline_export(&disk, &version, &output)?;
            print(json!({"version_id":version,"exported":output,"mode":"offline-isolated"}))
        }
        Command::Init {
            config,
            development_disks,
            username,
            password_file,
        } => {
            let config = if let Some(path) = config {
                storage_engine::decode_strict::<Config>(&std::fs::read(path)?)?
            } else if development_disks.len() == 3 {
                Config::development(development_disks.try_into().unwrap())
            } else {
                return Err("provide --config or exactly three --development-disks paths".into());
            };
            let secret = password(password_file.as_deref())?;
            authentication::normalize_administrator_username(&username)?;
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
            trusted_proxy_ips,
            web_dir,
            max_upload_bytes,
            http_spool_bytes_per_account,
            http_spool_bytes_total,
            http_spool_recovery_reserve_bytes,
            http_transfer_slots_total,
            http_transfer_slots_per_account,
            http_transfer_idle_seconds,
            http_transfer_total_seconds,
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
            config.trusted_proxy_ips = trusted_proxy_ips;
            config.web_dir = web_dir;
            config.max_upload_bytes = max_upload_bytes;
            config.http_spool_limits = storage_server::SpoolLimits {
                bytes_per_account: http_spool_bytes_per_account,
                bytes_total: http_spool_bytes_total,
                recovery_reserve_bytes: http_spool_recovery_reserve_bytes,
            };
            config.http_transfer_limits = storage_server::TransferLimits {
                slots_total: http_transfer_slots_total,
                slots_per_account: http_transfer_slots_per_account,
                idle_timeout: std::time::Duration::from_secs(http_transfer_idle_seconds),
                total_timeout: std::time::Duration::from_secs(http_transfer_total_seconds),
            };
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
                    authentication::require_canonical_administrator_username(principal)?;
                }
                config.smb = Some(storage_server::SmbConfig {
                    socket,
                    allowed_uid,
                    users,
                });
            }
            eprintln!("zxs API listening on {listen}");
            storage_server::serve(backend, auth, config).await
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
            // These paths are part of the JSON draft and success receipt.
            // Validate before preserving a backup or changing either file.
            if config.to_str().is_none() || path.to_str().is_none() {
                return Err("configuration and disk paths must be valid UTF-8 for JSON output; the draft was not changed".into());
            }
            // Match the engine's bounded metadata reader, including whitespace
            // in hand-written drafts. Never read a link or unowned input first.
            const MAX_DRAFT_BYTES: usize = 64 * 1024 * 1024;
            let original = private_io::read_owned_bytes(&config, MAX_DRAFT_BYTES)?;
            let mut draft: Config = storage_engine::decode_strict(&original)?;
            draft.disks.push(storage_engine::DiskConfig {
                slot_id: slot.clone(),
                disk_id,
                root: path,
                level,
                upstream_slot_id: upstream,
                metadata_member,
                binding_generation: 1,
                capacity_bytes: None,
            });
            Engine::validate_configuration(&draft)?;
            ensure_uninitialized_draft(&config, &draft)?;
            let candidate = serde_json::to_vec_pretty(&draft)?;
            if candidate.len() > MAX_DRAFT_BYTES {
                return Err("configuration draft exceeds its byte limit".into());
            }
            let backup = registration_backup(&config)?;
            let backup_result = if backup.symlink_metadata().is_ok() {
                private_io::replace_private(&backup, &original)
            } else {
                private_io::write_new(&backup, &original)
            };
            backup_result.map_err(|error| {
                format!(
                    "cannot preserve configuration backup {} (the draft was not changed): {error}",
                    backup.display()
                )
            })?;
            private_io::replace_private(&config, &candidate)?;
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

/// The selected configuration, not an unrelated --root argument, determines
/// whether this is an offline draft. Stat only: never initialize or lock disks.
fn ensure_uninitialized_draft(path: &Path, config: &Config) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()?;
    for marker in ["auth.json", "writer.lock", "index.sqlite", "workspaces"] {
        reject_initialized_marker(&parent.join(marker))?;
    }
    let mut resolved = config.clone();
    for disk in &mut resolved.disks {
        disk.root = resolve_draft_disk_root(&disk.root)?;
        // An enrolled disk remains enrolled when the draft was copied outside
        // the system root, or when the new path is nested inside its root.
        for ancestor in disk.root.ancestors() {
            for marker in ["disk.json", ".writer.lock"] {
                reject_initialized_marker(&ancestor.join(marker))?;
            }
        }
        reject_initialized_marker(&disk.root.join("metadata/pool-membership/config.json"))?;
    }
    // Existing directory aliases and aliases of a not-yet-created child must
    // not hide overlapping disk roots from the shared topology validation.
    Engine::validate_configuration(&resolved)?;
    Ok(())
}

fn reject_initialized_marker(path: &Path) -> Result<()> {
    match path.symlink_metadata() {
        Ok(_) => Err("disk registration modifies only an uninitialized configuration draft; live migration requires a protected topology plan".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn resolve_draft_disk_root(path: &Path) -> Result<std::path::PathBuf> {
    let mut existing = path;
    let mut suffix = Vec::new();
    loop {
        match existing.canonicalize() {
            Ok(mut resolved) => {
                if !resolved.is_dir() {
                    return Err("disk root must name a directory".into());
                }
                for name in suffix.into_iter().rev() {
                    resolved.push(name);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // A dangling symlink is not a missing, safely creatable root.
                if existing.symlink_metadata().is_ok() {
                    return Err("disk root contains a dangling link".into());
                }
                suffix.push(
                    existing
                        .file_name()
                        .ok_or("disk root has no existing parent")?,
                );
                existing = existing
                    .parent()
                    .ok_or("disk root has no existing parent")?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn registration_backup(config: &Path) -> Result<std::path::PathBuf> {
    let mut name = config
        .file_name()
        .ok_or("configuration draft must have a file name")?
        .to_os_string();
    // Preserve the complete name: config.json and config.draft must never
    // share a recovery file. Keep non-UTF8 path bytes intact.
    name.push(".before-registration");
    Ok(config.with_file_name(name))
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
            let generation = storage_engine::parse_u64_decimal(&expected_history_generation)?;
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
                storage_engine::new_id("policy")
            } else {
                id
            };
            let generation = storage_engine::parse_u64_decimal(&expected_generation)?;
            print(
                engine.update_protection_policy(storage_engine::ProtectionPolicyUpdate {
                    policy_id: &id,
                    repository_id: &repository,
                    path_prefix: &path_prefix,
                    target_copies: copies,
                    expected_generation: generation,
                    principal,
                    operation_id: &op(operation_id),
                })?,
            )
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
                id
            };
            let generation = storage_engine::parse_u64_decimal(&expected_generation)?;
            print(
                engine.update_placement_rule(storage_engine::PlacementRuleUpdate {
                    rule_id: &id,
                    repository_id: &repository,
                    path_prefix: &path_prefix,
                    primary_slot_id: &primary_slot,
                    expected_generation: generation,
                    principal,
                    operation_id: &op(operation_id),
                })?,
            )
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
                storage_engine::HistoricalSaveOptions {
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
mod registration_tests;

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
                disk_id: storage_engine::new_id("disk"),
                path: root.join("fourth"),
                level: 1,
                upstream: None,
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
        let saved: Config = storage_engine::decode_strict(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.disks.len(), 4);
        assert_eq!(saved.disks.last().unwrap().slot_id, "D01");
        assert_eq!(
            std::fs::read(path.with_extension("json.before-registration")).unwrap(),
            original
        );
    }
    #[tokio::test]
    async fn different_configuration_extensions_keep_independent_registration_backups() {
        let root = tempfile::tempdir().unwrap();
        let draft = Config::development([
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ]);
        let first = root.path().join("config.json");
        let second = root.path().join("config.draft");
        let original = serde_json::to_vec(&draft).unwrap();
        private_io::write_new(&first, &original).unwrap();
        let mut other = draft.clone();
        other.pool_id = storage_engine::new_id("pool");
        let other_original = serde_json::to_vec(&other).unwrap();
        private_io::write_new(&second, &other_original).unwrap();
        run(registration(root.path(), first)).await.unwrap();
        run(registration(root.path(), second)).await.unwrap();
        assert_eq!(
            std::fs::read(root.path().join("config.json.before-registration")).unwrap(),
            original
        );
        assert_eq!(
            std::fs::read(root.path().join("config.draft.before-registration")).unwrap(),
            other_original
        );
    }

    #[test]
    fn registration_backup_preserves_the_complete_native_file_name() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        for name in ["config", "config.json", "config.draft", ".config"] {
            assert_eq!(
                registration_backup(Path::new(name)).unwrap(),
                Path::new(&format!("{name}.before-registration"))
            );
        }
        let name = std::ffi::OsString::from_vec(b"config-\xff.json".to_vec());
        assert_eq!(
            registration_backup(Path::new(&name))
                .unwrap()
                .as_os_str()
                .as_bytes(),
            b"config-\xff.json.before-registration"
        );
        assert!(registration_backup(Path::new("/")).is_err());
    }

    #[tokio::test]
    async fn oversized_backup_name_fails_without_changing_the_draft() {
        let root = tempfile::tempdir().unwrap();
        let draft = Config::development([
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ]);
        let path = root.path().join("c".repeat(250));
        let original = serde_json::to_vec(&draft).unwrap();
        private_io::write_new(&path, &original).unwrap();
        let error = run(registration(root.path(), path.clone()))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("draft was not changed"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn non_utf8_registration_path_is_rejected_before_any_write() {
        use std::os::unix::ffi::OsStringExt;
        let root = tempfile::tempdir().unwrap();
        let draft = Config::development([
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ]);
        let path = root
            .path()
            .join(std::ffi::OsString::from_vec(b"config-\xff.json".to_vec()));
        let original = serde_json::to_vec(&draft).unwrap();
        private_io::write_new(&path, &original).unwrap();
        let error = run(registration(root.path(), path.clone()))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("valid UTF-8"));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn repeated_registration_keeps_the_immediately_previous_draft() {
        let root = tempfile::tempdir().unwrap();
        let draft = Config::development([
            root.path().join("p"),
            root.path().join("b"),
            root.path().join("c"),
        ]);
        let path = root.path().join("config.json");
        private_io::write_new(&path, &serde_json::to_vec(&draft).unwrap()).unwrap();
        run(registration(root.path(), path.clone())).await.unwrap();
        let previous = std::fs::read(&path).unwrap();
        let mut args = registration(root.path(), path.clone());
        if let Command::RegisterDisk { slot, path, .. } = &mut args.command {
            *slot = "D02".into();
            *path = root.path().join("fifth");
        }
        run(args).await.unwrap();
        assert_eq!(
            std::fs::read(registration_backup(&path).unwrap()).unwrap(),
            previous
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
