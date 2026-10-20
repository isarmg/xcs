# Native dependency maintenance

These sources are repository-owned compatibility forks, not a claim that the
upstream GTK3 ecosystem is maintained. Original versions and licenses are kept.
`native-provenance.json` records immutable crates.io archive URLs/SHA-256, every
vendored file hash, license files and reviewable patch hashes. `patches/` records
the complete delta from each archived crate. Zero-length patches mean an exact
source import under repository maintenance, not an upstream fix.

## Decisions and scope

- GTK3/Tauri requires GLib 0.18; the official RUSTSEC-2024-0429 fix begins at
  GLib 0.20 and is not a drop-in compatible dependency. Backport the exact mutable
  out-argument correction from https://github.com/gtk-rs/gtk-rs-core/pull/1343.
  Version stays 0.18.5. The optimized regression exercises forward/reverse/nth/
  nth_back/last/mixed iteration including UTF-8 and empty strings.
- GLib/GTK3 procedural macros move from proc-macro-error 1 to our maintained
  proc-macro-error2 2 source. **Both upstream implementations are unmaintained**
  (RUSTSEC-2024-0370 and RUSTSEC-2026-0173). Version 2 is not represented as an
  upstream maintained replacement. We own the imported error handler and its
  attribute macro, fix the private `proc_macro` re-export future-incompatibility,
  and run positive expansion and negative compile-error diagnostic regressions.
- The latest compatible stable Tauri opener is 2.7.2. The available 3.0 prerelease
  is not a drop-in Tauri 2 upgrade. Its Unix implementation moves from zbus 5 to
  zbus 6 (https://rustsec.org/advisories/RUSTSEC-2026-0370.html and
  https://rustsec.org/advisories/RUSTSEC-2026-0371.html). The complete async operation
  runs on a dedicated thread, preserving callers already inside async runtimes.
  File URI construction and existing fallback behavior remain unchanged; mobile,
  Windows, macOS, permission scopes and URL/file-opening paths are not altered.

## Required validation and update policy

1. Verify source provenance: `python3 tools/native-dependency-tests/verify_sources.py`.
2. Run tamper/missing-patch regressions: `python3 tools/native-dependency-tests/test_verify_sources.py`.
3. On Linux with GLib SDK and D-Bus: `dbus-run-session -- cargo test --manifest-path tools/native-dependency-tests/Cargo.toml --release --locked`.
4. Run host checks on Linux, Windows and macOS. The native test harness includes
   the real opener implementation, but does not replace full Tauri platform checks.
5. Run every owned lockfile through the current advisory database. Local path
   dependencies may disappear from cargo-audit matching, so the explicit
   `native-advisories.json` inventory and fail-closed source verification are
   mandatory even when the scanner reports no findings. No blanket ignore or
   cosmetic version bump is permitted. A new advisory requires review, not reuse
   of an old classification.

To update: obtain the exact official crate archive; verify its checksum against
registry metadata; review all source/license changes and regenerate the complete
patch/file manifest; rerun all validation above. Do not merely update hashes after
an unexpected drift. Track future GTK/Tauri migration upstream rather than
expanding the GLib backport beyond its reviewed safety fix.

## Local validation limits

The current runner has GLib 2.84.4 shared libraries but no SDK pkg-config files.
Optimized GLib tests use temporary linker metadata pointing to those actual
installed libraries, without installing software or modifying system settings.
The private D-Bus regression is compiled, but execution requires a Unix socket;
this sandbox rejects socket creation and its escalation backend fails before
execution. The dedicated CI job installs the proper SDK and runs the live D-Bus
regression. CI configured in this change is not evidence that CI already ran.

## Future advisories on path dependencies

The security gate also builds a temporary, non-build Cargo.lock containing the
original registry identity and archive checksum of all six native forks, all
11 imported Leptos-family dependencies, the existing unicode-normalization fork, and the original `leptos`/`leptos_macro`
0.8.19 identities behind the renamed CSR forks. The latter mapping is recorded
in `vendor/web/upstream-audit-identities.json`, with official archive checksums
verified against registry metadata; it conservatively scans upstream lineage,
not a claim that the customized code equals the release archive.

All 20 identities are required, and every non-first-party local package in the
consumer lockfiles must be covered. A newly vendored package cannot silently
skip the inventory. The shadow lock is recreated from verified provenance and compared byte-for-byte
before classification. Missing/changed identities, incomplete scanner output,
registry lookup failures and every new/unreviewed advisory fail the gate. Only
RUSTSEC-2024-0429 on exact GLib 0.18.5 and RUSTSEC-2026-0173 on exact
proc-macro-error2 2.0.1 are classified according to the documented source
backport/repository maintenance decisions. Both are printed as retained upstream
advisories. No real consumer lock is rewritten, no advisory ID is passed to an
ignore option, and clean path-dependency scanner output is not sufficient.

Regression tests in `test_upstream_audit.py` inject future findings on native,
Leptos-family and renamed core/macro sources and tamper with shadow identities;
all must fail closed. A same-test negative control against unpatched upstream
GLib 0.18.5 was also run in release mode: it crashed with SIGSEGV, while the
repository-backported source passes.
