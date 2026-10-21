use crate::{
    html::attribute::{Attribute, AttributeValue},
    hydration::Cursor,
    renderer::Rndr,
    view::{
        add_attr::AddAnyAttr, Mountable, Position, PositionState, Render, RenderHtml, ToTemplate,
    },
};
use reactive_graph::effect::RenderEffect;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

/// Types for two way data binding.
pub mod bind;
mod class;
mod inner_html;
/// Provides a reactive [`NodeRef`](node_ref::NodeRef) type.
pub mod node_ref;
mod owned;
mod property;
mod style;
mod suspense;

pub use owned::*;
pub use suspense::*;

impl<F, V> ToTemplate for F
where
    F: ReactiveFunction<Output = V>,
    V: ToTemplate,
{
    const TEMPLATE: &'static str = V::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        // FIXME this seems wrong
        V::to_template(buf, class, style, inner_html, position)
    }
}

impl<F, V> Render for F
where
    F: ReactiveFunction<Output = V>,
    V: Render,
    V::State: 'static,
{
    type State = RenderEffectState<V::State>;

    #[track_caller]
    fn build(mut self) -> Self::State {
        let hook = throw_error::get_error_hook();
        RenderEffect::new(move |prev| {
            let _guard = hook
                .as_ref()
                .map(|h| throw_error::set_error_hook(Arc::clone(h)));
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.build()
            }
        })
        .into()
    }

    #[track_caller]
    fn rebuild(self, state: &mut Self::State) {
        let new = self.build();
        let mut old = std::mem::replace(state, new);
        old.insert_before_this(state);
        old.unmount();
    }
}

/// Retained view state for a [`RenderEffect`].
pub struct RenderEffectState<T: 'static>(Option<RenderEffect<T>>);

impl<T> From<RenderEffect<T>> for RenderEffectState<T> {
    fn from(value: RenderEffect<T>) -> Self {
        Self(Some(value))
    }
}

impl<T> Mountable for RenderEffectState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        if let Some(ref mut inner) = self.0 {
            inner.unmount();
        }
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        if let Some(ref mut inner) = self.0 {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        if let Some(inner) = &self.0 {
            inner.insert_before_this(child)
        } else {
            false
        }
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.0
            .as_ref()
            .map(|inner| inner.elements())
            .unwrap_or_default()
    }
}

impl<F, V> RenderHtml for F
where
    F: ReactiveFunction<Output = V>,
    V: RenderHtml + 'static,
    V::State: 'static,
{
    type Owned = Self;

    fn hydrate(mut self, cursor: &Cursor, position: &PositionState) -> Self::State {
        /// codegen optimisation:
        fn prep(
            cursor: &Cursor,
            position: &PositionState,
        ) -> (
            Cursor,
            PositionState,
            Option<Arc<dyn throw_error::ErrorHook>>,
        ) {
            let cursor = cursor.clone();
            let position = position.clone();
            let hook = throw_error::get_error_hook();
            (cursor, position, hook)
        }
        let (cursor, position, hook) = prep(cursor, position);

        RenderEffect::new(move |prev| {
            /// codegen optimisation:
            fn get_guard(
                hook: &Option<Arc<dyn throw_error::ErrorHook>>,
            ) -> Option<throw_error::ResetErrorHookOnDrop> {
                hook.as_ref()
                    .map(|h| throw_error::set_error_hook(Arc::clone(h)))
            }
            let _guard = get_guard(&hook);

            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&mut state);
                state
            } else {
                value.hydrate(&cursor, &position)
            }
        })
        .into()
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl<F, V> AddAnyAttr for F
where
    F: ReactiveFunction<Output = V>,
    V: RenderHtml + 'static,
{
    type Output<SomeNewAttr: Attribute> =
        Box<dyn FnMut() -> V::Output<SomeNewAttr::CloneableOwned> + Send>;

    fn add_any_attr<NewAttr: Attribute>(mut self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable_owned();
        Box::new(move || self.invoke().add_any_attr(attr.clone()))
    }
}

