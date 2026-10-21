use super::{add_attr::AddAnyAttr, Mountable, Position, PositionState, Render, RenderHtml};
use crate::{html::attribute::Attribute, hydration::Cursor, renderer::Rndr};
use view_alternatives::Either;
use iterator_toolkit::Itertools;

/// Retained view state for an `Option`.
pub type OptionState<T> = Either<<T as Render>::State, <() as Render>::State>;

impl<T> Render for Option<T>
where
    T: Render,
{
    type State = OptionState<T>;

    fn build(self) -> Self::State {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .build()
    }

    fn rebuild(self, state: &mut Self::State) {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .rebuild(state)
    }
}

impl<T> AddAnyAttr for Option<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> = Option<<T as AddAnyAttr>::Output<SomeNewAttr>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        self.map(|n| n.add_any_attr(attr))
    }
}

impl<T> RenderHtml for Option<T>
where
    T: RenderHtml,
{
    type Owned = Option<T::Owned>;

    #[track_caller]
    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        match self {
            Some(value) => Either::Left(value),
            None => Either::Right(()),
        }
        .hydrate(cursor, position)
    }

    fn into_owned(self) -> Self::Owned {
        self.map(RenderHtml::into_owned)
    }
}

impl<T> Render for Vec<T>
where
    T: Render,
{
    type State = VecState<T::State>;

    fn build(self) -> Self::State {
        let marker = Rndr::create_placeholder();
        VecState {
            states: self.into_iter().map(T::build).collect(),
            marker,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let VecState { states, marker } = state;
        let old = states;
        // this is an unkeyed diff
        if old.is_empty() {
            let mut new = self.build().states;
            for item in new.iter_mut() {
                Rndr::try_mount_before(item, marker.as_ref());
            }
            *old = new;
        } else if self.is_empty() {
            // TODO fast path for clearing
            for item in old.iter_mut() {
                item.unmount();
            }
            old.clear();
        } else {
            let mut adds = vec![];
            let mut removes_at_end = 0;
            for item in self.into_iter().zip_longest(old.iter_mut()) {
                match item {
                    iterator_toolkit::EitherOrBoth::Both(new, old) => T::rebuild(new, old),
                    iterator_toolkit::EitherOrBoth::Left(new) => {
                        let mut new_state = new.build();
                        Rndr::try_mount_before(&mut new_state, marker.as_ref());
                        adds.push(new_state);
                    }
                    iterator_toolkit::EitherOrBoth::Right(old) => {
                        removes_at_end += 1;
                        old.unmount()
                    }
                }
            }
            old.truncate(old.len() - removes_at_end);
            old.append(&mut adds);
        }
    }
}

/// Retained view state for a `Vec<_>`.
pub struct VecState<T>
where
    T: Mountable,
{
    states: Vec<T>,
    // Vecs keep a placeholder because they have the potential to add additional items,
    // after their own items but before the next neighbor. It is much easier to add an
    // item before a known placeholder than to add it after the last known item, so we
    // just leave a placeholder here unlike zero-or-one iterators (Option, Result, etc.)
    marker: crate::renderer::types::Placeholder,
}

impl<T> Mountable for VecState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        for state in self.states.iter_mut() {
            state.unmount();
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
        self.marker.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        for state in &self.states {
            if state.insert_before_this(child) {
                return true;
            }
        }
        self.marker.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.states
            .iter()
            .flat_map(|item| item.elements())
            .collect()
    }
}

impl<T> AddAnyAttr for Vec<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> = Vec<<T as AddAnyAttr>::Output<SomeNewAttr::Cloneable>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable();
        self.into_iter()
            .map(|n| n.add_any_attr(attr.clone()))
            .collect()
    }
}

impl<T> RenderHtml for Vec<T>
where
    T: RenderHtml,
{
    type Owned = Vec<T::Owned>;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let states = self
            .into_iter()
            .map(|child| child.hydrate(cursor, position))
            .collect();

        let marker = cursor.next_placeholder(position);
        position.set(Position::NextChild);

        VecState { states, marker }
    }

    fn into_owned(self) -> Self::Owned {
        self.into_iter()
            .map(RenderHtml::into_owned)
            .collect::<Vec<_>>()
    }
}

/// A container used for ErasedMode. It's slightly better than a raw Vec<> because the rendering traits don't have to worry about the length of the Vec changing, therefore no marker traits etc.
pub struct StaticVec<T>(pub(crate) Vec<T>);

impl<T: Clone> Clone for StaticVec<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> IntoIterator for StaticVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> StaticVec<T> {
    /// Iterates over the items.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }
}

impl<T> From<Vec<T>> for StaticVec<T> {
    fn from(vec: Vec<T>) -> Self {
        Self(vec)
    }
}

impl<T> From<StaticVec<T>> for Vec<T> {
    fn from(static_vec: StaticVec<T>) -> Self {
        static_vec.0
    }
}

