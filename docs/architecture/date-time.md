# Date/time capability

`date-time` owns calendar arithmetic, timestamps, UTC and explicit fixed offsets,
format descriptions, parsing, formatting, and optional serialization. It replaces
five packages (`time`, `time-core`, `deranged`, `powerfmt`, `num-conv`) with one
functional package, not forwarding wrappers or compatibility Cargo aliases.
The Rust import is `date_time`; standard-library `std::time`/`core::time` and the
tracing collector's private `fmt::time` namespace remain distinct.

Calendar units, ranged integer invariants, width-aware display and integer
conversions now share one compilation/feature boundary. Support modules are
private except for intentionally exported calendar units and utility functions.
The numeric helper retains the widening and low-bits truncation contracts actually
used by date/time arithmetic, not deprecated conversion aliases or unused
saturating/checked extension APIs. Unused standalone formatter buffer/extensions
are removed. Ranged arithmetic keeps one uniform generated implementation across
integer widths; narrowly scoped dead-code allowances account for width/feature
selection, without weakening the warning gate. The 68 hyphenated lint names from
the time manifest become underscore spellings with unchanged levels/priorities.
`check-cfg` remains Cargo's hyphenated configuration key.

## Supported consumers and features

- storage-engine uses formatting/parsing for stored container timestamps.
- storage-server uses formatting for wire timestamps and telemetry.
- tracing's `time` feature enables date-time formatting for its real `UtcTime`
  and `OffsetTime` collectors. A fixed offset is supplied explicitly.
- The serialization gate exercises the actual data-serialization/json-codec
  providers, including a date-time JSON round trip.
- `std` (default) enables `alloc`. No-default-feature calendar/duration arithmetic
  requires neither allocator nor standard library. Parsing is available without
  either; allocated format descriptions require `alloc`.
- `formatting` requires `std`; `large-dates` expands year ranges locally, without
  cross-package feature forwarding.
- `serialization` uses data-serialization directly, including private ranged
  representations. Human-readable and well-known adapters additionally enable
  formatting/parsing. Compact serialization does not require `std`.
- `wasm-bindgen` retains the actual js-sys Date interoperability and clock path
  for browser WASM. Linux amd64 is the native platform; browser WASM is retained
  for the product's web client, not as another native platform.

## Explicit unsupported boundary

There is no OS-local-timezone discovery or timezone-refresh API. The previously
advertised `local-offset` feature referenced an absent `num_threads` backend and
had no product consumer. Its feature, local-offset error variant, local clock and
TZ-refresh methods and platform implementations are removed and archived in the
consolidation receipt. UTC is never substituted for a requested system offset.
Callers provide a fixed offset, or use UTC. Named zones, DST rules and changing
local timezone resolution require a separately designed capability.

The unbacked random, property-test and procedural-macro features are absent.
No date/time procedural macro package was vendored or merged. Use constructors
and runtime format-description parsing, not an advertised unavailable macro.
Negative gates verify unsupported features, local-time methods and private
implementation modules cannot be consumed.

## Evidence and limits

`.github/scripts/check-date-time-family.py` checks native/WASM isolated feature
closures, calendar and timestamp boundary behavior, real serialization, and
actual tracing UTC/+05:30 timer formatting. WASM checks are compilation evidence,
not browser execution. Existing tracing and serialization gates retain their
owned consumers and now resolve the date-time package.

All original provenance stays immutable. External historical tests are relocated
with exact reversible import/config edits, not silently rewritten as new tests.
Project capability tests live outside `src`, under `test/date-time`.
The 17 historical benchmark source files stay byte-identical and inert pending a
separate audited relocation. They are not executed and no speed, binary-size or
memory improvement is claimed from package consolidation. Historical upstream
integration/doctest material is not equated with the executable project gates.

The feature-isolated inherited default unit target executes 64 tests. The ISO
fixture now uses two ordinary table-driven tests instead of an unavailable
`rstest` provider: all 23 configuration cases retain their six assertions, and
all seven invalid encodings are checked independently. The receiving workspace's
`cargo test -p date-time --lib` passes 66 tests with formatting enabled. Original
source attribution is unchanged; the [bounded fixture inverse proof](../reports/date-time-iso-fixture-proof.json)
records the reviewed transformation separately from frozen consolidation history.
The historical all-feature unit target still references unavailable `serde_test2`
and the old JSON test harness. Feature-isolated project gates independently
exercise the real serialization, formatting and parsing providers; they are not
presented as a successful complete upstream all-features unit suite.

The new byte-count regression exposed and fixes an existing RFC3339 bug:
`format_into` wrote fractional-second digits without counting them in its return
value. Its returned count now includes those digits. Tests compare reported and
actual byte counts for 1-, 3- and 9-digit fractions in UTC and +05:30, including a
pre-epoch instant. This is a correctness fix, not a performance claim.