impl<M> Mountable for RenderEffect<M>
where
    M: Mountable + 'static,
{
    fn unmount(&mut self) {
        self.with_value_mut(|state| state.unmount());
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        self.with_value_mut(|state| {
            state.mount(parent, marker);
        });
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.with_value_mut(|value| value.insert_before_this(child))
            .unwrap_or(false)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.with_value_mut(|inner| inner.elements())
            .unwrap_or_default()
    }
}

impl<T> Drop for RenderEffectState<T> {
    fn drop(&mut self) {
        if let Some(effect) = self.0.take() {
            drop(effect.take_value());
            drop(effect);
        }
    }
}

impl<M, E> Mountable for Result<M, E>
where
    M: Mountable,
{
    fn unmount(&mut self) {
        if let Ok(ref mut inner) = self {
            inner.unmount();
        }
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        if let Ok(ref mut inner) = self {
            inner.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        if let Ok(inner) = &self {
            inner.insert_before_this(child)
        } else {
            false
        }
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.as_ref()
            .map(|inner| inner.elements())
            .unwrap_or_default()
    }
}

// Dynamic attributes
impl<F, V> AttributeValue for F
where
    F: ReactiveFunction<Output = V>,
    V: AttributeValue + 'static,
    V::State: 'static,
{
    type State = RenderEffect<V::State>;
    type Cloneable = SharedReactiveFunction<V>;
    type CloneableOwned = SharedReactiveFunction<V>;

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(mut self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        let key = Rndr::intern(key);
        let key = key.to_owned();
        let el = el.to_owned();

        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&key, &mut state);
                state
            } else {
                value.hydrate(&key, &el)
            }
        })
    }

    fn build(mut self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        let key = Rndr::intern(key);
        let key = key.to_owned();
        let el = el.to_owned();

        RenderEffect::new(move |prev| {
            let value = self.invoke();
            if let Some(mut state) = prev {
                value.rebuild(&key, &mut state);
                state
            } else {
                value.build(&el, &key)
            }
        })
    }

    fn rebuild(mut self, key: &str, state: &mut Self::State) {
        let key = Rndr::intern(key);
        let key = key.to_owned();
        let prev_value = state.take_value();

        *state = RenderEffect::new_with_value(
            move |prev| {
                let value = self.invoke();
                if let Some(mut state) = prev {
                    value.rebuild(&key, &mut state);
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

impl<V> AttributeValue for Suspend<V>
where
    V: AttributeValue + 'static,
    V::State: 'static,
{
    type State = Rc<RefCell<Option<V::State>>>;

    type Cloneable = ();
    type CloneableOwned = ();

    fn to_template(_key: &str, _buf: &mut String) {}

    fn hydrate(self, key: &str, el: &crate::renderer::types::Element) -> Self::State {
        let key = key.to_owned();
        let el = el.to_owned();
        let state = Rc::new(RefCell::new(None));
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(&state);
            async move {
                *state.borrow_mut() = Some(self.inner.await.hydrate(&key, &el));
                self.subscriber.forward();
            }
        });
        state
    }

    fn build(self, el: &crate::renderer::types::Element, key: &str) -> Self::State {
        let key = key.to_owned();
        let el = el.to_owned();
        let state = Rc::new(RefCell::new(None));
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(&state);
            async move {
                *state.borrow_mut() = Some(self.inner.await.build(&el, &key));
                self.subscriber.forward();
            }
        });
        state
    }

    fn rebuild(self, key: &str, state: &mut Self::State) {
        let key = key.to_owned();
        reactive_graph::spawn_local_scoped({
            let state = Rc::clone(state);
            async move {
                let value = self.inner.await;
                let mut state = state.borrow_mut();
                if let Some(state) = state.as_mut() {
                    value.rebuild(&key, state);
                }
                self.subscriber.forward();
            }
        });
    }

    fn into_cloneable(self) -> Self::Cloneable {
        #[cfg(feature = "tracing")]
        tracing::error!("Suspended attributes cannot be spread");
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        #[cfg(feature = "tracing")]
        tracing::error!("Suspended attributes cannot be spread");
    }
}

