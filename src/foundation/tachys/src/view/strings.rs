use super::{Mountable, Position, PositionState, Render, RenderHtml, ToTemplate};
use crate::{
    hydration::Cursor,
    no_attrs,
    renderer::{CastFrom, Rndr},
};
use std::{borrow::Cow, rc::Rc, sync::Arc};

no_attrs!(&'a str);
no_attrs!(String);
no_attrs!(Arc<str>);
no_attrs!(Cow<'a, str>);

/// Retained view state for `&str`.
pub struct StrState<'a> {
    pub(crate) node: crate::renderer::types::Text,
    str: &'a str,
}

impl<'a> Render for &'a str {
    type State = StrState<'a>;

    fn build(self) -> Self::State {
        let node = Rndr::create_text_node(self);
        StrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let StrState { node, str } = state;
        if &self != str {
            Rndr::set_text(node, self);
            *str = self;
        }
    }
}

impl RenderHtml for &str {
    type Owned = String;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        if position.get() == Position::FirstChild {
            cursor.child();
        } else {
            cursor.sibling();
        }

        // separating placeholder marker comes before text node
        if matches!(position.get(), Position::NextChildAfterText) {
            cursor.sibling();
        }

        let node = cursor.current();
        let node = crate::renderer::types::Text::cast_from(node.clone())
            .unwrap_or_else(|| crate::hydration::failed_to_cast_text_node(node));

        {
            Rndr::set_text(&node, self);
        }
        position.set(Position::NextChildAfterText);

        StrState { node, str: self }
    }

    fn into_owned(self) -> Self::Owned {
        self.to_string()
    }
}

impl ToTemplate for &str {
    const TEMPLATE: &'static str = " <!>";

    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        position: &mut Position,
    ) {
        if matches!(*position, Position::NextChildAfterText) {
            buf.push_str("<!>")
        }
        buf.push(' ');
        *position = Position::NextChildAfterText;
    }
}

impl Mountable for StrState<'_> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.node.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

/// Retained view state for `String`.
pub struct StringState {
    node: crate::renderer::types::Text,
    str: String,
}

impl Render for String {
    type State = StringState;

    fn build(self) -> Self::State {
        let node = Rndr::create_text_node(&self);
        StringState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let StringState { node, str } = state;
        if &self != str {
            Rndr::set_text(node, &self);
            *str = self;
        }
    }
}

impl RenderHtml for String {
    type Owned = Self;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let StrState { node, .. } = self.as_str().hydrate(cursor, position);
        StringState { node, str: self }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl ToTemplate for String {
    const TEMPLATE: &'static str = <&str as ToTemplate>::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        <&str as ToTemplate>::to_template(buf, class, style, inner_html, position)
    }
}

impl Mountable for StringState {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.node.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

/// Retained view state for `Rc<str>`.
pub struct RcStrState {
    node: crate::renderer::types::Text,
    str: Rc<str>,
}

impl Render for Rc<str> {
    type State = RcStrState;

    fn build(self) -> Self::State {
        let node = Rndr::create_text_node(&self);
        RcStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let RcStrState { node, str } = state;
        if !Rc::ptr_eq(&self, str) {
            Rndr::set_text(node, &self);
            *str = self;
        }
    }
}

// can't Send an Rc<str> between threads, so can't implement async HTML rendering that might need
// to send it

impl ToTemplate for Rc<str> {
    const TEMPLATE: &'static str = <&str as ToTemplate>::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        <&str as ToTemplate>::to_template(buf, class, style, inner_html, position)
    }
}

impl Mountable for RcStrState {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.node.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

/// Retained view state for `Arc<str>`.
pub struct ArcStrState {
    node: crate::renderer::types::Text,
    str: Arc<str>,
}

impl Render for Arc<str> {
    type State = ArcStrState;

    fn build(self) -> Self::State {
        let node = Rndr::create_text_node(&self);
        ArcStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let ArcStrState { node, str } = state;
        if self != *str {
            Rndr::set_text(node, &self);
            *str = self;
        }
    }
}

impl RenderHtml for Arc<str> {
    type Owned = Self;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } = this.hydrate(cursor, position);
        ArcStrState { node, str: self }
    }

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl ToTemplate for Arc<str> {
    const TEMPLATE: &'static str = <&str as ToTemplate>::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        <&str as ToTemplate>::to_template(buf, class, style, inner_html, position)
    }
}

impl Mountable for ArcStrState {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.node.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}

/// Retained view state for `Cow<'_, str>`.
pub struct CowStrState<'a> {
    node: crate::renderer::types::Text,
    str: Cow<'a, str>,
}

impl<'a> Render for Cow<'a, str> {
    type State = CowStrState<'a>;

    fn build(self) -> Self::State {
        let node = Rndr::create_text_node(&self);
        CowStrState { node, str: self }
    }

    fn rebuild(self, state: &mut Self::State) {
        let CowStrState { node, str } = state;
        if self != *str {
            Rndr::set_text(node, &self);
            *str = self;
        }
    }
}

impl RenderHtml for Cow<'_, str> {
    type Owned = String;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        let this: &str = self.as_ref();
        let StrState { node, .. } = this.hydrate(cursor, position);
        CowStrState { node, str: self }
    }

    fn into_owned(self) -> <Self as RenderHtml>::Owned {
        self.into_owned()
    }
}

impl ToTemplate for Cow<'_, str> {
    const TEMPLATE: &'static str = <&str as ToTemplate>::TEMPLATE;

    fn to_template(
        buf: &mut String,
        class: &mut String,
        style: &mut String,
        inner_html: &mut String,
        position: &mut Position,
    ) {
        <&str as ToTemplate>::to_template(buf, class, style, inner_html, position)
    }
}

impl Mountable for CowStrState<'_> {
    fn unmount(&mut self) {
        self.node.unmount()
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, self.node.as_ref(), marker);
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        self.node.insert_before_this(child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        vec![]
    }
}
