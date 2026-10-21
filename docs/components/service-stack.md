# Service stack

`service-stack` owns the single `Service` and `Layer` definitions, layer composition, service adapters and opt-in middleware. The previous `tower`, `tower-layer` and `tower-service` package identities are not compatibility aliases.

## Boundaries

- No features: zero external dependencies. `Service`, `Layer`, `ServiceBuilder`, identity, stack and function layers remain available on native and browser WASM targets.
- `util`, `filter`, `load-shed`, `steer`: explicit pure service utilities; do not pull in Tokio.
- Runtime middleware stays behind its original explicit capabilities. `full` enables the maintained native middleware union, never an implicit default.
- No HTTP protocol parsing, TLS implementation, or unsafe boundary changed. The original `forbid(unsafe_code)` remains.

## Deliberate retirement

Unused hedge middleware was already nonfunctional because its `hdrhistogram` provider was absent. No maintained Cargo consumer enabled it. The broken `hedge` feature and its `full` activation are removed. Five original production files are preserved exactly in the consolidation receipt, not silently discarded. Historical hedge tests remain attributed and dormant; they are not counted as passing coverage.

## Validation and origins

`.github/scripts/check-service-stack.py` checks native/WASM feature separation, zero-dependency bare traits and actual layering, errors, cancellation and trait identity. It requires all four behavioral cases to pass; the gate does not manufacture missing upstream test providers. Other retained upstream tests requiring unavailable `tower-test` remain historical, not claimed as executed.

Origins: Tower 0.5.3, tower-layer 0.3.3, tower-service 0.3.3. Exact source, license, upstream version and test attribution are kept in `compliance/provenance`, the source catalog and the consolidation receipt.
