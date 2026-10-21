use super::{ReactiveFunction, SharedReactiveFunction};
use crate::html::element::InnerHtmlValue;
use reactive_graph::effect::RenderEffect;

impl<F, V> InnerHtmlValue for F
where
    F: ReactiveFunction<Output = V>,
    V: InnerHtmlValue + 'static,
    V::State: 'static,
{
    type State = RenderEffect<V::State>;
    type Cloneable = SharedReactiveFunction<V>;
    type CloneableOwned = SharedReactiveFunction<V>;

    fn to_template(_buf: &mut String) {}

    fn hydrate(mut self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.hydrate(&el)
            }
        })
    }

    fn build(mut self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.to_owned();
        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build(&el)
            }
        })
    }

    fn rebuild(mut self, state: &mut Self::State) {
        let prev_value = state.take_value();
        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&mut state);
                    state
                } else {
                    unreachable!()
                }
            },
            prev_value,
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_shared()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into_shared()
    }
}

macro_rules! inner_html_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> InnerHtmlValue for $name<$($gen),*>
        where
            $v: InnerHtmlValue + Clone + Send + Sync + 'static,
            <$v as InnerHtmlValue>::State: 'static,
            $($where_clause)*
        {

            type State = RenderEffect<<$v as InnerHtmlValue>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn to_template(_buf: &mut String) {}

            fn hydrate(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                (move || self.get()).hydrate(el)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                (move || self.get()).build(el)
            }

            fn rebuild(self, state: &mut Self::State) {
                (move || self.get()).rebuild(state)
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }

        }
    };
}

mod stable {
    use crate::html::element::InnerHtmlValue;
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };

    inner_html_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    inner_html_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    inner_html_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    inner_html_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    inner_html_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}
