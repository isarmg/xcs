use super::*;
use std::sync::{
    Barrier,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn independent_payload_workers_are_bounded_and_file_order_is_serial() {
    let jobs: Vec<_> = (0..8).flat_map(|file| [(file, 0), (file, 1)]).collect();
    let groups: Vec<_> = jobs.chunks(2).map(|jobs| jobs.iter().collect()).collect();
    let progress: Vec<_> = (0..8).map(|_| AtomicUsize::new(0)).collect();
    let active = AtomicUsize::new(0);
    let maximum = AtomicUsize::new(0);
    let barrier = Barrier::new(4);
    run_payload_groups(&groups, |&(file, step)| {
        assert_eq!(progress[file].fetch_add(1, Ordering::SeqCst), step);
        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
        maximum.fetch_max(now, Ordering::SeqCst);
        barrier.wait();
        active.fetch_sub(1, Ordering::SeqCst);
        barrier.wait();
        Ok(())
    })
    .unwrap();
    assert_eq!(maximum.load(Ordering::SeqCst), 4);
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert!(
        progress
            .iter()
            .all(|count| count.load(Ordering::SeqCst) == 2)
    );
}
