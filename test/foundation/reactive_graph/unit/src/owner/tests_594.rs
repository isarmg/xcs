#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        owner::{on_cleanup, Owner, StoredValue},
        signal::signal,
        traits::{Dispose, GetValue, Set},
    };
    use std::{sync::mpsc, thread, time::Duration};

    fn does_not_deadlock(f: impl FnOnce() + Send + 'static) -> bool {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            f();
            _ = tx.send(());
        });
        rx.recv_timeout(Duration::from_secs(5)).is_ok()
    }

    fn dispose_through_arena(
        root: &Owner,
        setup: impl FnOnce(),
        dispose: impl FnOnce(Owner),
    ) {
        let (inner, outer) = root.with(|| (Owner::new(), Owner::new()));

        inner.with(setup);
        outer.with(|| {
            _ = StoredValue::new(inner);
        });

        dispose(outer);
    }

    #[test]
    fn dropping_owner_held_in_arena_removes_its_nodes() {
        assert!(does_not_deadlock(|| {
            let root = Owner::new();
            dispose_through_arena(
                &root,
                || {
                    _ = signal(0);
                },
                drop,
            );
        }));
    }

    #[test]
    fn dropping_owner_held_in_arena_runs_cleanups_accessing_arena() {
        assert!(does_not_deadlock(|| {
            let root = Owner::new();
            let value = root.with(|| StoredValue::new(0));

            dispose_through_arena(
                &root,
                move || {
                    let (_, set_count) = signal(0);
                    on_cleanup(move || {
                        _ = value.get_value();
                        set_count.set(1);
                    });
                },
                drop,
            );
        }));
    }

    #[test]
    fn cleaning_up_owner_holding_owner_in_arena() {
        assert!(does_not_deadlock(|| {
            let root = Owner::new();
            dispose_through_arena(
                &root,
                || {
                    _ = signal(0);
                },
                |outer| outer.cleanup(),
            );
        }));
    }
}
