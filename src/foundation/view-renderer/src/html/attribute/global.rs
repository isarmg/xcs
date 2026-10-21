use super::Lang;
use crate::{
    html::{
        attribute::*,
        class::{class, Class, IntoClass},
        element::{ElementType, HasElementType, HtmlElement},
        event::{on, on_target, EventDescriptor, On, Targeted},
        property::{prop, IntoProperty, Property},
        style::{style, IntoStyle, Style},
    },
    prelude::RenderHtml,
    view::add_attr::AddAnyAttr,
};
use core::convert::From;

/// Adds an attribute that modifies the `class`.
pub trait ClassAttribute<C>
where
    C: IntoClass,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a CSS class to an element.
    fn class(self, value: C) -> Self::Output;
}

impl<E, At, Ch, C> ClassAttribute<C> for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    C: IntoClass,
{
    type Output = <Self as AddAnyAttr>::Output<Class<C>>;

    fn class(self, value: C) -> Self::Output {
        self.add_any_attr(class(value))
    }
}

/// Adds an attribute that modifies the DOM properties.
pub trait PropAttribute<K, P>
where
    P: IntoProperty,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a DOM property to an element.
    fn prop(self, key: K, value: P) -> Self::Output;
}

impl<E, At, Ch, K, P> PropAttribute<K, P> for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    K: AsRef<str> + Send,
    P: IntoProperty,
{
    type Output = <Self as AddAnyAttr>::Output<Property<K, P>>;

    fn prop(self, key: K, value: P) -> Self::Output {
        self.add_any_attr(prop(key, value))
    }
}

/// Adds an attribute that modifies the CSS styles.
pub trait StyleAttribute<S>
where
    S: IntoStyle,
{
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds a CSS style to an element.
    fn style(self, value: S) -> Self::Output;
}

impl<E, At, Ch, S> StyleAttribute<S> for HtmlElement<E, At, Ch>
where
    E: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    S: IntoStyle,
{
    type Output = <Self as AddAnyAttr>::Output<Style<S>>;

    fn style(self, value: S) -> Self::Output {
        self.add_any_attr(style(value))
    }
}

/// Adds an event listener to an element definition.
pub trait OnAttribute<E, F> {
    /// The type of the element with the event listener added.
    type Output;

    /// Adds an event listener to an element.
    fn on(self, event: E, cb: F) -> Self::Output;
}

impl<El, At, Ch, E, F> OnAttribute<E, F> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<crate::renderer::types::Event>,
    F: FnMut(E::EventType) + 'static,
{
    type Output = <Self as AddAnyAttr>::Output<On<E, F>>;

    fn on(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on(event, cb))
    }
}

/// Adds an event listener with a typed target to an element definition.
pub trait OnTargetAttribute<E, F, T> {
    /// The type of the element with the new attribute added.
    type Output;

    /// Adds an event listener with a typed target to an element definition.
    fn on_target(self, event: E, cb: F) -> Self::Output;
}

impl<El, At, Ch, E, F> OnTargetAttribute<E, F, Self> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    E: EventDescriptor + Send + 'static,
    E::EventType: 'static,
    E::EventType: From<crate::renderer::types::Event>,
    F: FnMut(Targeted<E::EventType, <Self as HasElementType>::ElementType>) + 'static,
{
    type Output = <Self as AddAnyAttr>::Output<On<E, Box<dyn FnMut(E::EventType)>>>;

    fn on_target(self, event: E, cb: F) -> Self::Output {
        self.add_any_attr(on_target::<E, HtmlElement<El, At, Ch>, F>(event, cb))
    }
}

/// Global attributes can be added to any HTML element.
pub trait GlobalAttributes<V>
where
    Self: Sized + AddAnyAttr,
    V: AttributeValue,
{
    /// The `autocapitalize` global attribute controls whether and how text input is automatically capitalized as it is entered/edited by the user.
    fn autocapitalize(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Autocapitalize, V>> {
        self.add_any_attr(autocapitalize(value))
    }

    /// The `dir` global attribute is an enumerated attribute indicating the directionality of the element's text.
    fn dir(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Dir, V>> {
        self.add_any_attr(dir(value))
    }

    /// The `hidden` global attribute is a Boolean attribute indicating that the element is not yet, or is no longer, relevant.
    fn hidden(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Hidden, V>> {
        self.add_any_attr(hidden(value))
    }

    /// The `id` global attribute defines a unique identifier (ID) which must be unique in the whole document.
    fn id(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Id, V>> {
        self.add_any_attr(id(value))
    }

    /// The `lang` global attribute helps define the language of an element.
    fn lang(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Lang, V>> {
        self.add_any_attr(lang(value))
    }

    /// The `popover` global attribute defines the popover's behavior.
    fn popover(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Popover, V>> {
        self.add_any_attr(popover(value))
    }

    /// The `role` global attribute defines the role of an element in ARIA.
    fn role(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Role, V>> {
        self.add_any_attr(role(value))
    }

    /// The `spellcheck` global attribute is an enumerated attribute that defines whether the element may be checked for spelling errors.
    fn spellcheck(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Spellcheck, V>> {
        self.add_any_attr(spellcheck(value))
    }

    /// The `tabindex` global attribute indicates if the element can take input focus.
    fn tabindex(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Tabindex, V>> {
        self.add_any_attr(tabindex(value))
    }

    /// The `title` global attribute contains text representing advisory information.
    fn title(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<Title, V>> {
        self.add_any_attr(title(value))
    }
}

impl<El, At, Ch, V> GlobalAttributes<V> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    V: AttributeValue,
{
}
