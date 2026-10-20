# Rust quality policy

`check.sh quality` checks formatting and runs Clippy with warnings denied for each
first-party workspace. Package selection deliberately avoids reformatting third-party
path dependencies. The native host receives its Clippy check in the platform job,
after system dependencies and generated Web assets are available. Samba VFS still
requires its configured SDK; the portable protocol crate is checked here. The
independent `smb-sdk` job builds the pinned official Samba SDK, then runs strict
VFS Clippy, linked tests and the release build against that real SDK.

`check.sh security` installs the exact `cargo-audit` 0.22.2 release with its published
lockfile and audits every consumer lockfile against current RustSec data. Because
RustSec skips path packages, a separate synthetic upstream-identity lock also
covers maintained vendor forks, including renamed framework crates. Its package
identities and checksums must match verified provenance exactly; it is never used
for builds. Newly reported upstream advisories fail instead of disappearing behind
a local path dependency. Findings,
including unmaintained or yanked dependencies, fail the job. There are no advisory
ignore lists. A reviewed local backport may be explicitly classified only after
its exact source, patch hashes and resolved dependency identity are verified; the
original advisory remains visible in the output. An unavailable registry or advisory database fails rather than giving
a false clean result. The scanner installation is reproducible; the security result
intentionally depends on the latest advisories. This is not a license-policy audit.

## Scoped API migration debt

A small set of functions uses `#[expect(clippy::too_many_arguments)]`, with reasons
at the declaration, instead of disabling any lint for a crate. The attributes are
checked: removing the triggering condition makes the expectation itself fail.

In the next API revision, migrate the public client/core backup configuration and
placement/protection policy methods to named request structures together with all
callers, preserving scope/generation fencing, operation identity and wire fields.
The internal `publish_node`, `build_version`, `insert_boundary` and demo `child`/reference `node` helpers should
likewise receive named publication, sealing and provenance inputs when their callers
are migrated. Do not hide distinct identity or fencing values in anonymous tuples
solely to meet an argument-count limit. These exceptions do not cover correctness,
ownership, allocation, dead-code, or style warnings.
