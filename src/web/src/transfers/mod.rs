//! Session-scoped upload ledger. File browsing never owns or resets this queue.
mod view;
use crate::files::SaveStatus;
pub(crate) use view::{summary, view};

#[derive(Clone, Default)]
pub(crate) struct Model {
    pub rows: Vec<UploadRow>,
    pub filter: Option<SaveStatus>,
}
impl Model {
    /// Dismiss successful receipts only. Errors retain their recovery details.
    /// This never calls a server API or removes a remote file.
    pub fn clear_completed(&mut self) {
        self.rows
            .retain(|row| !matches!(row.status, SaveStatus::Published | SaveStatus::Degraded));
    }
    pub fn count(&self, status: Option<SaveStatus>) -> usize {
        self.rows
            .iter()
            .filter(|row| status.is_none_or(|status| row.status == status))
            .count()
    }
}
#[derive(Clone, Default, PartialEq)]
pub(crate) struct UploadRow {
    pub operation_id: String,
    pub name: String,
    pub slot: String,
    pub path: String,
    pub branch_id: String,
    pub workspace: String,
    pub expected_head: String,
    pub status: SaveStatus,
    pub message: String,
    pub polling: bool,
    pub commit_attempted: bool,
    pub size: Option<u64>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub request_id: Option<String>,
}

pub(crate) fn now() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::new_0().to_iso_string().as_string()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}
impl UploadRow {
    pub fn queryable(&self) -> bool {
        self.status == SaveStatus::Unknown && !self.polling && !self.operation_id.is_empty()
    }
    pub fn settle_time(&mut self) {
        if !self.status.pending() && self.completed_at.is_none() {
            self.completed_at = now();
        }
    }
}

#[cfg(test)]
#[path = "../../../../test/web/unit/transfers/tests.rs"]
mod tests;
