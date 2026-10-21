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
