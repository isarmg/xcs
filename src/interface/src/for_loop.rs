use crate::into_view::IntoView;
use reactive_graph::{
    owner::Owner,
    signal::{ArcRwSignal, ReadSignal},
    traits::Set,
};
use std::hash::Hash;
use tachys::{reactive_graph::OwnedView, view::keyed::keyed};
use view_macros::component;

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use web_interface::prelude::*;
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// struct Counter {
///   id: usize,
///   count: RwSignal<i32>
/// }
///
/// #[component]
/// fn Counters() -> impl IntoView {
///   let (counters, set_counters) = create_signal::<Vec<Counter>>(vec![]);
///
///   view! {
///     <div>
///       <For
///         // a function that returns the items we're iterating over; a signal is fine
///         each=move || counters.get()
///         // a unique key for each item
///         key=|counter| counter.id
///         // renders each item to a view
///         children=move |counter: Counter| {
///           view! {
///             <button>"Value: " {move || counter.count.get()}</button>
///           }
///         }
///       />
///     </div>
///   }
/// }
/// ```
///
/// For convenience, you can also choose to write template code directly in the `<For>`
/// component, using the `let` syntax:
///
/// ```
/// # use web_interface::prelude::*;
///
/// # #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// # struct Counter {
/// #   id: usize,
/// #   count: RwSignal<i32>
/// # }
/// #
/// # #[component]
/// # fn Counters() -> impl IntoView {
/// #   let (counters, set_counters) = create_signal::<Vec<Counter>>(vec![]);
/// #
///   view! {
///     <div>
///         <For
///           each=move || counters.get()
///           key=|counter| counter.id
///           let(counter)
///         >
///             <button>"Value: " {move || counter.count.get()}</button>
///         </For>
///     </div>
///   }
/// # }
/// ```
///
/// The `let` syntax also supports destructuring the pattern of your data.
/// `let((one, two))` in the case of tuples, and `let(Struct { field_one, field_two })`
/// in the case of structs.
///
/// ```
/// # use web_interface::prelude::*;
///
/// # #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// # struct Counter {
/// #   id: usize,
/// #   count: RwSignal<i32>
/// # }
/// #
/// # #[component]
/// # fn Counters() -> impl IntoView {
/// #   let (counters, set_counters) = create_signal::<Vec<Counter>>(vec![]);
/// #
///   view! {
///     <div>
///         <For
///           each=move || counters.get()
///           key=|counter| counter.id
///           let(Counter { id, count })
///         >
///             <button>"Value: " {move || count.get()}</button>
///         </For>
///     </div>
///   }
/// # }
/// ```
#[component]
pub fn For<IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the item, and returns the view that will be displayed for each item.
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
    EF: Fn(T) -> N + Send + Clone + 'static,
    N: IntoView + 'static,
    KF: Fn(&T) -> K + Send + Clone + 'static,
    K: Eq + Hash + 'static,
    T: Send + 'static,
{
    // this takes the owner of the For itself
    // this will end up with N + 1 children
    // 1) the effect for the `move || keyed(...)` updates
    // 2) an owner for each child
    //
    // this means
    // a) the reactive owner for each row will not be cleared when the whole list updates
    // b) context provided in each row will not wipe out the others
    let parent = Owner::current().expect("no reactive owner");
    let children = move |_, child| {
        let owner = parent.with(Owner::new);
        let view = owner.with(|| children(child));
        (drop, OwnedView::new_with_owner(view, owner))
    };
    move || keyed(each(), key.clone(), children.clone())
}

/// Iterates over children and displays them, keyed by the `key` function given.
///
/// Compared with For, it has an additional index parameter, which can be used to obtain the current index in real time.
///
/// This is much more efficient than naively iterating over nodes with `.iter().map(|n| view! { ... })...`,
/// as it avoids re-creating DOM nodes that are not being changed.
///
/// ```
/// # use web_interface::prelude::*;
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// struct Counter {
///   id: usize,
///   count: RwSignal<i32>
/// }
///
/// #[component]
/// fn Counters() -> impl IntoView {
///   let (counters, set_counters) = create_signal::<Vec<Counter>>(vec![]);
///
///   view! {
///     <div>
///       <ForEnumerate
///         // a function that returns the items we're iterating over; a signal is fine
///         each=move || counters.get()
///         // a unique key for each item
///         key=|counter| counter.id
///         // renders each item to a view
///         children={move |index: ReadSignal<usize>, counter: Counter| {
///           view! {
///             <button>{move || index.get()} ". Value: " {move || counter.count.get()}</button>
///           }
///         }}
///       />
///     </div>
///   }
/// }
/// ```
#[component]
pub fn ForEnumerate<IF, I, T, EF, N, KF, K>(
    /// Items over which the component should iterate.
    each: IF,
    /// A key function that will be applied to each item.
    key: KF,
    /// A function that takes the index and the item, and returns the view that will be displayed for each item.
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
    EF: Fn(ReadSignal<usize>, T) -> N + Send + Clone + 'static,
    N: IntoView + 'static,
    KF: Fn(&T) -> K + Send + Clone + 'static,
    K: Eq + Hash + 'static,
    T: Send + 'static,
{
    // this takes the owner of the For itself
    // this will end up with N + 1 children
    // 1) the effect for the `move || keyed(...)` updates
    // 2) an owner for each child
    //
    // this means
    // a) the reactive owner for each row will not be cleared when the whole list updates
    // b) context provided in each row will not wipe out the others
    let parent = Owner::current().expect("no reactive owner");
    let children = move |index, child| {
        let owner = parent.with(Owner::new);
        let (index, set_index) = ArcRwSignal::new(index).split();
        let view = owner.with(|| children(index.into(), child));
        (
            move |index| set_index.set(index),
            OwnedView::new_with_owner(view, owner),
        )
    };
    move || keyed(each(), key.clone(), children.clone())
}
