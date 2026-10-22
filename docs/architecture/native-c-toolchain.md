# Retained native C build toolchain

The internal `cc` component runs on Linux amd64 hosts. It accepts Linux amd64
(excluding the x32 ABI) and `wasm32-unknown-unknown` browser compilation targets.
This is a project support contract, not an assertion that upstream cc lacks other
platforms.

## Exact consumers and retained behavior

- `ring` uses C and GNU assembler compilation for native cryptography.
- `blake3` uses native x86 assembly / compiler capability probes. Its browser SIMD
  implementation is Rust, not a C/Clang build.
- `libsqlite3-sys` uses `cc` when bundled SQLite is selected.
- `bindgen` and `cc` both use the retained `shell-tokenizer` component. The Samba
  SDK bridge uses bindgen and its extra-Clang-argument parsing; neither consumer
  can be replaced with whitespace splitting.

GNU/Clang discovery, explicit compiler and archiver overrides, compiler wrappers,
compiler-family probes, supported-flag probes, assembly flags, C++ handling,
quoted flag tokenization and browser Clang target selection remain intact.
Shared compiler-family recognition and flag construction are not removed merely
because an upstream branch mentions another platform.

## Removed capability

`find-msvc-tools` had no workspace consumer except `cc`. Windows registry, COM,
Visual Studio / Windows SDK discovery, MASM dispatch and MSVC archiver discovery
are removed. Unsupported compiler targets fail explicitly, not by falling back
to the Linux host compiler.

No retained consumer enabled cc's `parallel` feature. Its existing declaration
was unusable: implementation referenced the already-absent jobserver package.
The unused parallel/jobserver features, optional libc edge, module declaration
and active implementation branches are removed rather than reintroducing that
dependency. The eight original `src/parallel/` files remain byte-exact in place,
unreachable and uncompiled. Physical retirement of this historical source is
pending a separately reviewed general provenance mechanism: the existing tools
permit whole-package retirement, but not these partial deletions. No fake
consolidation or relaxed source-removal checks are used.
C compilation remains serial, as in all retained consumers before this change.
Cargo still schedules independent packages normally.

## Preservation and regression

The complete last `find-msvc-tools` source and external tests are in the pinned
retirement archive, checked by the existing historical-source verifier. Original
upstream identity, metadata, documentation and licenses remain unchanged.
The complete pre-specialization cc source and external tests are in
`compliance/cc-1.8.0-specialization.json.gz`; the native-toolchain check authenticates
its fixed digest, exact file hashes and still-present license bytes.

`python3 .github/scripts/check-native-c-toolchain.py` is part of the
`foundation-platform` shard. It checks a real native BLAKE3 known answer, compiles
C/C++/GNU assembly with flags and paths containing spaces, links and executes the
result, checks supported/unsupported compiler flags, and rejects unsupported
targets and the removed Cargo features. Its Rust build uses the shared strict
Cargo diagnostic checker. It also runs cc's retained unit tests and doctests,
checks explicit CC/CFLAGS overrides and complete forwarding of long compiler
stderr lines, and pins the unreachable parallel source bytes. This does not pretend to be a Samba SDK or wasm C
compiler test; those require the actual configured SDK or Clang toolchain.
