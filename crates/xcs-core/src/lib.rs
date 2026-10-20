//! Single-writer full-file storage, durable publication, and offline recovery.
#![forbid(unsafe_code)]
mod business;
mod container_storage;
mod degraded_recovery;
mod disk;
mod engine;
mod historical_save;
mod history_package;
mod journal;
mod maintenance;
mod metadata_budget;
mod protection;
mod repair;
mod wire;
pub use degraded_recovery::RecoveryPlan;
pub use disk::{offline_check, offline_export};
pub use engine::{
    ContentReadPlan, Engine, OpenedContent, RangeContent, ReadLease, WorkspaceReadPlan,
};
pub use historical_save::HistoricalSaveOptions;
pub use history_package::{HistoryManifest, HistoryPackageReport, offline_check_history_package};
pub use wire::{commit_request_digest, manifest_hash as workspace_manifest_hash};
pub use xcs_contract::*;
#[cfg(test)]
mod tests;