/// A reactive function that can be shared across multiple locations and across threads.
pub type SharedReactiveFunction<T> = Arc<Mutex<dyn FnMut() -> T + Send>>;

/// A reactive view function.
pub trait ReactiveFunction: Send + 'static {
    /// The return type of the function.
    type Output;

    /// Call the function.
    fn invoke(&mut self) -> Self::Output;

    /// Converts the function into a cloneable, shared type.
    fn into_shared(self) -> Arc<Mutex<dyn FnMut() -> Self::Output + Send>>;
}

impl<T: 'static> ReactiveFunction for Arc<Mutex<dyn FnMut() -> T + Send>> {
    type Output = T;

    fn invoke(&mut self) -> Self::Output {
        let mut fun = self.lock().expect("lock poisoned");
        fun()
    }

    fn into_shared(self) -> Arc<Mutex<dyn FnMut() -> Self::Output + Send>> {
        self
    }
}

impl<T: Send + Sync + 'static> ReactiveFunction for Arc<dyn Fn() -> T + Send + Sync> {
    type Output = T;

    fn invoke(&mut self) -> Self::Output {
        self()
    }

    fn into_shared(self) -> Arc<Mutex<dyn FnMut() -> Self::Output + Send>> {
        Arc::new(Mutex::new(move || self()))
    }
}

impl<F, T> ReactiveFunction for F
where
    F: FnMut() -> T + Send + 'static,
{
    type Output = T;

    fn invoke(&mut self) -> Self::Output {
        self()
    }

    fn into_shared(self) -> Arc<Mutex<dyn FnMut() -> Self::Output + Send>> {
        Arc::new(Mutex::new(self))
    }
}

