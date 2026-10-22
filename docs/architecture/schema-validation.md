# Schema capability boundary

`schema-validation` consolidates the implementation of schema values, ECMA pattern translation,
reference registries and offline JSON Schema validation. Implementations are private modules;
consumers use `values`, `patterns`, `references`, or the root validator API.

## Features

- No features: an empty, `no_std` capability boundary.
- `value`: generic typed JSON access, comparison and numeric semantics, without a validator,
  reference registry, URL dependency or regex engine. `json-codec` enables the JSON value backend.
- `pattern`: allocation-backed ECMA syntax validation and translation; the regex dependency has
  syntax and Unicode tables only, not `std`, a matcher or automata.
- `reference`: URI references, draft detection, registries and custom synchronous retrieval.
- `validator` (default): the offline validator, using value, pattern and reference capabilities.
- `arbitrary-precision`: exact large integer/rational semantics through the existing numeric and
  JSON codec implementation, without automatically selecting a validator.
- `jsonb`: typed PostgreSQL JSONB representation. `jsonb-testkit` additionally exposes the real
  retained byte encoder used by test fixtures.
- `resolve-async`: real asynchronous retrieval and reference resolution. Only futures allocation
  and utility capabilities are enabled; no executor, network client or blocking bridge is supplied.
- `idna`: opt-in domain normalization using `url-encoding/domain`, not the old URL/IDNA packages.
- `conformance`: typed-value accessor conformance helpers.

The URL encoding dependency is optional, enabled only by reference/validator capabilities with
its `std` edge. Domain tables remain opt-in. Default retrieval rejects external HTTP and file
resources. Provide a `Retrieve` or `AsyncRetrieve` implementation explicitly when needed.
Unbacked Python, Ruby, procedural-validator-macro and reqwest bridges are not features or APIs.

## Retained evidence

All 38 bundled metaschema JSON files remain byte-exact, including their standard identifiers.
Original source manifests, licenses and history must be retained by the guarded consolidation
receipt before retiring the four original package directories. Source construction alone is not
proof that retirement has been applied.

Original external tests remain byte-exact under `test/foundation/schema-validation/upstream`.
Authored runners live under `test/schema-validation`, separately from attributed historical evidence.
The canonical integration target adapts the original `run_case` assertions, replacing only its
unavailable test-registration macro with an ordinary runner. It executes all 31 JSON fixtures
and asserts their 1,509-case count. It does not substitute a no-op provider or weaken assertions.
The original macro-dependent unit/integration suites are retained as source evidence, not claimed
as fully executed by the focused gate.

Run `.github/scripts/check-schema-validation.py` for capability builds, dependency-graph checks,
default and arbitrary-precision canonical runs, and real asynchronous/typed JSONB runtime tests.
The numeric-precision and pattern-matching consumer gates exercise the consolidated APIs.
