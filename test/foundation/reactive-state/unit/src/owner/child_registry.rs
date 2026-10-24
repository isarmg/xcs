// Native, executor-free coverage for a stable list owner's child registry.
#[cfg(test)]
mod child_registry_regressions {
    use crate::owner::{provide_context, Owner};
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    fn tracked_child(parent: &Owner, sequence: usize, cleaned: &Arc<AtomicUsize>) -> Owner {
        // Cover both registration entry points without depending on a renderer.
        let child = if sequence % 2 == 0 {
            parent.with(Owner::new)
        } else {
            parent.child()
        };
        let cleaned = Arc::clone(cleaned);
        child.with(|| {
            provide_context(sequence);
            Owner::on_cleanup(move || {
                cleaned.fetch_add(1, Ordering::Relaxed);
            });
        });
        child
    }

    #[test]
    fn dropped_rows_do_not_grow_a_stable_parent_registry_without_bound() {
        const CHURN: usize = 4096;
        let parent = Owner::default();
        let cleaned = Arc::new(AtomicUsize::new(0));
        for sequence in 0..CHURN {
            drop(tracked_child(&parent, sequence, &cleaned));
        }
        // Resources really died. The bug is retained Weak allocations, not delayed cleanup.
        assert_eq!(cleaned.load(Ordering::Relaxed), CHURN);
        {
            let inner = parent.inner.read().unwrap();
            assert!(inner.children.iter().all(|child| child.strong_count() == 0));
            // Permit bounded lazy pruning rather than requiring immediate unlink on Drop.
            assert!(
                inner.children.len() <= 64,
                "{} dead registrations remain after {CHURN} completed row lifetimes",
                inner.children.len()
            );
        }
        parent.cleanup();
        assert!(parent.inner.read().unwrap().children.is_empty());
    }

    #[test]
    fn replacement_keeps_registry_bounded_and_live_children_in_order() {
        const LIVE: usize = 64;
        const CHURN: usize = 4096;
        let parent = Owner::default();
        let cleaned = Arc::new(AtomicUsize::new(0));
        let mut rows: VecDeque<_> = (0..LIVE)
            .map(|sequence| tracked_child(&parent, sequence, &cleaned))
            .collect();
        for sequence in LIVE..LIVE + CHURN {
            drop(rows.pop_front().unwrap());
            rows.push_back(tracked_child(&parent, sequence, &cleaned));
        }
        assert_eq!(cleaned.load(Ordering::Relaxed), CHURN);
        {
            let inner = parent.inner.read().unwrap();
            // Bound against peak concurrent rows, not total historical row lifetimes.
            assert!(
                inner.children.len() <= 4 * (LIVE + 1),
                "{} registrations remain for {LIVE} live rows after {CHURN} replacements",
                inner.children.len()
            );
            let registered: Vec<_> = inner
                .children
                .iter()
                .filter(|child| child.strong_count() > 0)
                .map(|child| child.as_ptr() as usize)
                .collect();
            let expected: Vec<_> = rows.iter().map(Owner::debug_id).collect();
            // Bidirectional context lookup must retain registration order.
            assert_eq!(registered, expected);
        }
        assert_eq!(parent.use_context_bidirectional::<usize>(), Some(CHURN));
        parent.pause();
        assert!(rows.iter().all(Owner::paused));
        parent.resume();
        assert!(rows.iter().all(|row| !row.paused()));
        parent.cleanup();
        assert_eq!(cleaned.load(Ordering::Relaxed), LIVE + CHURN);
        assert!(parent.inner.read().unwrap().children.is_empty());
        drop(rows);
        assert_eq!(cleaned.load(Ordering::Relaxed), LIVE + CHURN);
    }

    #[test]
    fn cleanup_resets_child_registry_watermark_for_reuse() {
        let parent = Owner::default();
        let rows: Vec<_> = (0..4096).map(|_| parent.child()).collect();
        parent.cleanup();
        drop(rows);
        for _ in 0..256 {
            drop(parent.child());
        }
        // A stale 4096-row sweep threshold would retain all 256 dead children.
        assert!(parent.inner.read().unwrap().children.len() <= 64);
    }

    #[test]
    fn pruning_work_is_linear_for_live_growth_and_replacement() {
        const LIVE: usize = 2048;
        const CHURN: usize = 16384;
        let parent = Owner::default();
        let mut rows: VecDeque<_> = (0..LIVE).map(|_| parent.child()).collect();
        assert!(parent.inner.read().unwrap().child_slots_scanned <= 2 * LIVE);
        for _ in 0..CHURN {
            drop(rows.pop_front().unwrap());
            rows.push_back(parent.child());
        }
        // Count inspected slots, not elapsed time. Scanning on every insertion
        // would be quadratic for initial growth and O(LIVE * CHURN) thereafter.
        let scanned = parent.inner.read().unwrap().child_slots_scanned;
        assert!(
            scanned <= 4 * (LIVE + CHURN),
            "scanned {scanned} child slots"
        );
    }
}
