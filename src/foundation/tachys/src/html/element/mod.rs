#[cfg(any(debug_assertions, leptos_debuginfo))]
use crate::hydration::set_currently_hydrating;
#[cfg(erase_components)]
use crate::view::any_view::AnyView;
use crate::{
    html::attribute::Attribute,
    hydration::{failed_to_cast_element, Cursor},
    renderer::{CastFrom, Rndr},
    view::{
        add_attr::AddAnyAttr, IntoRender, Mountable, Position, PositionState, Render, RenderHtml,
        ToTemplate,
    },
};
use const_str_slice_concat::{const_concat, const_concat_with_prefix, str_from_buffer};
use std::ops::Deref;

mod custom;
mod element_ext;
mod elements;
mod inner_html;
use super::attribute::NextAttribute;
pub use custom::*;
pub use element_ext::*;
pub use elements::*;
pub use inner_html::*;
#[cfg(any(debug_assertions, leptos_debuginfo))]
use std::panic::Location;

/// The typed representation of an HTML element.
#[derive(Debug, PartialEq, Eq)]
pub struct HtmlElement<E, At, Ch> {
    #[cfg(any(debug_assertions, leptos_debuginfo))]
    pub(crate) defined_at: &'static Location<'static>,
    pub(crate) tag: E,
    pub(crate) attributes: At,
    pub(crate) children: Ch,
}

impl<E: Clone, At: Clone, Ch: Clone> Clone for HtmlElement<E, At, Ch> {
    fn clone(&self) -> Self {
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag.clone(),
            attributes: self.attributes.clone(),
            children: self.children.clone(),
        }
    }
}

impl<E: Copy, At: Copy, Ch: Copy> Copy for HtmlElement<E, At, Ch> {}

/*impl<E, At, Ch> ElementType for HtmlElement<E, At, Ch>
where
    E: ElementType,
{
    type Output = E::Output;

    const TAG: &'static str = E::TAG;

    const SELF_CLOSING: bool = E::SELF_CLOSING;

    fn tag(&self) -> &str {
        Self::TAG
    }
}*/

#[cfg(not(erase_components))]
impl<E, At, Ch, NewChild> ElementChild<NewChild> for HtmlElement<E, At, Ch>
where
    E: ElementWithChildren,
    Ch: RenderHtml + next_tuple::NextTuple,
    <Ch as next_tuple::NextTuple>::Output<NewChild::Output>: Render,

    NewChild: IntoRender,
    NewChild::Output: RenderHtml,
{
    type Output = HtmlElement<E, At, <Ch as next_tuple::NextTuple>::Output<NewChild::Output>>;

    fn child(self, child: NewChild) -> Self::Output {
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes: self.attributes,
            children: self.children.next_tuple(child.into_render()),
        }
    }
}

#[cfg(erase_components)]
impl<E, At, Ch, NewChild> ElementChild<NewChild> for HtmlElement<E, At, Ch>
where
    E: ElementWithChildren,
    Ch: RenderHtml + NextChildren,

    NewChild: IntoRender,
    NewChild::Output: RenderHtml,
{
    type Output = HtmlElement<E, At, crate::view::iterators::StaticVec<AnyView>>;

    fn child(self, child: NewChild) -> Self::Output {
        use crate::view::any_view::IntoAny;

        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes: self.attributes,
            children: self.children.next_children(child.into_render().into_any()),
        }
    }
}

#[cfg(erase_components)]
trait NextChildren {
    fn next_children(self, child: AnyView) -> crate::view::iterators::StaticVec<AnyView>;
}

#[cfg(erase_components)]
mod erased_tuples {
    use super::*;
    use crate::view::{any_view::IntoAny, iterators::StaticVec};

    impl NextChildren for StaticVec<AnyView> {
        fn next_children(mut self, child: AnyView) -> StaticVec<AnyView> {
            self.0.push(child);
            self
        }
    }

    impl NextChildren for () {
        fn next_children(self, child: AnyView) -> StaticVec<AnyView> {
            vec![child].into()
        }
    }

    impl<T: RenderHtml> NextChildren for (T,) {
        fn next_children(self, child: AnyView) -> StaticVec<AnyView> {
            vec![self.0.into_owned().into_any(), child].into()
        }
    }

    macro_rules! impl_next_children_tuples {
        ($($ty:ident),*) => {
            impl<$($ty: RenderHtml),*> NextChildren for ($($ty,)*)
             {
                fn next_children(
                    self, child: AnyView,
                ) -> StaticVec<AnyView> {
                    #[allow(non_snake_case)]
                    let ($($ty,)*) = self;
                    vec![$($ty.into_owned().into_any(),)* child].into()
                }
            }
        };
    }

