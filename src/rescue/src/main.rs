//! Deliberately small offline rescue utility. Stop the daemon before using it.
mod configuration;
mod private_io;
use command_line::{Parser, Subcommand};
use json_codec::json;
use std::path::PathBuf;
use storage_engine::Engine;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Parser)]
#[command(
    name = "zxs-rescue",
    version,
    about = "Offline verified recovery; stop ZXS before use",
    after_help = "restore-system requires at least two trusted original metadata members.
Recovery preview/commit only accepts decided, unpublished operations with all original disks accessible. It requires a reviewed digest and explicit permanent-loss acknowledgement; it is not a reset or missing-disk replacement.
Offline export validates one published version, not the entire pool or current permissions."
)]
struct Args {
    #[arg(long, default_value = "./zxs-data", global = true)]
    root: PathBuf,
    #[arg(long, default_value = "admin", global = true)]
    principal: String,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Prepare or safely edit uninitialized topology drafts; never mutates a live pool.
    Configuration {
        #[command(subcommand)]
        command: configuration::Command,
    },
    RestoreSystem {
        #[arg(long,num_args=2..,required=true)]
        disks: Vec<PathBuf>,
    },
    RecoveryPreview {
        operation: String,
        #[arg(long)]
        reason: String,
    },
    RecoveryCommit {
        recovery: String,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        accept_permanent_loss: bool,
    },
    /// Export, verify or restore a complete independently validated history package.
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    OfflineCheck {
        disk: PathBuf,
    },
    OfflineExport {
        disk: PathBuf,
        version: String,
        output: PathBuf,
    },
}
#[derive(Subcommand)]
enum PackageCommand {
    /// Destination must not exist; parent must be owned by this user and mode 0700.
    Export {
        history: String,
        destination: PathBuf,
    },
    Check {
        package: PathBuf,
    },
    Restore {
        package: PathBuf,
        #[arg(long)]
        primary_slot: String,
        #[arg(long, default_value_t = 3)]
        copies: u32,
        #[arg(long)]
        operation_id: Option<String>,
    },
}
fn print(value: impl data_serialization::Serialize) -> Result<()> {
    println!("{}", json_codec::to_string_pretty(&value)?);
    Ok(())
}
fn main() {
    let args = Args::parse();
    if let Err(error) = run(args) {
        eprintln!("zxs-rescue: {error}");
        std::process::exit(1);
    }
}
fn run(args: Args) -> Result<()> {
    let root = args.root;
    match args.command {
        Command::Configuration { command } => configuration::run(command),
        Command::Package { command } => match command {
            PackageCommand::Check { package } => {
                print(storage_engine::offline_check_history_package(&package)?)
            }
            PackageCommand::Export {
                history,
                destination,
            } => {
                let engine = Engine::open(&root)?;
                print(engine.export_history_package(&history, &destination, &args.principal)?)
            }
            PackageCommand::Restore {
                package,
                primary_slot,
                copies,
                operation_id,
            } => {
                let mut engine = Engine::open(&root)?;
                let operation = operation_id.unwrap_or_else(|| storage_engine::new_id("op"));
                print(engine.restore_history_package(
                    &package,
                    &primary_slot,
                    copies,
                    &args.principal,
                    &operation,
                )?)
            }
        },
        Command::RestoreSystem { disks } => {
            let engine = Engine::restore_from_disks(&root, &disks)?;
            let pool = engine.config().pool_id.clone();
            let repositories = engine.state().repositories.len();
            let credentials_available = engine.auth_state().is_some();
            print(
                json!({"restored":true,"pool_id":pool,"repositories":repositories,"credential_recovery_snapshot_available":credentials_available}),
            )
        }
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
    }
}
