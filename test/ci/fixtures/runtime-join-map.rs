use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::adapters::task::JoinMap;

struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn keyed_tasks_complete_with_original_keys() {
    let mut tasks = JoinMap::new();
    tasks.spawn("one", async { 1 });
    tasks.spawn("two", async { 2 });
    assert_eq!(tasks.len(), 2);
    assert!(tasks.contains_key("one"));
    let mut out = Vec::new();
    while let Some((key, value)) = tasks.join_next().await {
        out.push((key, value.unwrap()));
    }
    out.sort();
    assert_eq!(out, vec![("one", 1), ("two", 2)]);
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn abort_by_key_preserves_key_and_drops_task() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mark = DropCounter(drops.clone());
    let mut tasks: JoinMap<&str, usize> = JoinMap::new();
    tasks.spawn("pending", async move {
        let _mark = mark;
        std::future::pending().await
    });
    assert!(!tasks.abort("missing"));
    assert!(tasks.abort("pending"));
    let (key, result) = tasks.join_next().await.unwrap();
    assert_eq!(key, "pending");
    assert!(result.unwrap_err().is_cancelled());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn replacing_key_aborts_original_and_keeps_new_value() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mark = DropCounter(drops.clone());
    let mut tasks = JoinMap::new();
    tasks.spawn("same", async move {
        let _mark = mark;
        std::future::pending::<usize>().await
    });
    tasks.spawn("same", async { 7 });
    assert_eq!(tasks.len(), 1);
    let (key, value) = tasks.join_next().await.unwrap();
    assert_eq!(key, "same");
    assert_eq!(value.unwrap(), 7);
    assert!(tasks.join_next().await.is_none());
    tokio::task::yield_now().await;
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn shutdown_cancels_all_and_empty_is_reusable() {
    let mut tasks: JoinMap<u8, u8> = JoinMap::new();
    tasks.spawn(1, std::future::pending());
    tasks.spawn(2, std::future::pending());
    tasks.shutdown().await;
    assert!(tasks.is_empty());
    tasks.spawn(3, async { 9 });
    let (key, value) = tasks.join_next().await.unwrap();
    assert_eq!((key, value.unwrap()), (3, 9));
}

#[tokio::test]
async fn task_panic_is_reported_with_key() {
    let mut tasks: JoinMap<&str, usize> = JoinMap::new();
    tasks.spawn("panic", async { panic!("expected") });
    let (key, value) = tasks.join_next().await.unwrap();
    assert_eq!(key, "panic");
    assert!(value.unwrap_err().is_panic());
}
