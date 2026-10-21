#[cfg(test)]
mod tests {

    use crate::{
        into_reactive_value::IntoReactiveValue,
        owner::{LocalStorage, Owner},
        traits::GetUntracked,
        wrappers::read::Signal,
    };
    use typed_builder::TypedBuilder;

    #[test]
    fn test_into_signal_compiles() {
        let owner = Owner::new();
        owner.set();

        #[cfg(not(all(feature = "nightly", rustc_nightly)))]
        let _: Signal<usize> = (|| 2).into_reactive_value();
        let _: Signal<usize, LocalStorage> = 2.into_reactive_value();
        #[cfg(not(all(feature = "nightly", rustc_nightly)))]
        let _: Signal<usize, LocalStorage> = (|| 2).into_reactive_value();
        let _: Signal<String> = "str".into_reactive_value();
        let _: Signal<String, LocalStorage> = "str".into_reactive_value();

        #[derive(TypedBuilder)]
        struct Foo {
            #[builder(setter(
                fn transform<M>(value: impl IntoReactiveValue<Signal<usize>, M>) {
                    value.into_reactive_value()
                }
            ))]
            sig: Signal<usize>,
        }

        assert_eq!(Foo::builder().sig(2).build().sig.get_untracked(), 2);
        #[cfg(not(all(feature = "nightly", rustc_nightly)))]
        assert_eq!(Foo::builder().sig(|| 2).build().sig.get_untracked(), 2);
        assert_eq!(
            Foo::builder()
                .sig(Signal::stored(2))
                .build()
                .sig
                .get_untracked(),
            2
        );
    }
}
