//! # Parser diagnostics
//!
//! This project uses parser errors and [`Parser::context`][crate::Parser::context]
//! to explain malformed input. The optional terminal tracing feature and its
//! instrumentation are not part of the maintained parser implementation.
//!
//! [`ContextError`][crate::error::ContextError] keeps structured context without
//! writing to process stderr or probing terminal dimensions.
