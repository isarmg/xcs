use super::{Attribute, NextAttribute};
use crate::{
    erased::{Erased, ErasedLocal},
    html::attribute::NamedAttributeKey,
    renderer::{dom::Element, Rndr},
};
use std::{any::TypeId, fmt::Debug, mem};

/// A type-erased container for any [`Attribute`].
pub struct AnyAttribute {
    type_id: TypeId,
    value: Erased,
    clone: fn(&Erased) -> AnyAttribute,

    build: fn(Erased, el: crate::renderer::types::Element) -> AnyAttributeState,
    rebuild: fn(Erased, &mut AnyAttributeState),

    keys: fn(&Erased) -> Vec<NamedAttributeKey>,
}

impl Clone for AnyAttribute {
    fn clone(&self) -> Self {
        (self.clone)(&self.value)
    }
}

impl Debug for AnyAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyAttribute").finish_non_exhaustive()
    }
}

/// View state for [`AnyAttribute`].
pub struct AnyAttributeState {
    type_id: TypeId,
    state: ErasedLocal,
    el: crate::renderer::types::Element,
    keys: Vec<NamedAttributeKey>,
}

/// Converts an [`Attribute`] into [`AnyAttribute`].
pub trait IntoAnyAttribute {
    /// Wraps the given attribute.
    fn into_any_attr(self) -> AnyAttribute;
}

impl<T> IntoAnyAttribute for T
where
    Self: Send,
    T: Attribute,
    crate::renderer::types::Element: Clone,
{
    fn into_any_attr(self) -> AnyAttribute {
        fn clone<T: Attribute + Clone + 'static>(value: &Erased) -> AnyAttribute {
            value.get_ref::<T>().clone().into_any_attr()
        }

        fn build<T: Attribute + 'static>(
            value: Erased,
            el: crate::renderer::types::Element,
        ) -> AnyAttributeState {
            AnyAttributeState {
                type_id: TypeId::of::<T>(),
                keys: value.get_ref::<T>().keys(),
                state: ErasedLocal::new(value.into_inner::<T>().build(&el)),
                el,
            }
        }

        fn rebuild<T: Attribute + 'static>(value: Erased, state: &mut AnyAttributeState) {
            let value = value.into_inner::<T>();
            let state = state.state.get_mut::<T::State>();
            value.rebuild(state);
        }

        fn keys<T: Attribute + 'static>(value: &Erased) -> Vec<NamedAttributeKey> {
            value.get_ref::<T>().keys()
        }

        let value = self.into_cloneable_owned();
        AnyAttribute {
            type_id: TypeId::of::<T::CloneableOwned>(),
            value: Erased::new(value),
            clone: clone::<T::CloneableOwned>,

            build: build::<T::CloneableOwned>,
            rebuild: rebuild::<T::CloneableOwned>,

            keys: keys::<T::CloneableOwned>,
        }
    }
}

impl NextAttribute for AnyAttribute {
    type Output<NewAttr: Attribute> = Vec<AnyAttribute>;

    fn add_any_attr<NewAttr: Attribute>(self, new_attr: NewAttr) -> Self::Output<NewAttr> {
        vec![self, new_attr.into_any_attr()]
    }
}

impl Attribute for AnyAttribute {
    type State = AnyAttributeState;
    type Cloneable = AnyAttribute;
    type CloneableOwned = AnyAttribute;


    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            _ = el;
            panic!("Type-erased attributes cannot be used in a static browser template.");
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        (self.build)(self.value, el.clone())
    }

    fn rebuild(self, state: &mut Self::State) {
        if self.type_id == state.type_id {
            (self.rebuild)(self.value, state)
        } else {
            let new = self.build(&state.el);
            *state = new;
        }
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn keys(&self) -> Vec<NamedAttributeKey> {
        (self.keys)(&self.value)
    }
}

impl NextAttribute for Vec<AnyAttribute> {
    type Output<NewAttr: Attribute> = Self;

    fn add_any_attr<NewAttr: Attribute>(mut self, new_attr: NewAttr) -> Self::Output<NewAttr> {
        self.push(new_attr.into_any_attr());
        self
    }
}

impl Attribute for Vec<AnyAttribute> {
    type State = (Element, Vec<AnyAttributeState>);
    type Cloneable = Vec<AnyAttribute>;
    type CloneableOwned = Vec<AnyAttribute>;


    fn hydrate(self, el: &crate::renderer::types::Element) -> Self::State {
        {
            _ = el;
            panic!("Type-erased attributes cannot be used in a static browser template.");
        }
    }

    fn build(self, el: &crate::renderer::types::Element) -> Self::State {
        (
            el.clone(),
            self.into_iter().map(|attr| attr.build(el)).collect(),
        )
    }

    fn rebuild(self, state: &mut Self::State) {
        let (el, state) = state;
        for old in mem::take(state) {
            for key in old.keys {
                match key {
                    NamedAttributeKey::InnerHtml => {
                        Rndr::set_inner_html(&old.el, "");
                    }
                    NamedAttributeKey::Property(prop_name) => {
                        Rndr::set_property(&old.el, &prop_name, &wasm_bindgen::JsValue::UNDEFINED);
                    }
                    NamedAttributeKey::Attribute(key) => {
                        Rndr::remove_attribute(&old.el, &key);
                    }
                }
            }
        }
        *state = self.into_iter().map(|s| s.build(el)).collect();
    }

    fn into_cloneable(self) -> Self::Cloneable {
        self
    }

    fn into_cloneable_owned(self) -> Self::CloneableOwned {
        self
    }

    fn keys(&self) -> Vec<NamedAttributeKey> {
        self.iter().flat_map(|s| s.keys()).collect()
    }
}
