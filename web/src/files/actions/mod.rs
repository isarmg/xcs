//! File-manager actions. Views call this facade; private modules separate reads,
//! mutations, publication recovery and content transfer without owning UI state.
mod browse;
mod mutation;
mod operation;
mod transfer;

pub(crate) use browse::{go_directory, go_root, load_disks, load_entries, open_disk, select_entry};
pub(crate) use mutation::{
    create_branch, directory_branch, namespace, rename_directory_branch, rename_file_branch,
    select_branch, select_default_branch, update_node_note, update_note,
};
pub(crate) use operation::{poll, record_operation, record_receipt};
pub(crate) use transfer::{download, preview, upload_contents, upload_files};

use super::model::{SaveStatus, operation_status};
use crate::{api, ui::State};

fn disk_path(slot: &str) -> String {
    format!("/v1/file-manager/disks/{}", api::segment(slot))
}
pub(crate) fn pending(state: &State) -> bool {
    state.files.mutation_pending
        || state.files.uploads.iter().any(|row| row.status.pending())
        || state
            .files
            .operations
            .iter()
            .any(|op| operation_status(op) == SaveStatus::Unknown)
}
