use super::*;
#[test]
fn terminal_outcomes_cannot_be_overwritten() {
    assert_eq!(Phase::Pending.settle(Phase::Ready), Some(Phase::Ready));
    assert_eq!(Phase::Pending.settle(Phase::Failed), Some(Phase::Failed));
    assert_eq!(Phase::Failed.settle(Phase::Ready), None);
    assert_eq!(Phase::Ready.settle(Phase::Failed), None);
    assert_eq!(Phase::Pending.settle(Phase::Pending), None);
}
#[test]
fn startup_attributes_have_a_closed_vocabulary() {
    assert_eq!(Phase::from_attribute(None), Some(Phase::Pending));
    assert_eq!(Phase::from_attribute(Some("failed")), Some(Phase::Failed));
    assert_eq!(Phase::from_attribute(Some("other")), None);
    assert_eq!(TIMEOUT_MS, 30_000);
}
