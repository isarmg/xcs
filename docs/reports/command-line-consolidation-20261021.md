# Command-line component: two compiler units, one runtime

The facade, command builder/parser and OS-string lexer form one command-line runtime crate. Only command-derive remains separate because Rust procedural macros are host-side compiler extensions and cannot be the same normal runtime library target. There are no independent consumers requiring builder or lexer packages. Product consumers use command-line; the private lexer is not re-exported.

The builder's existing module graph and macro definitions are integrated at the runtime crate root. Both old runtime lib.rs files map explicitly to the resulting lib.rs, rather than introducing a wrapper that trips Rust macro-export restrictions. All other implementation files retain exact mapped destinations. Two redundant manifests and two unused wrapping implementation files have explicit archival dispositions, with exact last-current bytes and reviewed reasons in the authenticated receipt. No deleted implementation is relabeled as a retained refactor. Four original upstream identities remain under two active owners.

The private lexer already contained unsafe OS-string slicing. The merged runtime uses deny(unsafe_code) and permits it only inside that imported private lexer module; no new unsafe algorithm/site is introduced. All original safety comments remain. This is an explicit moved trust boundary, not a claim that the runtime contains no unsafe implementation.

Legacy catch-all helpers and the versioned preview umbrella remain removed. Canonical modern parser behavior stays as already tested, retaining prior help-width behavior. Immutable upstream source archives, license material, and origin/version provenance remain unchanged.

Private tokenizer examples are maintained as in-crate tests instead of keeping a redundant public crate/API for doctests. Thirteen lexer examples execute, four remain compile-only, and the one upstream ignored example stays explicitly ignored. With ten maintained command contracts, the fixture executes 23 tests and retains all three negative helper probes. Actual storage-admin/web-build CLI goldens stay byte-identical; web-bindings' real CLI tests run separately.

Source review inventory: two-consolidation-inventory.json, with 147 production routes and 25 external-test routes. Exactly one external fragment requires a reversible import edit; all builder external test fragments remain byte-identical. The shared consolidation helper validated the initial runtime merge. The subsequent explicit source archives and final current-baseline rebase must validate before any apply bundle is authorized. No current root catalog/lock or shared CI file is copied wholesale from this prototype.

## Verified compiler and documentation boundaries

The gate compiles isolated copies of the actual merged runtime with an unsafe block
injected first into the crate root and then into the builder module. Both fail
specifically on the unsafe-code lint. The sole intentional allowance remains the
private lexer module containing the existing OS-string implementation. An external
import of `command_line::lexer` also fails specifically because the module is private.
No upstream unsafe implementation was added or expanded.

Normal merged-runtime doctests: 355 passed, 3 upstream ignored. One external doctest
import required the public crate path after merging. The styling example now uses
equivalent ANSI escape strings directly rather than the unavailable upstream
`color_print` development dependency. The inherited unsupported `unicode`, `wrap_help`, and `unstable-markdown` flags
were removed, not replaced. Their missing providers were unicode-width,
terminal_size, and pulldown-cmark; no actual product consumer enabled these flags.
Case-insensitive matching remains ASCII-only, scalar-count help width and source
line breaks remain unchanged, and doc comments use plain paragraph processing.
The doc umbrella no longer forwards unavailable capabilities.

Every retained runtime and derive feature compiles together. A real debug error
path verifies that std::backtrace::Backtrace::force_capture captures the executing
test frame. This replaces the absent third-party backtrace provider only for
opt-in debug diagnostics; textual backtrace formatting may differ. Four Cargo
negative probes reject the removed flags. No new dependency is introduced.

## Private tokenizer surface

Tokenizer declarations are crate-scoped rather than unreachable public exports.
The argument-environment constructor exists only for the preserved compile-only
examples under `cfg(test)`. Two unused end/empty convenience methods were removed;
no blanket dead-code allowance was introduced. All five original unsafe blocks
remain unchanged inside the explicitly allowed private module.
