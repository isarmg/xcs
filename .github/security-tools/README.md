# Independent security scanner

Run `bash .github/scripts/security.sh`. The wrapper tests and invokes
`install-security-audit.py` before scanning the independent scanner lock, product and original upstream
identities. This tool is not a product workspace member or runtime dependency.

## Why a separate reviewed lock

The official cargo-audit 0.22.2 release ships a lockfile containing
rustls-webpki 0.103.13, affected by
[RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382.html).
The scanner's default gix/reqwest TLS chain can use the affected verifier on
Linux. System roots alone do not make that dependency immune.

`cargo-audit-0.22.2.lock` starts from the official release resolution. A real
self-audit also found 15 additional advisories in that old resolution, so the
reviewed scanner-only repair includes their minimum fixed versions and the
required compatible transitive closure:

- anyhow 1.0.102 → 1.0.103
- autocfg 1.5.0 → 1.5.1
- aws-lc-rs 1.16.3 → 1.18.2
- aws-lc-sys 0.40.0 → 0.46.0
- crossbeam-epoch 0.9.18 → 0.9.20
- faster-hex 0.10.0 → 0.10.1
- h2 0.4.14 → 0.4.16
- memmap2 0.9.10 → 0.9.11
- quinn-proto 0.11.14 → 0.11.18
- rand 0.9.4 → 0.10.3
- rand_core 0.9.5 → 0.10.1
- rustls 0.23.40 → 0.23.46
- rustls-webpki 0.103.13 → 0.103.16
- webpki-root-certs 1.0.7 → 1.0.10

Required additions: bitflags 1.3.2, chacha20 0.10.2, cpufeatures 0.3.1, defmt 0.3.101, defmt 1.2.0, defmt-macros 1.2.0, defmt-parser 1.1.0, pkg-config 0.3.34, rand_pcg 0.10.2, syn 3.0.9.

Removed obsolete dependencies: ppv-lite86 0.2.21, rand_chacha 0.9.0, zerocopy 0.8.48, zerocopy-derive 0.8.48.

The webpki update requires aws-lc-rs ^1.18 and its aws-lc-sys/pkg-config chain;
quinn-proto requires the rand 0.10 chain; faster-hex requires autocfg 1.5.1 and
introduces the optional defmt chain in the lock resolution. The new duplicate
crate names also require Cargo to qualify dependency references with versions.
The rand update migrates the WebAssembly feature edges from getrandom 0.3 to
0.4. Existing Windows dependency selections are preserved, avoiding Cargo
update's otherwise unrelated windows-sys edge downgrades.
Unrelated package versions, sources and checksums remain unchanged. The recipe
records all 66 exact changed objects, including reference disambiguations and
explicit removals. The final resolution contains 405 packages (original: 399).

It is a tool input, deliberately not named `Cargo.lock`; the product retains one root
workspace lockfile. The installed scanner still reports its actual version,
`cargo-audit 0.22.2`. This does not claim an upstream scanner release exists.
No advisory is ignored, no trust store is replaced and TLS is not disabled.

## Provenance and installation

`cargo-audit.json` records the exact archive URLs and SHA-256 values, the
original official lock SHA-256, the reviewed replacement lock SHA-256, and
the exact dependency closure. Every changed dependency checksum was verified
against both its official sparse index record and static.crates.io archive.
The official sources used to verify those checksums are:

- https://index.crates.io/ca/rg/cargo-audit (version 0.22.2)
- https://static.crates.io/crates/cargo-audit/cargo-audit-0.22.2.crate
- https://index.crates.io/ru/st/rustls-webpki (version 0.103.16)
- https://static.crates.io/crates/rustls-webpki/rustls-webpki-0.103.16.crate
- https://index.crates.io/aw/s-/aws-lc-rs and https://static.crates.io/crates/aws-lc-rs/aws-lc-rs-1.18.2.crate
- https://index.crates.io/aw/s-/aws-lc-sys and https://static.crates.io/crates/aws-lc-sys/aws-lc-sys-0.46.0.crate
- https://index.crates.io/pk/g-/pkg-config and https://static.crates.io/crates/pkg-config/pkg-config-0.3.34.crate

