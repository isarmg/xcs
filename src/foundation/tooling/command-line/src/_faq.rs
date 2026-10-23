//! # Documentation: FAQ
//!
//! 1. [Comparisons](#comparisons)
//!    1. [How does `command_line` compare to structopt?](#how-does-command_line-compare-to-structopt)
//!    2. [What are some reasons to use `command_line`? (The Pitch)](#what-are-some-reasons-to-use-command_line-the-pitch)
//!    3. [What are some reasons *not* to use `command_line`? (The Anti Pitch)](#what-are-some-reasons-not-to-use-command_line-the-anti-pitch)
//!    4. [Reasons to use `command_line`](#reasons-to-use-command_line)
//! 2. [How many approaches are there to create a parser?](#how-many-approaches-are-there-to-create-a-parser)
//! 3. [When should I use the builder vs derive APIs?](#when-should-i-use-the-builder-vs-derive-apis)
//! 4. [Why is there a default subcommand of help?](#why-is-there-a-default-subcommand-of-help)
//!
//! ### Comparisons
//!
//! First, let me say that these comparisons are highly subjective, and not meant
//! in a critical or harsh manner. All the argument parsing libraries out there (to
//! include `command_line`) have their own strengths and weaknesses. Sometimes it just
//! comes down to personal taste when all other factors are equal. When in doubt,
//! try them all and pick one that you enjoy :). There's plenty of room in the Rust
//! community for multiple implementations!
//!
//! For less detailed but more broad comparisons, see
//! [argparse-benchmarks](https://github.com/rust-cli/argparse-benchmarks-rs).
//!
//! #### How does `command_line` compare to [structopt](https://github.com/TeXitoi/structopt)?
//!
//! Simple! `command_line` *is* `structopt`.  `structopt` started as a derive API built on
//! top of command_line v2.  With command_line v3, we've forked structopt and integrated it
//! directly into command_line.  structopt is in
//! [maintenance mode](https://github.com/TeXitoi/structopt/issues/516#issuecomment-989566094)
//! with the release of `command_derive`.
//!
//! The benefits of integrating `structopt` and `command_line` are:
//! - Easier cross-linking in documentation
//! - Documentation parity
//! - Tighter design feedback loop, ensuring all new features are designed with
//!   derives in mind and easier to change `command_line` in response to `structopt` bugs.
//! - Clearer endorsement of `structopt`
//!
//! See also
//! - [`command_line` v3 CHANGELOG](https://github.com/clap-rs/clap/blob/v3-master/CHANGELOG.md#300---2021-12-31)
//! - [`structopt` migration guide](https://github.com/clap-rs/clap/blob/v3-master/CHANGELOG.md#migrate-structopt)
//!
//! #### What are some reasons to use `command_line`? (The Pitch)
//!
//! `command_line` is as fast, and as lightweight as possible while still giving all the features you'd expect from a modern argument parser. In fact, for the amount and type of features `command_line` offers it remains about as fast as `getopts`. If you use `command_line`, when you just need some simple arguments parsed, you'll find it's a walk in the park. `command_line` also makes it possible to represent extremely complex and advanced requirements without too much thought. `command_line` aims to be intuitive, easy to use, and fully capable for wide variety use cases and needs.
//!
//! #### What are some reasons *not* to use `command_line`? (The Anti Pitch)
//!
//! Depending on the style in which you choose to define the valid arguments, `command_line` can be very verbose. `command_line` also offers so many finetuning knobs and dials, that learning everything can seem overwhelming. I strive to keep the simple cases simple, but when turning all those custom dials it can get complex. `command_line` is also opinionated about parsing. Even though so much can be tweaked and tuned with `command_line` (and I'm adding more all the time), there are still certain features which `command_line` implements in specific ways that may be contrary to some users' use-cases.
//!
//! #### Reasons to use `command_line`
//!
//!  * You want all the nice CLI features your users may expect, yet you don't want to implement them all yourself. You'd like to focus on your application, not argument parsing.
//!  * In addition to the point above, you don't want to sacrifice performance to get all those nice features.
//!  * You have complex requirements/conflicts between your various valid args.
//!  * You want to use subcommands (although other libraries also support subcommands, they are not nearly as feature rich as those provided by `command_line`).
//!  * You want some sort of custom validation built into the argument parsing process, instead of as part of your application (which allows for earlier failures, better error messages, more cohesive experience, etc.).
//!
//! ### How many approaches are there to create a parser?
//!
//! The following APIs are supported:
//! - [Derive][crate::_derive::_tutorial]
//! - [Builder][crate::_tutorial]
//!
//! Previously, we supported:
//! - [YAML](https://github.com/clap-rs/clap/issues/3087)
//! - [docopt](http://docopt.org/)-inspired [usage parser](https://github.com/clap-rs/clap/issues/3086)
//! - [`command_line_app!`](https://github.com/clap-rs/clap/issues/2835)
//!
//! There are also experiments with other APIs:
//! - [fncmd](https://github.com/yuhr/fncmd): function attribute
//! - [command_line-serde](https://github.com/aobatact/clap-serde): create a `Command` from a deserializer
//!
//! ### When should I use the builder vs derive APIs?
//!
//! Our default answer is to use the [Derive API][crate::_derive::_tutorial]:
//! - Easier to read, write, and modify
//! - Easier to keep the argument declaration and reading of argument in sync
//! - Easier to reuse, e.g. [clap-verbosity-flag](https://crates.io/crates/clap-verbosity-flag)
//!
//! The [Builder API][crate::_tutorial] is a lower-level API that someone might want to use for
//! - Faster compile times if you aren't already using other procedural macros
//! - More flexibility, e.g. you can look up the [argument's values][crate::ArgMatches::get_many],
//!   their [ordering with other arguments][crate::ArgMatches::indices_of], and [what set
//!   them][crate::ArgMatches::value_source].  The Derive API can only report values and not
//!   indices of or other data.
//!
//! You can [interop between Derive and Builder APIs][crate::_derive#mixing-builder-and-derive-apis].
//!
//! ### Why is there a default subcommand of help?
//!
//! There is only a default subcommand of `help` when other subcommands have been defined manually. So it's opt-in(ish), being that you only get a `help` subcommand if you're actually using subcommands.
//!
//! Also, if the user defined a `help` subcommand themselves, the auto-generated one wouldn't be added (meaning it's only generated if the user hasn't defined one themselves).
//!
