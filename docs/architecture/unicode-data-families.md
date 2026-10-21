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

The retained historical UTF-16, datagen, serialization and third-party trait feature
surfaces are not newly certified by this consolidation. The historical complete
normalizer test file is preserved with exact inverse migration proof but is not
reported as executed: it requires the unavailable UTF-16 providers. No fake provider
or no-op implementation is supplied to make those tests appear green.

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

Focused native compile probes of this receiving tree found `serialization` builds
for both families, but serialization runtime interoperability is not certified.
`utf16_iter` lacks the `utf16_iter` and `write16` providers; `harfbuzz_traits` in
both families lacks its trait provider; properties `unicode_bidi` lacks its
provider. Both `datagen` selections fail in the retained `zerovec` dependency
because `databake` is absent. Normalizer `write16` alone is an empty no-op feature,
not a working UTF-16 implementation. These inherited surfaces remain unsupported;
this consolidation does not claim an all-features build or supply fake providers.
Retiring them requires a separate bounded API/source provenance change.

Executable documentation uses the real public crate/module paths. The normalizer
doc profile selects `compiled_data,utf8_iter,unicode-properties`; properties selects
`compiled_data,alloc,unstable`, because its documented parser namespace is public
only under `unstable`. Minimal/provider-only graph tests stay separate. The
alloc-only hidden parser module's doctest configuration is not certified. The
retained ignored examples remain ignored; no dormant historical tests are claimed.
