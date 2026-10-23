# Independent security scanner

Run `bash .github/scripts/security.sh`. The wrapper tests and invokes
`install-security-audit.py`, then scans the independent scanner lock, product
lock and original upstream identities. This tool is not a product workspace
member or runtime dependency.

## Pinned inputs and provenance

The scanner is `cargo-audit 0.22.2`, built with the reviewed
`cargo-audit-0.22.2.lock` rather than the official release's vulnerable dependency
resolution. The lock is a tool input, deliberately not named `Cargo.lock`; the
product retains one root workspace lockfile. No advisory is ignored, no trust
store is replaced and TLS is not disabled.

`cargo-audit.json` is the authoritative recipe. It records the exact official
archive URLs and SHA-256 values, original and reviewed lock hashes, and every
allowed dependency change. Dependency checksums are verified against the
official crates.io sparse index and static.crates.io archives. Do not duplicate
that version-by-version inventory in this manual.

The installer verifies the official archive and its original lock, parses both
lockfiles, and rejects semantic differences beyond the exact package objects
in the recipe. It verifies replacement bytes before and after
`cargo install --path ... --locked`. Cargo validates downloaded registry
packages against the reviewed checksums; installation never uses an unlocked
resolution.

## Installation and cache

Source extraction and Cargo's working directory use an external temporary
directory, outside the product's offline source replacement configuration.
Build output and the installed binary stay under ignored
`target/security-tools`. The repository's pinned Rust toolchain is used, with
two build jobs. Registry downloads and live advisory retrieval require network
access.

A cached scanner is accepted only when its recipe hash (installer, metadata,
reviewed lock and toolchain file), binary SHA-256 and reported version match.
A binary without a matching receipt is rebuilt, even if it reports the same
version. This is a local cache integrity check, not an attestation against an
attacker who can rewrite both the checkout and receipt.

## Audit policy isolation

The scanner runs from a temporary directory with this directory's `audit.toml`
copied to `.cargo/audit.toml`. In cargo-audit 0.22.2 that policy is selected
instead of, not merged with, caller/project or Cargo-home policies. Absolute
lock paths preserve the intended inputs.

The policy requires live advisory fetches, fresh registry/yank checks, all
informational categories and no ignored IDs or severity/target exclusions.
The report validator also rejects severity thresholds. `--color never`
overrides CI color settings so an incomplete registry check cannot hide an
`error:` diagnostic behind ANSI formatting. Every invocation rechecks all
three input sets; an earlier clean result is not a current security guarantee.

## Updating and testing

For a scanner update, obtain official crate/index checksums, review the lock
and adjust the explicit dependency-change recipe. Review the replacement lock
and metadata together. Do not remove `--locked`, relax semantic validation or
merely update the reported version string.

Run the network-free regression checks:

```sh
python3 test/ci/test_security_audit_install.py
python3 test/ci/test_security_gate.py
```

These tests mock compilation. They do not prove a real scanner build or live
audit. Verify those separately by running the complete security wrapper with
network access and retaining the output for the exact candidate being checked.
