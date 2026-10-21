# TOML configuration

One `toml` Cargo package owns five original components. Modules separate capabilities rather than keeping alias packages:

- `datetime`: dates, times and offsets, always available.
- `span`: values with original byte ranges, always available.
- `writer`: low-level encoding, enabled by `write`; `alloc` adds allocated output conveniences.
- `editor`: format-preserving model, enabled by `edit`; `parse` adds winnow, `display` adds the writer.
- Root `Value`, `Table`, serialization/deserialization and conversion APIs are enabled by `serialization`; defaults retain serialization, parse and display.

With default features disabled, datetime/span have no third-party production dependencies. Neither requires the editor, indexmap, winnow or serialization engine. Serialization alone does not require editing. Native Linux amd64 and browser WASM are maintained. This unified package is a std-based component; the old standalone writer's embedded/no_std support is not advertised. `alloc` and `std` select writer conveniences, not whole-package no_std guarantees.

The supported `preserve_order` and `unbounded` options retain their semantics. The old `perf` option referenced an absent kstring provider and is intentionally removed rather than emulated. Serialization's six private TOML/span marker strings remain exact. Error spans remain byte offsets. The editor's existing behavior canonicalizes CRLF to LF and adds a final newline; consolidation does not promise byte-for-byte CRLF preservation.

The editor retains its existing, narrowly scoped UTF-8 fast paths, whose inputs are guaranteed valid by parser boundaries. All other implementation modules prohibit unsafe code. The public parser accepts `&str`, not arbitrary byte buffers.

## Sources

Immutable origin records, original manifests, upstream READMEs and MIT/Apache-2.0 licenses are retained for toml 0.8.23, toml_datetime 0.6.11, toml_edit 0.22.27, toml_write 0.1.2, and serde_spanned 0.6.9 (previously maintained as source-spanned-value). The existing serde_spanned rename receipt is not rewritten. The new consolidation receipt binds that exact previous lineage to the new owner.

## Test scope

Original 16 external fragments retain original source paths/hashes and ownership through authenticated reversible transfer. The raw snapbox comparison is replaced by exact `assert_eq!`, retaining its inputs. Two original proptest generators remain as attributed, disabled source because no proptest provider exists. Separate deterministic tests check every Unicode scalar and explicit boundary strings; these do not reproduce randomized generation, string combination coverage or shrinking.

Focused tests cover source byte spans, formatting, malformed TOML, Unicode boundaries, datetime precision/offsets and all six serialized marker names. Native/WASM feature checks validate minimal capability closures. Actual consumers are release-tools and contract-check. Upstream benchmarks and unavailable external corpus harnesses are not claimed as executed.

The foundation suite runs `.github/scripts/check-toml-consolidation.py`. It rejects
Rust warnings in all 30 target/feature builds (including the complete feature union), checks capability-negative
compiles and the dependency-free primitive graph, and executes the retained unit,
boundary and documentation tests with real development dependencies in a temporary
byte-identical source copy. Only that temporary fixture generates its own lock;
the product root lock and source inventories are not modified. Parser-only span
storage and formatting-only helpers are compiled only with their owning feature,
rather than suppressing unused-code diagnostics.
