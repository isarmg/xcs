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
mod tests {
    use super::*;
    #[test]
    fn clearing_receipts_retains_every_unresolved_and_failed_transfer() {
        let statuses = [
            SaveStatus::Waiting,
            SaveStatus::Uploading,
            SaveStatus::Saving,
            SaveStatus::Unknown,
            SaveStatus::Published,
            SaveStatus::Degraded,
            SaveStatus::Rejected,
        ];
        let mut model = Model {
            rows: statuses
                .into_iter()
                .map(|status| UploadRow {
                    status,
                    ..Default::default()
                })
                .collect(),
            filter: Some(SaveStatus::Unknown),
        };
        assert_eq!(model.count(None), 7);
        assert_eq!(model.count(model.filter), 1);
        model.clear_completed();
        assert_eq!(
            model.rows.iter().map(|row| row.status).collect::<Vec<_>>(),
            vec![
                SaveStatus::Waiting,
                SaveStatus::Uploading,
                SaveStatus::Saving,
                SaveStatus::Unknown,
                SaveStatus::Rejected
            ]
        );
        assert_eq!(model.count(Some(SaveStatus::Published)), 0);
    }
    #[test]
    fn only_unknown_original_operations_can_be_queried() {
        for status in [
            SaveStatus::Waiting,
            SaveStatus::Uploading,
            SaveStatus::Saving,
            SaveStatus::Unknown,
            SaveStatus::Published,
            SaveStatus::Degraded,
            SaveStatus::Rejected,
        ] {
            let mut row = UploadRow {
                operation_id: "original".into(),
                status,
                ..Default::default()
            };
            assert_eq!(row.queryable(), status == SaveStatus::Unknown);
            row.polling = true;
            assert!(!row.queryable());
            row.polling = false;
            row.operation_id.clear();
            assert!(!row.queryable());
        }
    }
    #[test]
    fn file_navigation_cannot_reset_the_session_transfer_ledger() {
        use crate::ui::{State, navigate};
        use web_interface::prelude::*;
        Owner::new().with(|| {
            let state = RwSignal::new(State::default());
            state.update(|s| {
                s.transfers.rows.push(UploadRow {
                    operation_id: "original".into(),
                    status: SaveStatus::Unknown,
                    ..Default::default()
                })
            });
            navigate(state, "transfers");
            navigate(state, "files");
            state.update(|s| s.files = Default::default());
            assert_eq!(
                state.with_untracked(|s| s.transfers.rows[0].operation_id.clone()),
                "original"
            );
        });
    }
}
