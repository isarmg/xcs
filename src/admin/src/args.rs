use command_line::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "zxs",
    version,
    about = "Local storage service, setup, administration and offline rescue",
    after_help = "Service/setup: config-template, init, init-auth, register-disk, serve.\nOffline rescue: restore-system, recovery-preview, recovery-commit, offline-check, offline-export, export-history, check-history-package, restore-history-package.\nOther commands are local administration; stop the service before opening its pool.\nNo remote client or ordinary file transfer commands. Use SMB for file contents."
)]
pub(crate) struct Args {
    #[arg(long, default_value = "./zxs-data", global = true)]
    pub(crate) root: PathBuf,
    #[arg(long, default_value = "admin", global = true)]
    pub(crate) principal: String,
    #[command(subcommand)]
    pub(crate) command: Command,
}
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Produce a configuration draft without accessing or initializing any device.
    #[command(display_order = 0)]
    ConfigTemplate {
        #[arg(long)]
        production: bool,
        #[arg(long, num_args = 3, required = true)]
        disks: Vec<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Explicitly initialize a new pool. Development disks simulate independent devices.
    #[command(display_order = 1)]
    Init {
        #[arg(long, conflicts_with = "development_disks")]
        config: Option<PathBuf>,
        #[arg(long, num_args = 3, conflicts_with = "config")]
        development_disks: Vec<PathBuf>,
        #[arg(long)]
        username: String,
        #[arg(long)]
        password_file: Option<PathBuf>,
    },
    /// Initialize credentials for an existing local pool that has no credential store.
    #[command(display_order = 2)]
    InitAuth {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password_file: Option<PathBuf>,
    },
    #[command(display_order = 5)]
    RestoreSystem {
        #[arg(long,num_args=2..,required=true)]
        disks: Vec<PathBuf>,
    },
    /// Inspect an already decided, unpublished operation using all original members.
    #[command(display_order = 6)]
    RecoveryPreview {
        operation: String,
        #[arg(long)]
        reason: String,
    },
    /// Publish only the previously reviewed recovery plan after explicit loss acknowledgement.
    #[command(display_order = 7)]
    RecoveryCommit {
        recovery: String,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        accept_permanent_loss: bool,
    },

    #[command(display_order = 29)]
    HistoricalSave {
        branch: String,
        source_commit: String,
        name: String,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        primary_slot: Option<String>,
        #[arg(long)]
        copies: Option<u32>,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 30)]
    PlanCommit {
        id: String,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 31)]
    PlanCancel {
        id: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 8)]
    OfflineCheck { disk: PathBuf },
    #[command(display_order = 9)]
    OfflineExport {
        disk: PathBuf,
        version: String,
        output: PathBuf,
    },
    #[command(display_order = 4)]
    Serve {
        #[arg(long, default_value = "127.0.0.1:7400")]
        listen: SocketAddr,
        #[arg(long)]
        external_origin: Option<String>,
        /// Trust this exact loopback proxy peer; it must overwrite X-Forwarded-For with one IP.
        #[arg(long = "trusted-proxy-ip", requires = "external_origin")]
        trusted_proxy_ips: Vec<std::net::IpAddr>,
        #[arg(long)]
        web_dir: Option<PathBuf>,
        #[arg(long, default_value_t = 1_073_741_824)]
        max_upload_bytes: u64,
        /// HTTP spool quota only, including retained generations and metadata allowance.
        #[arg(long, default_value_t = 107_374_182_400)]
        http_spool_bytes_per_account: u64,
        #[arg(long, default_value_t = 536_870_912_000)]
        http_spool_bytes_total: u64,
        /// Keep this filesystem headroom outside new upload reservations.
        #[arg(long, default_value_t = 1_073_741_824)]
        http_spool_recovery_reserve_bytes: u64,
        /// Independent capacity for each HTTP upload/read pool.
        #[arg(long, default_value_t = 8)]
        http_transfer_slots_total: usize,
        #[arg(long, default_value_t = 2)]
        http_transfer_slots_per_account: usize,
        #[arg(long, default_value_t = 60)]
        http_transfer_idle_seconds: u64,
        #[arg(long, default_value_t = 3600)]
        http_transfer_total_seconds: u64,
        #[arg(long)]
        smb_socket: Option<PathBuf>,
        #[arg(long, requires = "smb_socket")]
        smb_peer_uid: Option<u32>,
        #[arg(long, requires = "smb_socket")]
        smb_user: Vec<String>,
    },

    /// Add or revoke an account locally; changing grants revokes all of its existing sessions.
    #[command(display_order = 13)]
    Account {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password_file: Option<PathBuf>,
        #[arg(long)]
        admin: bool,
        #[arg(long)]
        disabled: bool,
        #[arg(long)]
        repository: Vec<String>,
    },
    /// Add a disk to an uninitialized JSON configuration draft. Does not migrate a live pool.
    #[command(display_order = 3)]
    RegisterDisk {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        slot: String,
        #[arg(long)]
        disk_id: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        level: u32,
        #[arg(long)]
        upstream: Option<String>,
        #[arg(long)]
        metadata_member: bool,
    },
    #[command(display_order = 14)]
    CreateRepo {
        name: String,
        #[arg(long)]
        primary_slot: String,
        #[arg(long, default_value_t = 3)]
        copies: u32,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 16)]
    Policy {
        id: String,
        repository: String,
        #[arg(long, default_value = "")]
        path_prefix: String,
        #[arg(long)]
        copies: u32,
        #[arg(long, default_value = "0")]
        expected_generation: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 17)]
    Rule {
        id: String,
        repository: String,
        #[arg(long, default_value = "")]
        path_prefix: String,
        #[arg(long)]
        primary_slot: String,
        #[arg(long, default_value = "0")]
        expected_generation: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 15)]
    SmbBinding {
        repository: String,
        branch: String,
        #[arg(long,action=command_line::ArgAction::Set,required=true)]
        enabled: bool,
        #[arg(long)]
        expected_history_generation: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 18)]
    Repositories,
    #[command(display_order = 19)]
    Disks,
    #[command(display_order = 20)]
    Topology,

    #[command(display_order = 21)]
    List { branch: String },
    #[command(display_order = 22)]
    History { file: String },
    #[command(display_order = 23)]
    Graph { history: String },
    #[command(display_order = 24)]
    Branch {
        source_commit: String,
        name: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 25)]
    Link {
        source_branch: String,
        source_commit: String,
        target_branch: String,
        target_commit: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 26)]
    Namespace {
        branch: String,
        kind: String,
        path: String,
        destination: Option<String>,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 27)]
    Purge {
        history: String,
        #[arg(required = true)]
        versions: Vec<String>,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 28)]
    Split {
        history: String,
        cut_commit: String,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        operation_id: Option<String>,
    },
    #[command(display_order = 32)]
    Operation { id: String },
    #[command(display_order = 33)]
    Check,

    /// Export a complete independently verifiable history package. Destination must not exist; its parent must be owned by this user and mode 0700.
    #[command(display_order = 10)]
    ExportHistory {
        history: String,
        destination: PathBuf,
    },
    #[command(display_order = 11)]
    CheckHistoryPackage { package: PathBuf },
    #[command(display_order = 12)]
    RestoreHistoryPackage {
        package: PathBuf,
        #[arg(long)]
        primary_slot: String,
        #[arg(long, default_value_t = 3)]
        copies: u32,
        #[arg(long)]
        operation_id: Option<String>,
    },
}

#[cfg(test)]
#[path = "../../../test/admin/unit/args/trusted_proxy_tests.rs"]
mod trusted_proxy_tests;
