use super::{ReactiveFunction, SharedReactiveFunction};
use crate::{html::class::IntoClass, renderer::Rndr};
use reactive_state::effect::RenderEffect;
use std::borrow::Borrow;

pub struct RenderEffectWithClassName<T>
where
    T: 'static,
{
    name: &'static str,
    effect: RenderEffect<T>,
}

impl<T> RenderEffectWithClassName<T>
where
    T: 'static,
{
    fn new(name: &'static str, effect: RenderEffect<T>) -> Self {
        Self { effect, name }
    }
}

impl<F, C> IntoClass for F
where
    F: ReactiveFunction<Output = C>,
    C: IntoClass + 'static,
    C::State: 'static,
{
    type State = RenderEffect<C::State>;
    type Cloneable = SharedReactiveFunction<C>;
    type CloneableOwned = SharedReactiveFunction<C>;

    fn hydrate(mut self, el: &crate::renderer::types::Element) -> Self::State {
        let el = el.clone();
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

impl<F, T> IntoClass for (&'static str, F)
where
    F: ReactiveFunction<Output = T>,
    T: Borrow<bool> + Send + 'static,
{
    type State = RenderEffectWithClassName<(crate::renderer::types::ClassList, bool)>;
    type Cloneable = (&'static str, SharedReactiveFunction<T>);
    type CloneableOwned = (&'static str, SharedReactiveFunction<T>);

    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, mut f) = self;
        let class_list = Rndr::class_list(el);
        let name = Rndr::intern(name);

        RenderEffectWithClassName::new(
            name,
            RenderEffect::new(
                move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
                    let include = *f.invoke().borrow();
                    if let Some((class_list, prev)) = prev {
                        if include {
                            if !prev {
                                Rndr::add_class(&class_list, name);
                            }
                        } else if prev {
                            Rndr::remove_class(&class_list, name);
                        }
                    }
                    (class_list.clone(), include)
                },
            ),
        )
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        let (name, mut f) = self;
        let class_list = Rndr::class_list(el);
        let name = Rndr::intern(name);

        RenderEffectWithClassName::new(
            name,
            RenderEffect::new(
                move |prev: Option<(crate::renderer::types::ClassList, bool)>| {
                    let include = *f.invoke().borrow();
                    match prev {
                        Some((class_list, prev)) => {
                            if include {
                                if !prev {
                                    Rndr::add_class(&class_list, name);
                                }
                            } else if prev {
                                Rndr::remove_class(&class_list, name);
                            }
                        }
                        None => {
                            if include {
                                Rndr::add_class(&class_list, name);
                            }
                        }
                    }
                    (class_list.clone(), include)
                },
            ),
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        let (name, mut f) = self;

        let prev_name = state.name;
        let prev_state = state.effect.take_value();
        if let Some((list, prev_include)) = &prev_state {
            if prev_name != name && *prev_include {
                Rndr::remove_class(list, prev_name);
            }
        }

        // Name might've updated:
        state.name = name;
        let mut first_run = true;
        state.effect = RenderEffect::new_with_value(
            move |prev| {
                let include = *f.invoke().borrow();
                match prev {
                    Some((class_list, prev)) => {
                        if include {
                            if !prev || first_run {
                                Rndr::add_class(&class_list, name);
                            }
                        } else if prev {
                            Rndr::remove_class(&class_list, name);
                        }
                        first_run = false;
                        (class_list.clone(), include)
                    }
                    None => {
                        unreachable!()
                    }
                }
            },
            prev_state,
        );
    }

    fn into_cloneable(self) -> Self::Cloneable {
        (self.0, self.1.into_shared())
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        (self.0, self.1.into_shared())
    }

    fn reset(state: &mut Self::State) {
        let name = state.name;
        state.effect = RenderEffect::new_with_value(
            move |prev| {
                if let Some(mut state) = prev {
                    let (class_list, prev) = &mut state;
                    Rndr::remove_class(class_list, name);
                    *prev = false;
                    state
                } else {
                    unreachable!()
                }
            },
            state.effect.take_value(),
        );
    }
}

// TODO this needs a non-reactive form too to be restored

macro_rules!  tuple_class_reactive {
    ($name:ident, <$($impl_gen:ident),*>, <$($gen:ident),*>, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($impl_gen),*>  IntoClass for (&'static str, $name<$($gen),*>)
        where
            $($where_clause)*
        {

            type State = RenderEffectWithClassName<(
                crate::renderer::types::ClassList,
                bool,
            )>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn hydrate(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                IntoClass::hydrate(
                    (self.0, move || self.1.get()),
                    el,
                )
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                IntoClass::build((self.0, move || self.1.get()), el)
            }

            fn rebuild(self, state: &mut Self::State) {
                IntoClass::rebuild((self.0, move || self.1.get()), state)
            }

            fn into_cloneable(self) -> Self::Cloneable {
                self
            }

            fn into_cloneable_owned(self) -> Self::CloneableOwned {
                self
            }

            fn reset(state: &mut Self::State) {
                let name = state.name;
                *state = RenderEffectWithClassName::new(
                    state.name,
                    RenderEffect::new_with_value(
                        move |prev| {
                            if let Some(mut state) = prev {
                                let (class_list, prev) = &mut state;
                                Rndr::remove_class(class_list, name);
                                *prev = false;
                                state
                            } else {
                                unreachable!()
                            }
                        },
                        state.effect.take_value(),
                    ),
                );
            }
        }
    };
}

macro_rules!  class_reactive {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> IntoClass for $name<$($gen),*>
        where
            $v: IntoClass + Clone + Send + Sync + 'static,
            <$v as IntoClass>::State: 'static,
            $($where_clause)*
        {

            type State = RenderEffect<<$v as IntoClass>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;

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
    };
}

mod stable {
    use super::{RenderEffect, RenderEffectWithClassName};
    use crate::{html::class::IntoClass, renderer::Rndr};
    #[allow(deprecated)]
    use reactive_state::wrappers::read::MaybeSignal;
    use reactive_state::{
        computed::{ArcMemo, Memo},
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, Signal},
    };
    class_reactive!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    class_reactive!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    class_reactive!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    class_reactive!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    class_reactive!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);

    tuple_class_reactive!(
        RwSignal,
        <S>,
        <bool, S>,
        RwSignal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send  + 'static,
    );
    tuple_class_reactive!(
        ReadSignal,
        <S>,
        <bool, S>,
        ReadSignal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(
        Memo,
        <S>,
        <bool, S>,
        Memo<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(
        Signal,
        <S>,
        <bool, S>,
        Signal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(
        MaybeSignal,
        <S>,
        <bool, S>,
        MaybeSignal<bool, S>: Get<Value = bool>,
        S: Storage<bool>,
        S: Send + 'static,
    );
    tuple_class_reactive!(ArcRwSignal,<>, <bool>, ArcRwSignal<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcReadSignal,<>, <bool>, ArcReadSignal<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcMemo,<>, <bool>, ArcMemo<bool>: Get<Value = bool>);
    tuple_class_reactive!(ArcSignal,<>, <bool>, ArcSignal<bool>: Get<Value = bool>);
}
