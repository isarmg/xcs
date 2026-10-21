# Numeric foundation

The `numeric` component groups primitive numeric traits and optional mathematical
value types. It replaces the seven num-family Cargo packages, not their upstream
identities. Every input's license and source attribution remains independently
recorded by the consolidation receipt and immutable provenance.

## Boundaries

- `traits` is always available and re-exported at the crate root. Default features
  enable `std` only. Trait-only users do not compile big integers, complex values,
  rational values, or iterators, and have no runtime dependencies.
- `integer` adds integer algorithms, roots and averages.
- `iter` requires `integer`; `complex` is independent of allocation.
- `rational` requires `integer` and supports fixed-size ratios without allocation.
- `bigint` requires `alloc` and `integer`; rational big-integer aliases become
  available when both `rational` and `bigint` are selected.
- `collection` is the traditional broad mathematical facade without automatically
  enabling heap allocation. Consumers explicitly select `bigint` when needed.
- `serialization` uses the real maintained `data-serialization` provider and
  activates adapters only for the value modules already selected.
- `std` does not imply bigint or allocation. Native Linux amd64 and browser WASM
  are checked separately with default features disabled.

`autocfg` remains a build dependency. The original build script probes floating
point total ordering availability. It runs on the host and is not linked into
trait-only or WASM consumers. There is no dependency back into numeric.

## Consumers

`atoi` needs primitive traits; `deranged` enables its optional numeric traits
support through one optional dependency. `fraction` uses the collection facade,
with its `with-bigint` feature enabling bigint and std. `jsonschema` and
`jsonschema-value` use primitive traits and activate bigint only for arbitrary
precision. Absolute external crate paths distinguish this component from their
existing internal `numeric` modules.

## Removed unsupported surfaces

The former manifests advertised libm, rand, arbitrary, quickcheck, rkyv,
bytecheck, and bytemuck integrations without including their actual providers.
They were not part of the resolved product graph. These nonfunctional features
are no longer advertised; no stub providers or permanently-false features are
introduced. Four wholly provider-specific implementation files are archived
byte-for-byte through the reviewed source-archive route. Provider-specific
inline code is removed from retained sources. Ordinary no_std float-core,
integer/rational/complex arithmetic, bigint operations and real serialization
remain. The obsolete no-op i128 compatibility feature is removed; i128 support
is unconditional, as it already was in the code.

## Origins

- num 0.4.3: https://github.com/rust-num/num
- num-bigint 0.4.8: https://github.com/rust-num/num-bigint
- num-complex 0.4.6: https://github.com/rust-num/num-complex
- num-integer 0.1.47: https://github.com/rust-num/num-integer
- num-iter 0.1.46: https://github.com/rust-num/num-iter
- num-rational 0.4.2: https://github.com/rust-num/num-rational
- num-traits 0.2.19: https://github.com/rust-num/num-traits

All seven use MIT OR Apache-2.0. The maintained Cargo version is the primary
num-traits version, not a claim that all origins share a version. See immutable
compliance provenance and upstream documentation for original legal materials.

## Validation and integration status

The maintained CI gates check migrated unit and explicitly wired upstream
integration tests, actual consumer compilation, and the minimal native/WASM
feature matrix. Six fewer Cargo packages is a structural reduction, not a
benchmark result.

The consolidation receipt binds the prior num-bigint serialization-file move
through the exact `input_history` record. The original receipt remains immutable,
and source verification checks the current package inventory separately.

The precision consumer gate independently checks jsonschema-value and jsonschema
feature forwarding and nine exact-value regressions, including integers beyond
u64, rational numerators/denominators, multipleOf and exponents 1e400 and 1e-400.

Nine upstream benchmark source files remain byte-identical under
`origins/num-bigint/benches` and `origins/num-integer/benches`. They are inert
upstream test material, not compiled production implementations or tested
benchmarks; `autobenches = false` is retained. Their historical inventory must
not be silently discarded.
