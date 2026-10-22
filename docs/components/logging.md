# Logging

A single internal capability owns environment-driven configuration, log directive
parsing and matching, formatting, and synchronized stdout/stderr/custom-pipe output.
The web-bindings build tool initializes this logger. The shared terminal-style
capability still owns ANSI style and output adaptation; this package does not
copy or replace it.

```rust
use logging::Builder;
let logger = Builder::new().filter_level(log::LevelFilter::Warn).build();
assert_eq!(logger.filter(), log::LevelFilter::Warn);
```

The default features enable automatic color and regular-expression message filters.
With regex disabled, message filters remain literal substring matches. Logging
always uses std, including parser errors and forgiving directive parsing. Target
prefix precedence, default error level, environment variable names and output
formatting remain unchanged. `filter` exposes the underlying filter builder for
custom logger composition without a separate Cargo package.

Original env_logger 0.11.11 and env_filter 2.0.0 identities, MIT/Apache-2.0 licensing,
original documents and source/test evidence remain in compliance provenance.