    impl_next_children_tuples!(AA, BB);
    impl_next_children_tuples!(AA, BB, CC);
    impl_next_children_tuples!(AA, BB, CC, DD);
    impl_next_children_tuples!(AA, BB, CC, DD, EE);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP);
    impl_next_children_tuples!(AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ);
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS, TT
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS, TT, UU
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS, TT, UU, VV
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS, TT, UU, VV, WW
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS, TT, UU, VV, WW,
        XX
    );
    impl_next_children_tuples!(
        AA, BB, CC, DD, EE, FF, GG, HH, II, JJ, KK, LL, MM, NN, OO, PP, QQ, RR, SS, TT, UU, VV, WW,
        XX, YY
    );
}

impl<E, At, Ch> AddAnyAttr for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
{
    type Output<SomeNewAttr: Attribute> =
        HtmlElement<E, <At as NextAttribute>::Output<SomeNewAttr>, Ch>;

    fn add_any_attr<NewAttr: Attribute>(self, attr: NewAttr) -> Self::Output<NewAttr> {
        let HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at,
            tag,
            attributes,
            children,
        } = self;
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at,
            tag,
            attributes: attributes.add_any_attr(attr),
            children,
        }
    }
}

/// Adds a child to the element.
pub trait ElementChild<NewChild>
where
    NewChild: IntoRender,
{
    /// The type of the element, with the child added.
    type Output;

    /// Adds a child to an element.
    fn child(self, child: NewChild) -> Self::Output;
}

/// An HTML element.
pub trait ElementType: Send + 'static {
    /// The underlying native widget type that this represents.
    type Output;

    /// The element's tag.
    const TAG: &'static str;
    /// Whether the element is self-closing.
    const SELF_CLOSING: bool;
    /// Whether the element's children should be escaped. This should be `true` except for elements
    /// like `<style>` and `<script>`, which include other languages that should not use HTML
    /// entity escaping.
    const ESCAPE_CHILDREN: bool;
    /// The element's namespace, if it is not HTML.
    const NAMESPACE: Option<&'static str>;

    /// The element's tag.
    fn tag(&self) -> &str;
}

/// Denotes that the type that implements this has a particular HTML element type.
pub trait HasElementType {
    /// The element type.
    type ElementType;
}

pub(crate) trait ElementWithChildren {}

impl<E, At, Ch> HasElementType for HtmlElement<E, At, Ch>
where
    E: ElementType,
{
    type ElementType = E::Output;
}

impl<E, At, Ch> Render for HtmlElement<E, At, Ch>
where
    E: ElementType,
    At: Attribute,
    Ch: Render,
{
    type State = ElementState<At::State, Ch::State>;

    fn rebuild(self, state: &mut Self::State) {
        // check whether the tag is the same, for custom elements
        // because this is const `false` for all other element types,
        // the compiler should be able to optimize it out
        if E::TAG.is_empty() {
            // see https://github.com/leptos-rs/leptos/issues/4412
            let new_tag = self.tag.tag();

            // this is not particularly efficient, but it saves us from
            // having to keep track of the tag name for every element state
            let old_tag = state.el.tag_name();
            if new_tag != old_tag {
                let mut new_state = self.build();
                state.insert_before_this(&mut new_state);
                state.unmount();
                *state = new_state;
                return;
            }
        }

        // rebuild attributes and children for any element
        let ElementState {
            attrs, children, ..
        } = state;
        self.attributes.rebuild(attrs);
        if let Some(children) = children {
            self.children.rebuild(children);
        }
    }

    fn build(self) -> Self::State {
        let el = Rndr::create_element(self.tag.tag(), E::NAMESPACE);

        let attrs = self.attributes.build(&el);

        let children = if E::SELF_CLOSING {
            None
        } else {
            let mut children = self.children.build();
            children.mount(&el, None);
            Some(children)
        };

        ElementState {
            el,
            attrs,
            children,
        }
    }
}

impl<E, At, Ch> RenderHtml for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
{
    type Owned = HtmlElement<E, At::CloneableOwned, Ch::Owned>;

    fn hydrate(self, cursor: &Cursor, position: &PositionState) -> Self::State {
        // non-Static custom elements need special support in templates
        // because they haven't been inserted type-wise
        if E::TAG.is_empty() {
            panic!("Custom elements are not supported in ViewTemplate.");
        }

        // codegen optimisation:
        fn inner_1(
            cursor: &Cursor,
            position: &PositionState,
            tag_name: &str,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: &'static std::panic::Location<'static>,
        ) -> crate::renderer::types::Element {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            {
                set_currently_hydrating(Some(defined_at));
            }

            let curr_position = position.get();
            if curr_position == Position::FirstChild {
                cursor.child();
            } else if curr_position != Position::Current {
                cursor.sibling();
            }
            crate::renderer::types::Element::cast_from(cursor.current())
                .unwrap_or_else(|| failed_to_cast_element(tag_name, cursor.current()))
        }
        let el = inner_1(
            cursor,
            position,
            E::TAG,
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            self.defined_at,
        );

        let attrs = self.attributes.hydrate(&el);

        // hydrate children
        let children = if !Ch::EXISTS || !E::ESCAPE_CHILDREN {
            None
        } else {
            position.set(Position::FirstChild);
            Some(self.children.hydrate(cursor, position))
        };

        // codegen optimisation:
        fn inner_2(
            cursor: &Cursor,
            position: &PositionState,
            el: &crate::renderer::types::Element,
        ) {
            // go to next sibling
            cursor.set(
                <crate::renderer::types::Element as AsRef<crate::renderer::types::Node>>::as_ref(
                    el,
                )
                .clone(),
            );
            position.set(Position::NextChild);
        }
        inner_2(cursor, position, &el);

        ElementState {
            el,
            attrs,
            children,
        }
    }

