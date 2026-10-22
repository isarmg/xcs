# Terminal style capabilities

`terminal-style` is the single owner of ANSI styles, terminal parsing, native output adaptation, and process-global color policy. The old anstyle, anstyle-parse, anstyle-query, colorchoice, and anstream package boundaries are removed, not retained as forwarding crates.

- Defaults: dependency-free, no_std style types and one atomic ColorChoice policy.
- `std`: style writers using std::io.
- `parser`: no_std ANSI parser using alloc and ASCII character accumulation.
- `parser-core`: fixed-capacity OSC storage through arrayvec, without alloc.
- `parser-utf8`: UTF-8 character accumulation through utf8parse.
- `output`: explicit std output capability with UTF-8 parser support, consumed by CLI color and logger color features. Linux amd64 is the native runtime validation target; browser WASM compiles, but native stream detection is not a browser-console bridge.

The parser transition table and terminal environment queries are private implementation details. Consumers use `terminal_style::output::{AutoStream, stream::IsTerminal}` and root `terminal_style::ColorChoice`. CLI styles and logging now use the same style types and color-policy state. Canonical ANSI escapes and NO_COLOR, CLICOLOR, CLICOLOR_FORCE, TERM and CI variables are unchanged.

Unsupported Windows console adapters/query implementation and unused stream-formatting macros are removed. Exact old source bytes, hashes and paths are preserved in compliance provenance and the authenticated consolidation receipt. Original upstream documentation, licenses, versions, origin hashes and manifests are not rewritten. All 13 original external test files remain attributed and outside production source. Unavailable upstream proptest/code-generation tooling is retained as test provenance, not counted as executed tests; owned deterministic protocol and byte-boundary tests execute without those dependencies.

Run `.github/scripts/check-terminal-style.py` for seven native feature combinations, four WASM combinations, dependency-free default resolution, real CLI/logger consumers, and negative API/feature checks. The foundation-consumers harness independently checks real output stripping, passthrough and terminal detection.