macro_rules! reactive_impl {
    ($name:ident, <$($gen:ident),*>, $v:ty, $( $where_clause:tt )*) =>
    {
        #[allow(deprecated)]
        impl<$($gen),*> Render for $name<$($gen),*>
        where
            $v: Render + Clone + Send + Sync + 'static,
            <$v as Render>::State: 'static,
            $($where_clause)*
        {
            type State = RenderEffectState<<$v as Render>::State>;

            #[track_caller]
            fn build(self) -> Self::State {
                (move || self.get()).build()
            }

            #[track_caller]
            fn rebuild(self, state: &mut Self::State) {
                let new = self.build();
                let mut old = std::mem::replace(state, new);
                old.insert_before_this(state);
                old.unmount();
            }
        }

        #[allow(deprecated)]
        impl<$($gen),*> AddAnyAttr for $name<$($gen),*>
        where
            $v: RenderHtml + Clone + Send + Sync + 'static,
            <$v as Render>::State: 'static,
            $($where_clause)*
        {
            type Output<SomeNewAttr: Attribute> = Self;

            fn add_any_attr<NewAttr: Attribute>(
                self,
                _attr: NewAttr,
            ) -> Self::Output<NewAttr> {
                todo!()
            }
        }

        #[allow(deprecated)]
        impl<$($gen),*> RenderHtml for $name<$($gen),*>
        where
            $v: RenderHtml + Clone + Send + Sync + 'static,
            <$v as Render>::State: 'static,
            $($where_clause)*
        {

            type Owned = Self;

            fn hydrate(
                self,
                cursor: &Cursor,
                position: &PositionState,
            ) -> Self::State {
                (move || self.get())
                    .hydrate(cursor, position)
            }

            fn into_owned(self) -> Self::Owned {
                self
            }
        }

        #[allow(deprecated)]
        impl<$($gen),*> AttributeValue for $name<$($gen),*>
        where
            $v: AttributeValue + Send + Sync + Clone + 'static,
            <$v as AttributeValue>::State: 'static,
            $($where_clause)*
        {

            type State = RenderEffect<<$v as AttributeValue>::State>;
            type Cloneable = Self;
            type CloneableOwned = Self;

            fn to_template(_key: &str, _buf: &mut String) {}

            fn hydrate(
                self,
                key: &str,
                el: &crate::renderer::types::Element,
            ) -> Self::State {
                (move || self.get()).hydrate(key, el)
            }

            fn build(
                self,
                el: &crate::renderer::types::Element,
                key: &str,
            ) -> Self::State {
                (move || self.get()).build(el, key)
            }

            fn rebuild(self, key: &str, state: &mut Self::State) {
                (move || self.get()).rebuild(key, state)
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
    use super::RenderEffectState;
    use crate::{
        html::attribute::{Attribute, AttributeValue},
        hydration::Cursor,
        view::{add_attr::AddAnyAttr, Mountable, PositionState, Render, RenderHtml},
    };
    #[allow(deprecated)]
    use reactive_graph::wrappers::read::MaybeSignal;
    use reactive_graph::{
        computed::{ArcMemo, Memo},
        effect::RenderEffect,
        owner::Storage,
        signal::{ArcReadSignal, ArcRwSignal, ReadSignal, RwSignal},
        traits::Get,
        wrappers::read::{ArcSignal, MaybeProp, Signal, SignalTypes},
    };

    reactive_impl!(
        RwSignal,
        <V, S>,
        V,
        RwSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    reactive_impl!(
        ReadSignal,
        <V, S>,
        V,
        ReadSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    reactive_impl!(
        Memo,
        <V, S>,
        V,
        Memo<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    reactive_impl!(
        Signal,
        <V, S>,
        V,
        Signal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    reactive_impl!(
        MaybeSignal,
        <V, S>,
        V,
        MaybeSignal<V, S>: Get<Value = V>,
        S: Storage<V> + Storage<Option<V>>,
        S: Send + Sync + 'static,
    );
    reactive_impl!(
        MaybeProp,
        <V, S>,
        Option<V>,
        MaybeProp<V, S>: Get<Value = Option<V>>,
        S: Storage<Option<V>> + Storage<SignalTypes<Option<V>, S>>,
        S: Send + Sync + 'static,
    );
    reactive_impl!(ArcRwSignal, <V>, V, ArcRwSignal<V>: Get<Value = V>);
    reactive_impl!(ArcReadSignal, <V>, V, ArcReadSignal<V>: Get<Value = V>);
    reactive_impl!(ArcMemo, <V>, V, ArcMemo<V>: Get<Value = V>);
    reactive_impl!(ArcSignal, <V>, V, ArcSignal<V>: Get<Value = V>);
}

/*
#[cfg(test)]
mod tests {
    use crate::{
        html::element::{button, main, HtmlElement},
        renderer::mock_dom::MockDom,
        view::Render,
    };
    use leptos_reactive::{create_runtime, RwSignal, SignalGet, SignalSet};

    #[test]
    fn create_dynamic_element() {
        let rt = create_runtime();
        let count = RwSignal::new(0);
        let app: HtmlElement<_, _, _, MockDom> =
            button((), move || count.get().to_string());
        let el = app.build();
        assert_eq!(el.el.to_debug_html(), "<button>0</button>");
        rt.dispose();
    }

    #[test]
    fn update_dynamic_element() {
        let rt = create_runtime();
        let count = RwSignal::new(0);
        let app: HtmlElement<_, _, _, MockDom> =
            button((), move || count.get().to_string());
        let el = app.build();
        assert_eq!(el.el.to_debug_html(), "<button>0</button>");
        count.set(1);
        assert_eq!(el.el.to_debug_html(), "<button>1</button>");
        rt.dispose();
    }

    #[test]
    fn update_dynamic_element_among_siblings() {
        let rt = create_runtime();
        let count = RwSignal::new(0);
        let app: HtmlElement<_, _, _, MockDom> = main(
            (),
            button(
                (),
                ("Hello, my ", move || count.get().to_string(), " friends."),
            ),
        );
        let el = app.build();
        assert_eq!(
            el.el.to_debug_html(),
            "<main><button>Hello, my 0 friends.</button></main>"
        );
        count.set(42);
        assert_eq!(
            el.el.to_debug_html(),
            "<main><button>Hello, my 42 friends.</button></main>"
        );
        rt.dispose();
    }
}
 */