/// Retained view state for a `StaticVec<Vec<_>>`.
pub struct StaticVecState<T>
where
    T: Mountable,
{
    states: Vec<T>,
    marker: crate::renderer::types::Placeholder,
}

impl<T> Mountable for StaticVecState<T>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        for state in self.states.iter_mut() {
            state.unmount();
        }
        self.marker.unmount();
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
        self.marker.mount(parent, marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        for state in &self.states {
            if state.insert_before_this(child) {
                return true;
            }
        }
        self.marker.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.states
            .iter()
            .flat_map(|item| item.elements())
            .collect()
    }
}

impl<T> Render for StaticVec<T>
where
    T: Render,
{
    type State = StaticVecState<T::State>;

    fn build(self) -> Self::State {
        let marker = Rndr::create_placeholder();
        Self::State {
            states: self.0.into_iter().map(T::build).collect(),
            marker,
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let StaticVecState { states, marker } = state;
        let old = states;

        // reuses the Vec impl
        if old.is_empty() {
            let mut new = self.build().states;
            for item in new.iter_mut() {
                Rndr::mount_before(item, marker.as_ref());
            }
            *old = new;
        } else if self.0.is_empty() {
            // TODO fast path for clearing
            for item in old.iter_mut() {
                item.unmount();
            }
            old.clear();
        } else {
            let mut adds = vec![];
            let mut removes_at_end = 0;
            for item in self.0.into_iter().zip_longest(old.iter_mut()) {
                match item {
                    iterator_toolkit::EitherOrBoth::Both(new, old) => T::rebuild(new, old),
                    iterator_toolkit::EitherOrBoth::Left(new) => {
                        let mut new_state = new.build();
                        Rndr::mount_before(&mut new_state, marker.as_ref());
                        adds.push(new_state);
                    }
                    iterator_toolkit::EitherOrBoth::Right(old) => {
                        removes_at_end += 1;
                        old.unmount()
                    }
                }
            }
            old.truncate(old.len() - removes_at_end);
            old.append(&mut adds);
        }
    }
}

impl<T> AddAnyAttr for StaticVec<T>
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> =
        StaticVec<<T as AddAnyAttr>::Output<SomeNewAttr::Cloneable>>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable();
        self.0
            .into_iter()
            .map(|n| n.add_any_attr(attr.clone()))
            .collect::<Vec<_>>()
            .into()
    }
}

impl<T> RenderHtml for StaticVec<T>
where
    T: RenderHtml,
{
    type Owned = StaticVec<T::Owned>;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let states = self
            .0
            .into_iter()
            .map(|child| child.hydrate(cursor, position))
            .collect();

        let marker = cursor.next_placeholder(position);
        position.set(Position::NextChild);

        Self::State { states, marker }
    }

    fn into_owned(self) -> Self::Owned {
        self.0
            .into_iter()
            .map(RenderHtml::into_owned)
            .collect::<Vec<_>>()
            .into()
    }
}

impl<T, const N: usize> Render for [T; N]
where
    T: Render,
{
    type State = ArrayState<T::State, N>;

    fn build(self) -> Self::State {
        Self::State {
            states: self.map(T::build),
        }
    }

    fn rebuild(self, state: &mut Self::State) {
        let Self::State { states } = state;
        let old = states;
        // this is an unkeyed diff
        self.into_iter()
            .zip(old.iter_mut())
            .for_each(|(new, old)| T::rebuild(new, old));
    }
}

/// Retained view state for a `Vec<_>`.
pub struct ArrayState<T, const N: usize>
where
    T: Mountable,
{
    states: [T; N],
}

impl<T, const N: usize> Mountable for ArrayState<T, N>
where
    T: Mountable,
{
    fn unmount(&mut self) {
        self.states.iter_mut().for_each(Mountable::unmount);
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        for state in self.states.iter_mut() {
            state.mount(parent, marker);
        }
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        for state in &self.states {
            if state.insert_before_this(child) {
                return true;
            }
        }
        false
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        self.states
            .iter()
            .flat_map(|item| item.elements())
            .collect()
    }
}
impl<T, const N: usize> AddAnyAttr for [T; N]
where
    T: AddAnyAttr,
{
    type Output<SomeNewAttr: Attribute> = [<T as AddAnyAttr>::Output<SomeNewAttr::Cloneable>; N];

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr>
    where
        Self::Output<NewAttr>: RenderHtml,
    {
        let attr = attr.into_cloneable();
        self.map(|n| n.add_any_attr(attr.clone()))
    }
}

impl<T, const N: usize> RenderHtml for [T; N]
where
    T: RenderHtml,
{
    type Owned = [T::Owned; N];

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let states = self.map(|child| child.hydrate(cursor, position));
        ArrayState { states }
    }

    fn into_owned(self) -> Self::Owned {
        self.into_iter()
            .map(RenderHtml::into_owned)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }
}
