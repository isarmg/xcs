use wasm_bindgen::UnwrapThrowExt;

#[macro_export]
/// Use for tracing property
macro_rules! tracing_props {
    () => {
        $crate::__tracing::span!(
            $crate::__tracing::Level::TRACE,
            "dom_helpers::tracing_props",
            props = String::from("[]")
        );
    };
    ($($prop:tt),+ $(,)?) => {
        {
            use $crate::macro_helpers::tracing_property::{Match, SerializeMatch, DefaultMatch};
            let mut props = String::from('[');
            $(
                let prop = (&&Match {
                    name: stringify!{$prop},
                    value: std::cell::Cell::new(Some(&$prop))
                }).spez();
                props.push_str(&format!("{prop},"));
            )*
            props.pop();
            props.push(']');
            $crate::__tracing::span!(
                $crate::__tracing::Level::TRACE,
                "dom_helpers::tracing_props",
                props
            );
        }
    };
}

// Implementation based on spez
// see https://github.com/m-ou-se/spez

pub struct Match<T> {
    pub name: &'static str,
    pub value: std::cell::Cell<Option<T>>,
}

pub trait SerializeMatch {
    type Return;
    fn spez(&self) -> Self::Return;
}
impl<T: data_serialization::Serialize> SerializeMatch for &Match<&T> {
    type Return = String;
    fn spez(&self) -> Self::Return {
        let name = self.name;

        // suppresses warnings when serializing signals into props
        #[cfg(debug_assertions)]
        let _z = reactive_state::diagnostics::SpecialNonReactiveZone::enter();

        json_codec::to_string(self.value.get().unwrap_throw()).map_or_else(
            |err| format!(r#"{{"name": "{name}", "error": "{err}"}}"#),
            |value| format!(r#"{{"name": "{name}", "value": {value}}}"#),
        )
    }
}

pub trait DefaultMatch {
    type Return;
    fn spez(&self) -> Self::Return;
}
impl<T> DefaultMatch for Match<&T> {
    type Return = String;
    fn spez(&self) -> Self::Return {
        let name = self.name;
        format!(r#"{{"name": "{name}", "value": "[unserializable value]"}}"#)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/dom-helpers/functions/src/macro_helpers/tracing_property/match_primitive_75.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/dom-helpers/functions/src/macro_helpers/tracing_property/match_serialize_132.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/dom-helpers/functions/src/macro_helpers/tracing_property/match_no_serialize_151.rs"));