    fn into_owned(self) -> Self::Owned {
        HtmlElement {
            #[cfg(any(debug_assertions, leptos_debuginfo))]
            defined_at: self.defined_at,
            tag: self.tag,
            attributes: self.attributes.into_cloneable_owned(),
            children: self.children.into_owned(),
        }
    }
}

/// The retained view state for an HTML element.
pub struct ElementState<At, Ch> {
    pub(crate) el: crate::renderer::types::Element,
    pub(crate) attrs: At,
    pub(crate) children: Option<Ch>,
}

impl<At, Ch> Deref for ElementState<At, Ch> {
    type Target = crate::renderer::types::Element;

    fn deref(&self) -> &Self::Target {
        &self.el
    }
}

impl<At, Ch> Mountable for ElementState<At, Ch> {
    fn unmount(&mut self) {
        Rndr::remove(&self.el);
    }

    fn mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) {
        Rndr::insert_node(parent, &self.el, marker);
    }

    fn try_mount(
        &mut self,
        parent: &crate::renderer::types::Element,
        marker: Option<&crate::renderer::types::Node>,
    ) -> bool {
        Rndr::try_insert_node(parent, &self.el, marker)
    }

    fn insert_before_this(&self, child: &mut dyn Mountable) -> bool {
        // codegen optimisation:
        fn inner(element: &crate::renderer::types::Element, child: &mut dyn Mountable) -> bool {
            if let Some(parent) =
                Rndr::get_parent(element).and_then(crate::renderer::types::Element::cast_from)
            {
                child.mount(&parent, Some(element));
                true
            } else {
                false
            }
        }
        inner(&self.el, child)
    }

    fn elements(&self) -> Vec<crate::renderer::types::Element> {
        // codegen optimisation:
        fn inner(
            element: &crate::renderer::types::Element,
        ) -> Vec<crate::renderer::types::Element> {
            vec![element.clone()]
        }
        inner(&self.el)
    }
}

impl<E, At, Ch> ToTemplate for HtmlElement<E, At, Ch>
where
    E: ElementType,
    At: Attribute + ToTemplate,
    Ch: Render + ToTemplate,
{
    const TEMPLATE: &'static str = str_from_buffer(
        &const {
            const_concat(
                &const {
                    [
                        "<",
                        E::TAG,
                        At::TEMPLATE,
                        str_from_buffer(
                            &const { const_concat_with_prefix(&[At::CLASS], " class=\"", "\"") },
                        ),
                        str_from_buffer(
                            &const { const_concat_with_prefix(&[At::STYLE], " style=\"", "\"") },
                        ),
                        ">",
                        if E::SELF_CLOSING { "" } else { Ch::TEMPLATE },
                        if E::SELF_CLOSING { "" } else { "</" },
                        if E::SELF_CLOSING { "" } else { E::TAG },
                        if E::SELF_CLOSING { "" } else { ">" },
                    ]
                },
            )
        },
    );

    fn to_template(
        buf: &mut String,
        _class: &mut String,
        _style: &mut String,
        _inner_html: &mut String,
        position: &mut Position,
    ) {
        // for custom elements without type known at compile time, do nothing
        if !E::TAG.is_empty() {
            // opening tag and attributes
            let mut class = String::new();
            let mut style = String::new();
            let mut inner_html = String::new();

            buf.push('<');
            buf.push_str(E::TAG);
            <At as ToTemplate>::to_template_attribute(
                buf,
                &mut class,
                &mut style,
                &mut inner_html,
                position,
            );

            if !class.is_empty() {
                buf.push(' ');
                buf.push_str("class=\"");
                buf.push_str(class.trim_start().trim_end());
                buf.push('"');
            }
            if !style.is_empty() {
                buf.push(' ');
                buf.push_str("style=\"");
                buf.push_str(style.trim_start().trim_end());
                buf.push('"');
            }
            buf.push('>');

            if !E::SELF_CLOSING {
                // children
                *position = Position::FirstChild;
                class.clear();
                style.clear();
                inner_html.clear();
                Ch::to_template(buf, &mut class, &mut style, &mut inner_html, position);

                // closing tag
                buf.push_str("</");
                buf.push_str(E::TAG);
                buf.push('>');
            }
            *position = Position::NextChild;
        }
    }
}
