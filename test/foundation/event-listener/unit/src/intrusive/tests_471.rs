#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::pin;

    #[cfg(target_family = "wasm")]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    macro_rules! make_listeners {
        ($($id:ident),*) => {
            $(
                let $id = Option::<Listener<()>>::None;
                pin!($id);
            )*
        };
    }

    #[test]
    fn insert() {
        let inner = crate::Inner::new();
        make_listeners!(listen1, listen2, listen3);

        // Register the listeners.
        inner.insert(listen1.as_mut());
        inner.insert(listen2.as_mut());
        inner.insert(listen3.as_mut());

        assert_eq!(inner.list.try_total_listeners(), Some(3));

        // Remove one.
        assert_eq!(inner.remove(listen2, false), Some(State::Created));
        assert_eq!(inner.list.try_total_listeners(), Some(2));

        // Remove another.
        assert_eq!(inner.remove(listen1, false), Some(State::Created));
        assert_eq!(inner.list.try_total_listeners(), Some(1));
    }

    #[test]
    fn drop_non_notified() {
        let inner = crate::Inner::new();
        make_listeners!(listen1, listen2, listen3);

        // Register the listeners.
        inner.insert(listen1.as_mut());
        inner.insert(listen2.as_mut());
        inner.insert(listen3.as_mut());

        // Notify one.
        inner.notify(GenericNotify::new(1, false, || ()));

        // Remove one.
        inner.remove(listen3, true);

        // Remove the rest.
        inner.remove(listen1, true);
        inner.remove(listen2, true);
    }
}
