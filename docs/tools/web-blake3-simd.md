# Optional Web BLAKE3 SIMD correctness gate

Run from the repository root on Linux amd64:

```sh
bash test/evidence/web-blake3-simd/run-check.sh
```

An optional first argument selects another read-only source repository. Requires an existing Rust toolchain matching the repository, the installed `wasm32-unknown-unknown` target, Node with WebAssembly SIMD, a C toolchain for native BLAKE3, and Bash/coreutils. No tools or dependencies are downloaded. The script copies foundation sources into a temporary directory, materializes its Cargo.toml.in template outside the repository and generates its own fixture lockfile, builds offline with one job, and removes its temporary files on exit. It does not modify or build the source repository.

The fixture uses the Web app's wasm32-only BLAKE3 feature pattern, builds native and portable/SIMD Wasm variants, and checks that host feature resolution excludes wasm32_simd. SIMD module metadata must declare simd128; the portable reference must not. Deterministic input is generated in a bounded 64 KiB buffer, independent of update boundaries.

Coverage: 18 lengths × 4 update sizes = 72 cases, and 144 Wasm digest comparisons against native BLAKE3. Lengths cover empty input, 64-byte blocks, 1024-byte BLAKE3 chunks, four-way SIMD boundaries, 64 KiB updates, 1 MiB, and 2 MiB + 17 bytes. Update sizes are 63, 1024, 4096, and 65536 bytes. The known empty-input BLAKE3 digest is independently asserted.

A focused run with Rust 1.99.0 and Node v24.19.0 passed all 72 cases / 144 comparisons. This is an optional manual gate, not wired CI, a production Web app build, or a browser/throughput benchmark. It checks the copied BLAKE3 implementation and fixture target guards, not the production dependency graph. Existing production build and browser-smoke gates must validate the composed Web app, including real transfer integrity. SIMD-enabled production modules require SIMD-capable browsers; this fixture does not provide a legacy-browser fallback.