The installer verifies the pinned official archive and its original lock,
parses both lockfiles, and rejects any semantic difference beyond the exact
before/after package objects recorded in the recipe. It verifies the replacement bytes both before
and after `cargo install --path ... --locked`. Cargo validates downloaded
registry packages against the reviewed lock checksums. It never runs an
unlocked dependency resolution.

Source extraction and the Cargo working directory are in an external temporary
directory, outside the product's offline source replacement configuration.
Build output and the installed binary stay under ignored `target/security-tools`.
The repository's pinned Rust toolchain is used, with two build jobs. Normal
registry downloads and live advisory retrieval require network access.

A cached scanner is accepted only when its recorded recipe hash (installer,
recipe metadata, reviewed lock and toolchain file), binary SHA-256 and reported
version all match. A previous official binary with the same version but no
matching receipt is rebuilt. This is a local cache integrity check, not a
cryptographic attestation against an attacker who can rewrite the checkout
and receipt together.

## Updating and testing

For a future scanner release, obtain the official crate/index checksums,
review its lock and adjust the explicit dependency-change recipe.
Regenerate and review the snapshot and metadata together. Do not remove
`--locked`, relax semantic validation, or merely update the version string.

Run the network-free regression checks with:

```sh
python3 test/ci/test_security_audit_install.py
python3 test/ci/test_security_gate.py
```

These tests mock compilation; they are not evidence of a real scanner build or
live audit. A real installation followed by the complete security wrapper is
required to verify the executable and current product audit.

## Audit policy isolation

The scanner runs from a temporary working directory with this directory's
`audit.toml` copied to `.cargo/audit.toml`. In cargo-audit 0.22.2 that file is
selected instead of, not merged with, a caller/project or Cargo-home policy.
Absolute lock paths preserve the intended inputs. The policy requires live
advisory fetches, fresh registry/yank checks, all informational categories and
no ignored IDs or severity/target exclusions. The report validator also rejects
severity thresholds. `--color never` overrides CI color settings so an incomplete
registry check cannot hide its `error:` diagnostic behind ANSI formatting.

## Verification on 2026-10-20

The reviewed scanner dependency graph built successfully with the pinned
Rust 1.99.0 toolchain and `cargo install --path ... --locked` (two jobs).
Its executable reports `cargo-audit 0.22.2`. The official scanner source emitted
21 compiler warnings; these are upstream development-tool warnings, not a
claim that the product's strict zero-diagnostic checks were relaxed. No scanner
source was modified to suppress them. The offline installer and shell-wrapper
regression suite passes 19 tests, and the advisory classifier suite passes 12.

The reviewed repair covers these advisory IDs: RUSTSEC-2026-0185, RUSTSEC-2026-0186, RUSTSEC-2026-0190, RUSTSEC-2026-0204, RUSTSEC-2026-0258, RUSTSEC-2026-0285, RUSTSEC-2026-0306, RUSTSEC-2026-0338, RUSTSEC-2026-0339, RUSTSEC-2026-0340, RUSTSEC-2026-0342, RUSTSEC-2026-0346, RUSTSEC-2026-0348, RUSTSEC-2026-0349, RUSTSEC-2026-0382, RUSTSEC-2026-0383.

The final live wrapper run, with `CARGO_TERM_COLOR=always`, exited successfully:
405 independently locked scanner packages, 357 product packages and 341
original upstream identities were checked with no vulnerabilities, informational
findings or yank warnings. No exclusions or advisory ignores were used. The
final reviewed lock SHA-256 is
`fa524359a62e5f45daa2ed955c3a33dd669d85150e908dfefef357c66d880ba4`.
These are point-in-time results, not a promise about future advisories; each
wrapper invocation fetches current data and rechecks all three input sets.
