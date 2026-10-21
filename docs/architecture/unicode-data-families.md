# Unicode normalization and property families

Two independently governed runtime/data pairs now have one package each:

- `unicode-normalizer`: ICU4X `icu_normalizer` 2.3.0 and `icu_normalizer_data` 2.3.0.
- `unicode-properties`: ICU4X `icu_properties` 2.3.0 and `icu_properties_data` 2.3.0.

The maintained Rust crate names are `unicode_normalizer` and `unicode_properties`.
There are no old crate-name compatibility aliases. Shared `icu_provider`, collections,
locale-core and zero-copy types remain separate. The optional normalizer → properties
edge stays optional; the property family has no inverse normalizer dependency.

## Source and provenance boundaries

Each runtime lives in `src/foundation/unicode-*`; original external tests live under
`test/foundation/unicode-*`. Additional product-focused checks are in
`test/unicode-data`. Production code and generated tables are under
`src`; the inherited build script remains at the Cargo package entrypoint.

`src/compiled_data.rs` includes `src/data/mod.rs` only with `compiled_data`. All
original generated table files, filenames, marker versions, serialized data keys,
and table contents are byte-identical. Their `v1` suffixes describe ICU data formats,
not a project release version, and must not be renamed. The provider's local `icu`
module is the generated tables' implementation namespace, not a compatibility crate.

The exact upstream origins, manifests, licensing and version histories remain in
`compliance/provenance`, `compliance/licenses`, `docs/upstream` and the immutable
multi-origin consolidation receipts. Generated data records CLDR 48.2.1, ICU
release-78.1rc and LSTM v0.1.0 in its original source comments. The inherited
`ICU4X_DATA_DIR` build input and Cargo rerun trigger are retained byte-for-byte.

## Supported product and CI scope

Provider-only builds remain `no_std` without compiled tables. `compiled_data` adds
real baked data/provider support; property `alloc` is tested independently through
its behavior graph. Both minimum graphs compile on Linux amd64 and browser WASM.
Normalizers default to compiled tables and UTF-8 iteration. The inherited default
previously selected `utf16_iter` despite its `utf16_iter` and `write16` dependencies
already being absent; that broken selection is no longer a default.

The unsupported historical UTF-16, datagen and third-party trait Cargo surfaces have
been retired in a separate bounded follow-up. Serialization remains selectable.
The historical complete normalizer test file is preserved with exact inverse migration
proof but is not reported as executed: it requires unavailable UTF-16 providers.
No fake provider or no-op implementation is supplied to make those tests appear green.

`.github/scripts/check-unicode-data-family.py` supports each family independently:

- Assert table bytes and build inputs against the consolidation input receipt.
- Isolated provider-only/compiled-data graphs, native/WASM compilation, and negative
  checks that provider-only APIs do not expose `Baked`.
- Compile the real custom-data path using the unchanged generated tables.
- Normalization conformance over every retained corpus row's five columns and all
  four forms, plus supplementary-plane, Hangul, combining-mark and UTS-46 vectors.
- Property category/script/bidi/binary-property vectors, all 1,112,064 Unicode scalar
  comparisons between baked-provider and constant constructors, and 11 retained
  property unit tests using their real dependencies.

After URL consolidation, the URL family gate runs the retained
`property_masks_preserve_all_unicode_scalars` regression in its Unicode backend. Generated-data package
removal does not change ICU scalar values, property bit masks or IDNA wire semantics.


## Explicit optional-capability limits

The normalizer no longer declares `utf16_iter`, `write16`, `harfbuzz_traits` or
`datagen`. Properties no longer declares `harfbuzz_traits`, `unicode_bidi` or
`datagen`. Selecting any of these seven package/feature pairs fails at Cargo feature
resolution, before compilation. No compatibility alias or substitute provider exists.
The normalizer-to-properties datagen forwarding edge is removed with the features;
all retained Cargo manifests are scanned for consumers and indirect forwarders.

Dormant implementation bytes and historical test sources remain intact. Explicit
`check-cfg` declarations acknowledge historical names without enabling them. Original
upstream documentation remains immutable archival evidence, while the package readme
and active crate documentation describe the maintained capability boundary.

The maintained Unicode gate checks the exact supported feature sets, provider-only
serialization, serialization with normalizer/property integration or property `alloc`,
and all supported features together on native and WASM targets. It also runs seven
isolated negative feature-resolution probes. This is supported-profile compilation,
not serialization wire-format/runtime interoperability certification, and not execution
of dormant historical UTF-16, datagen or trait-integration tests. The existing scalar,
normalization, unit and executable-documentation checks remain separate.

This is an ordinary content refresh under the same source owners, not a package
retirement, rename, move or upstream upgrade. Original origin/legal material,
consolidation receipts and authenticated input history stay unchanged. A separate
explicit lifecycle adapter would be required for a later source-owner lifecycle change.

Executable documentation uses the real public crate/module paths. The normalizer
doc profile selects `compiled_data,utf8_iter,unicode-properties`; properties selects
`compiled_data,alloc,unstable`, because its documented parser namespace is public
only under `unstable`. Minimal/provider-only graph tests stay separate. The
alloc-only hidden parser module's doctest configuration is not certified. The
retained ignored examples remain ignored; no dormant historical tests are claimed.
