use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use web_interface::prelude::*;

#[derive(Default)]
struct Transfer {
    bytes: u64,
    result: String,
}
#[derive(Default)]
struct Workspace {
    transfer: Transfer,
    branch: String,
}

#[test]
fn nested_transfer_slice_preserves_branch_and_projects_the_latest_result() {
    Owner::new().with(|| {
        let workspace = RwSignal::new(Workspace {
            branch: "main".into(),
            ..Default::default()
        });
        let (bytes, set_bytes) = slice!(workspace.transfer.bytes);
        let result = memo!(workspace.transfer.result);
        assert_eq!(bytes.get(), 0);
        set_bytes.set(4096);
        workspace.update(|state| state.transfer.result = "PUBLISHED".into());
        assert_eq!(bytes.get(), 4096);
        assert_eq!(result.get(), "PUBLISHED");
        assert_eq!(workspace.with(|state| state.branch.clone()), "main");
    });
}

#[test]
fn dropping_an_owner_runs_cleanup_for_browser_resources() {
    let released = Arc::new(AtomicBool::new(false));
    let owner = Owner::new();
    owner.with(|| {
        let released = released.clone();
        on_cleanup(move || released.store(true, Ordering::SeqCst));
    });
    assert!(!released.load(Ordering::SeqCst));
    drop(owner);
    assert!(released.load(Ordering::SeqCst));
}
