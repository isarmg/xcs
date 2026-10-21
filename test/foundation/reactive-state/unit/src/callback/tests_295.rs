#[cfg(test)]
mod tests {
    use super::Callable;
    use crate::{
        callback::{Callback, UnsyncCallback},
        owner::Owner,
        traits::Dispose,
        IntoReactiveValue,
    };

    struct NoClone {}

    #[test]
    fn clone_callback() {
        let owner = Owner::new();
        owner.set();

        let callback = Callback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback;
    }

    #[test]
    fn clone_unsync_callback() {
        let owner = Owner::new();
        owner.set();

        let callback =
            UnsyncCallback::new(move |_no_clone: NoClone| NoClone {});
        let _cloned = callback;
    }

    #[test]
    fn runback_from() {
        let owner = Owner::new();
        owner.set();

        let _callback: Callback<(), String> = (|| "test").into();
        let _callback: Callback<(i32, String), String> =
            (|num, s| format!("{num} {s}")).into();
        // Single params should work without needing the (foo,) tuple using IntoReactiveValue:
        let _callback: Callback<usize, &'static str> =
            (|_usize| "test").into_reactive_value();
        let _callback: Callback<usize, String> =
            (|_usize| "test").into_reactive_value();
    }

    #[test]
    fn sync_callback_from() {
        let owner = Owner::new();
        owner.set();

        let _callback: UnsyncCallback<(), String> = (|| "test").into();
        let _callback: UnsyncCallback<(i32, String), String> =
            (|num, s| format!("{num} {s}")).into();
        // Single params should work without needing the (foo,) tuple using IntoReactiveValue:
        let _callback: UnsyncCallback<usize, &'static str> =
            (|_usize| "test").into_reactive_value();
        let _callback: UnsyncCallback<usize, String> =
            (|_usize| "test").into_reactive_value();
    }

    #[test]
    fn sync_callback_try_run() {
        let owner = Owner::new();
        owner.set();

        let callback = Callback::new(move |arg| arg);
        assert_eq!(callback.try_run((0,)), Some((0,)));
        callback.dispose();
        assert_eq!(callback.try_run((0,)), None);
    }

    #[test]
    fn unsync_callback_try_run() {
        let owner = Owner::new();
        owner.set();

        let callback = UnsyncCallback::new(move |arg| arg);
        assert_eq!(callback.try_run((0,)), Some((0,)));
        callback.dispose();
        assert_eq!(callback.try_run((0,)), None);
    }

    #[test]
    fn callback_matches_same() {
        let owner = Owner::new();
        owner.set();

        let callback1 = Callback::new(|x: i32| x * 2);
        let callback2 = callback1;
        assert!(callback1.matches(&callback2));
    }

    #[test]
    fn callback_matches_different() {
        let owner = Owner::new();
        owner.set();

        let callback1 = Callback::new(|x: i32| x * 2);
        let callback2 = Callback::new(|x: i32| x + 1);
        assert!(!callback1.matches(&callback2));
    }

    #[test]
    fn unsync_callback_matches_same() {
        let owner = Owner::new();
        owner.set();

        let callback1 = UnsyncCallback::new(|x: i32| x * 2);
        let callback2 = callback1;
        assert!(callback1.matches(&callback2));
    }

    #[test]
    fn unsync_callback_matches_different() {
        let owner = Owner::new();
        owner.set();

        let callback1 = UnsyncCallback::new(|x: i32| x * 2);
        let callback2 = UnsyncCallback::new(|x: i32| x + 1);
        assert!(!callback1.matches(&callback2));
    }
}
