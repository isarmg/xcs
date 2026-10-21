use super::*;

#[test]
fn branch_registration_is_distinct_from_legacy_content_creation() {
    assert_eq!(action_label("file_branch_pending"), "Registered branch");
    assert_eq!(action_label("branch_pending"), "Registered branch");
    assert_eq!(action_label("file_branch"), "Created file branch");
    assert_eq!(
        action_label("directory.branch.rename"),
        "Renamed folder branch"
    );
    assert_eq!(
        action_label("directory.default.select"),
        "Switched SMB default folder branch"
    );
    assert_eq!(
        action_label("directory.branches.create"),
        "Registered folder branch"
    );
    assert_eq!(
        action_label("directory.branches.select"),
        "Switched folder branch"
    );
    assert_eq!(action_label("directory.tree.write"), "Wrote folder subtree");
}
