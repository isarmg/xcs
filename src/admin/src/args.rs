use clap::{Parser, Subcommand};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Parser)]
#[command(
    name = "zxs",
    version,
    about = "Full-file versioned storage and recovery"
)]
pub(crate) struct Args {
    #[arg(long, default_value = "./zxs-data", global = true)]
    pub(crate) root: PathBuf,
    #[arg(long, default_value = "admin", global = true)]
    pub(crate) principal: String,
    /// Use the authenticated remote API instead of opening a local pool.
    #[arg(long, global = true)]
    pub(crate) server: Option<String>,
    /// Read a bearer token from a private file. Tokens are never accepted on the command line.
    #[arg(long, global = true)]
    pub(crate) token_file: Option<PathBuf>,
    #[command(subcommand)]
    pub(crate) command: Command,
}
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Produce a configuration draft without accessing or initializing any device.
    ConfigTemplate {
        #[arg(long)]
        production: bool,
        #[arg(long, num_args = 3, required = true)]
        disks: Vec<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Explicitly initialize a new pool. Development disks simulate independent devices.
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
    InitAuth {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password_file: Option<PathBuf>,
    },
    RestoreSystem {
        #[arg(long,num_args=2..,required=true)]
        disks: Vec<PathBuf>,
    },
    /// Inspect an already decided, unpublished operation using all original members.
    RecoveryPreview {
        operation: String,
        #[arg(long)]
        reason: String,
    },
    /// Publish only the previously reviewed recovery plan after explicit loss acknowledgement.
    RecoveryCommit {
        recovery: String,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        accept_permanent_loss: bool,
    },
    RetryCommit {
        request_file: PathBuf,
    },
    HistoricalSave {
        branch: String,
        source_commit: String,
        name: String,
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long, requires = "path")]
        source: Option<PathBuf>,
        #[arg(long, requires = "source")]
        path: Option<String>,
        #[arg(long)]
        primary_slot: Option<String>,
        #[arg(long)]
        copies: Option<u32>,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        operation_id: Option<String>,
    },
    PlanCommit {
        id: String,
        #[arg(long)]
        digest: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    PlanCancel {
        id: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    OfflineCheck {
        disk: PathBuf,
    },
    OfflineExport {
        disk: PathBuf,
        version: String,
        output: PathBuf,
    },
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
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password_file: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    Logout,
    Whoami,
    /// Add or revoke an account locally; changing grants revokes all of its existing sessions.
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
    CreateRepo {
        name: String,
        #[arg(long)]
        primary_slot: String,
        #[arg(long, default_value_t = 3)]
        copies: u32,
        #[arg(long)]
        operation_id: Option<String>,
    },
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
    SmbBinding {
        repository: String,
        branch: String,
        #[arg(long,action=clap::ArgAction::Set,required=true)]
        enabled: bool,
        #[arg(long)]
        expected_history_generation: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Repositories,
    Disks,
    Topology,
    Put {
        branch: String,
        path: String,
        source: PathBuf,
        #[arg(long)]
        expected_head: Option<String>,
        #[arg(long)]
        message: Option<String>,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Get {
        version: String,
        output: PathBuf,
    },
    List {
        branch: String,
    },
    History {
        file: String,
    },
    Graph {
        history: String,
    },
    Branch {
        source_commit: String,
        name: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Link {
        source_branch: String,
        source_commit: String,
        target_branch: String,
        target_commit: String,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Namespace {
        branch: String,
        kind: String,
        path: String,
        destination: Option<String>,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Purge {
        history: String,
        #[arg(required = true)]
        versions: Vec<String>,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Split {
        history: String,
        cut_commit: String,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        operation_id: Option<String>,
    },
    Operation {
        id: String,
    },
    Check,
    Export {
        version: String,
        output: PathBuf,
    },
    /// Export a complete independently verifiable history package. Destination must not exist; its parent must be owned by this user and mode 0700.
    ExportHistory {
        history: String,
        destination: PathBuf,
    },
    CheckHistoryPackage {
        package: PathBuf,
    },
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
mod trusted_proxy_tests {
    use super::*;

    #[test]
    fn trusted_proxy_is_explicit_repeatable_and_requires_origin() {
        let args = Args::try_parse_from(["zxs", "serve"]).unwrap();
        let Command::Serve {
            trusted_proxy_ips, ..
        } = args.command
        else {
            panic!("serve expected")
        };
        assert!(trusted_proxy_ips.is_empty());
        assert!(Args::try_parse_from(["zxs", "serve", "--trusted-proxy-ip", "127.0.0.1"]).is_err());
        assert!(
            Args::try_parse_from([
                "zxs",
                "serve",
                "--external-origin",
                "https://storage.example.org",
                "--trusted-proxy-ip",
                "127.0.0.0/8"
            ])
            .is_err()
        );
        let args = Args::try_parse_from([
            "zxs",
            "serve",
            "--external-origin",
            "https://storage.example.org",
            "--trusted-proxy-ip",
            "127.0.0.1",
            "--trusted-proxy-ip",
            "::1",
        ])
        .unwrap();
        let Command::Serve {
            trusted_proxy_ips, ..
        } = args.command
        else {
            panic!("serve expected")
        };
        assert_eq!(
            trusted_proxy_ips,
            vec![
                "127.0.0.1".parse::<std::net::IpAddr>().unwrap(),
                "::1".parse().unwrap()
            ]
        );
    }
}
