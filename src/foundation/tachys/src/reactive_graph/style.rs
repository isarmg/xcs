use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{
    html::style::{IntoStyle, IntoStyleValue},
    renderer::Rndr,
};
use reactive_graph::effect::RenderEffect;
use std::sync::Arc;

impl<F, S> IntoStyleValue for F
where
    F: ReactiveFunction<Output = S>,
    S: IntoStyleValue + 'static,
{
    type State = (Arc<str>, RenderEffect<S::State>);
    type Cloneable = SharedReactiveFunction<S>;
    type CloneableOwned = SharedReactiveFunction<S>;

    fn build(
        mut self,
        style: &crate::renderer::dom::CssStyleDeclaration,
        name: &str,
    ) -> Self::State {
        let name: Arc<str> = Rndr::intern(name).into();
        let style = style.to_owned();
        (
            Arc::clone(&name),
            RenderEffect::new(move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&style, &name, &mut state);
                    state
                } else {
                    value.build(&style, &name)
                }
            }),
        )
    }

    fn rebuild(
        mut self,
        style: &crate::renderer::dom::CssStyleDeclaration,
        name: &str,
        state: &mut Self::State,
    ) {
        let (prev_name, prev_effect) = state;
        let mut prev_value = prev_effect.take_value();
        if name != prev_name.as_ref() {
            Rndr::remove_css_property(style, prev_name.as_ref());
            prev_value = None;
        }
        let name: Arc<str> = name.into();
        let style = style.to_owned();

        *state = (
            Arc::clone(&name),
            RenderEffect::new_with_value(
                move |prev| {
                    let value = self.invoke();
                    if let Some(mut state) = prev {
                        value.rebuild(&style, &name, &mut state);
                        state
                    } else {
                        value.build(&style, &name)
                    }
                },
                prev_value,
            ),
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self.into_shared()
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self.into_shared()
    }
}

impl<F, C> IntoStyle for F
where
    F: ReactiveFunction<Output = C>,
    C: IntoStyle + 'static,
    C::State: 'static,
{
    type State = RenderEffect<C::State>;
    type Cloneable = SharedReactiveFunction<C>;
    type CloneableOwned = SharedReactiveFunction<C>;

    fn build(mut self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.clone();
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

    fn reset(state: &mut Self::State) {
        *state = RenderEffect::new_with_value(
            move |prev| {
                if let Some(mut state) = prev {
                    C::reset(&mut state);
                    state
                } else {
                    unreachable!()
                }
            },
            state.take_value(),
        );
    }
}

macro_rules! style_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> IntoStyle for $name<$($gen),*>
        where
            $v: IntoStyle + Clone + Send + Sync + 'static,
            <$v as IntoStyle>::State: 'static,
            $($where_clause)*
        {

            type State = RenderEffect<<$v as IntoStyle>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;


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

            fn reset(state: &mut Self::State) {
                *state = RenderEffect::new_with_value(
                    move |prev| {
                        if let Some(mut state) = prev {
                            <$v>::reset(&mut state);
                            state
                        } else {
                            unreachable!()
                        }
                    },
                    state.take_value(),
                );
            }
        }

        #[allow(deprecated)]
        impl<$($gen),*> IntoStyleValue for $name<$($gen),*>
        where
            $v: IntoStyleValue + Send + Sync + Clone + 'static,
            $($where_clause)*
        {

            type State = (Arc<str>, RenderEffect<<$v as IntoStyleValue>::State>);
            type Cloneable = $name<$($gen),*>;
            type CloneableOwned = $name<$($gen),*>;

            fn build(
                self,
                style: &crate::renderer::dom::CssStyleDeclaration,
                name: &str,
            ) -> Self::State {
                IntoStyleValue::build(move || self.get(), style, name)
            }

            fn rebuild(
                self,
                style: &crate::renderer::dom::CssStyleDeclaration,
                name: &str,
                state: &mut Self::State,
            ) {
                IntoStyleValue::rebuild(
                    move || self.get(),
                    style,
                    name,
                    state,
                )
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
    use super::RenderEffect;
    use crate::html::style::{IntoStyle, IntoStyleValue};
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    use std::sync::Arc;

    style_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    style_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    style_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    style_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    style_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}
