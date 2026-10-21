#![allow(clippy::type_complexity)]

use std::cell::RefCell;
pub(crate) mod array;
pub(crate) mod datetime;
pub(crate) mod document;
pub(crate) mod error;
pub(crate) mod inline_table;
pub(crate) mod key;
pub(crate) mod numbers;
pub(crate) mod state;
pub(crate) mod strings;
pub(crate) mod table;
pub(crate) mod trivia;
pub(crate) mod value;

pub(crate) use crate::editor::error::TomlError;

pub(crate) fn parse_document<S: AsRef<str>>(
    raw: S,
) -> Result<crate::editor::ImDocument<S>, TomlError> {
    use prelude::*;

    let b = new_input(raw.as_ref());
    let state = RefCell::new(state::ParseState::new());
    let state_ref = &state;
    document::document(state_ref)
        .parse(b.clone())
        .map_err(|e| TomlError::new(e, b))?;
    let doc = state
        .into_inner()
        .into_document(raw)
        .map_err(|e| TomlError::custom(e.to_string(), None))?;
    Ok(doc)
}

pub(crate) fn parse_key(raw: &str) -> Result<crate::editor::Key, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    let result = key::simple_key.parse(b.clone());
    match result {
        Ok((raw, key)) => Ok(crate::editor::Key::new(key)
            .with_repr_unchecked(crate::editor::Repr::new_unchecked(raw))),
        Err(e) => Err(TomlError::new(e, b)),
    }
}

pub(crate) fn parse_key_path(raw: &str) -> Result<Vec<crate::editor::Key>, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    let result = key::key.parse(b.clone());
    match result {
        Ok(keys) => Ok(keys),
        Err(e) => Err(TomlError::new(e, b)),
    }
}

pub(crate) fn parse_value(raw: &str) -> Result<crate::editor::Value, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    let parsed = value::value.parse(b.clone());
    match parsed {
        Ok(value) => Ok(value),
        Err(e) => Err(TomlError::new(e, b)),
    }
}

pub(crate) mod prelude {
    pub(crate) use winnow::combinator::dispatch;
    pub(crate) use winnow::error::ContextError;
    pub(crate) use winnow::error::FromExternalError;
    pub(crate) use winnow::error::StrContext;
    pub(crate) use winnow::error::StrContextValue;
    pub(crate) use winnow::ModalParser;
    pub(crate) use winnow::ModalResult;
    pub(crate) use winnow::Parser as _;

    pub(crate) type Input<'b> =
        winnow::Stateful<winnow::LocatingSlice<&'b winnow::BStr>, RecursionCheck>;

    pub(crate) fn new_input(s: &str) -> Input<'_> {
        winnow::Stateful {
            input: winnow::LocatingSlice::new(winnow::BStr::new(s)),
            state: Default::default(),
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub(crate) struct RecursionCheck {
        #[cfg(not(feature = "unbounded"))]
        current: usize,
    }

    #[cfg(not(feature = "unbounded"))]
    const LIMIT: usize = 80;

    impl RecursionCheck {
        pub(crate) fn check_depth(_depth: usize) -> Result<(), super::error::CustomError> {
            #[cfg(not(feature = "unbounded"))]
            if LIMIT <= _depth {
                return Err(super::error::CustomError::RecursionLimitExceeded);
            }

            Ok(())
        }

        fn enter(&mut self) -> Result<(), super::error::CustomError> {
            #[cfg(not(feature = "unbounded"))]
            {
                self.current += 1;
                if LIMIT <= self.current {
                    return Err(super::error::CustomError::RecursionLimitExceeded);
                }
            }
            Ok(())
        }

        fn exit(&mut self) {
            #[cfg(not(feature = "unbounded"))]
            {
                self.current -= 1;
            }
        }
    }

    pub(crate) fn check_recursion<'b, O>(
        mut parser: impl ModalParser<Input<'b>, O, ContextError>,
    ) -> impl ModalParser<Input<'b>, O, ContextError> {
        move |input: &mut Input<'b>| {
            input
                .state
                .enter()
                .map_err(|err| winnow::error::ErrMode::from_external_error(input, err).cut())?;
            let result = parser.parse_next(input);
            input.state.exit();
            result
        }
    }
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/toml/editor/unit/src/parser/mod/test_143.rs"
));
