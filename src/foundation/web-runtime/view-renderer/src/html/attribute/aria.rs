use crate::{
    html::{
        attribute::{Attr, *},
        element::{ElementType, HtmlElement},
    },
    renderer::Rndr,
    view::{add_attr::AddAnyAttr, RenderHtml},
};

/// Applies ARIA attributes to an HTML element.
pub trait AriaAttributes<Rndr, V>
where
    Self: Sized + AddAnyAttr,
    V: AttributeValue,
{
    /// Identifies the currently active descendant of a composite widget.
    fn aria_activedescendant(
        self,
        value: V,
    ) -> <Self as AddAnyAttr>::Output<Attr<AriaActivedescendant, V>> {
        self.add_any_attr(aria_activedescendant(value))
    }

    /// Indicates whether assistive technologies will present all, or only parts of, the changed region based on the change notifications defined by the `aria-relevant` attribute.
    fn aria_atomic(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaAtomic, V>> {
        self.add_any_attr(aria_atomic(value))
    }

    /// Indicates whether an element, and its subtree, are currently being updated.
    fn aria_busy(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaBusy, V>> {
        self.add_any_attr(aria_busy(value))
    }

    /// Identifies the element (or elements) whose contents or presence are controlled by the current element.
    fn aria_controls(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaControls, V>> {
        self.add_any_attr(aria_controls(value))
    }

    /// Indicates the element that represents the current item within a container or set of related elements.
    fn aria_current(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaCurrent, V>> {
        self.add_any_attr(aria_current(value))
    }

    /// Indicates that the element is perceivable but disabled, so it is not editable or otherwise operable.
    fn aria_disabled(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaDisabled, V>> {
        self.add_any_attr(aria_disabled(value))
    }

    /// Indicates whether the element, or another grouping element it controls, is currently expanded or collapsed.
    fn aria_expanded(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaExpanded, V>> {
        self.add_any_attr(aria_expanded(value))
    }

    /// Indicates the availability and type of interactive popup element, such as menu or dialog, that can be triggered by an element.
    fn aria_haspopup(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaHaspopup, V>> {
        self.add_any_attr(aria_haspopup(value))
    }

    /// Indicates whether the element is exposed to an accessibility API.
    fn aria_hidden(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaHidden, V>> {
        self.add_any_attr(aria_hidden(value))
    }

    /// Indicates the entered value does not conform to the format expected by the application.
    fn aria_invalid(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaInvalid, V>> {
        self.add_any_attr(aria_invalid(value))
    }

    /// Defines a string value that labels the current element.
    fn aria_label(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaLabel, V>> {
        self.add_any_attr(aria_label(value))
    }

    /// Identifies the element (or elements) that labels the current element.
    fn aria_labelledby(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaLabelledby, V>> {
        self.add_any_attr(aria_labelledby(value))
    }

    /// Indicates that an element will be updated, and describes the types of updates the user agents, assistive technologies, and user can expect from the live region.
    fn aria_live(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaLive, V>> {
        self.add_any_attr(aria_live(value))
    }

    /// Indicates the current "pressed" state of toggle buttons.
    fn aria_pressed(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaPressed, V>> {
        self.add_any_attr(aria_pressed(value))
    }

    /// Indicates the current "selected" state of various widgets.
    fn aria_selected(self, value: V) -> <Self as AddAnyAttr>::Output<Attr<AriaSelected, V>> {
        self.add_any_attr(aria_selected(value))
    }
}

impl<El, At, Ch, V> AriaAttributes<Rndr, V> for HtmlElement<El, At, Ch>
where
    El: ElementType + Send,
    At: Attribute + Send,
    Ch: RenderHtml + Send,
    V: AttributeValue,
{
}
