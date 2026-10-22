# Management Web acceptance

The Web shell never infers a fresh installation from storage failure or missing
credentials. `/api/management/status` is public and coarse; storage writes are
admitted only after an explicit `ready` status. Session restoration and login do
not load the data disk before showing the shell. Read-only, unavailable,
unconfigured, recovery-required and unknown states remain distinct.

Protected setup is an explicit form, not a fallback login action. The local
bootstrap key is held in component memory and sent only as `X-ZXS-Bootstrap` to
the same-origin setup endpoints. The status response supplies the CSRF token and
allowlisted disk IDs. Exactly three distinct IDs and an explicit confirmation
are required. Development mode appears only when deployment explicitly enables
it. Password/key inputs clear at submission. An ambiguous response permanently
blocks repeat initialization within that mounted form; the operator must verify
server state rather than blindly resubmit. No arbitrary host path is accepted.

## Automated checks

- `cargo test -p web-app --lib`: typed status, mutation admission, setup allowlist,
  explicit confirmation, uncertain outcomes, and existing session/metadata tests.
- `cargo clippy -p web-app --target wasm32-unknown-unknown -- -D warnings`.
- The existing `browser-smoke` CI target runs production WASM in Chrome. Its
  `runner/management.rs` starts a second real daemon with corrupt pool config and
  a verified management-authority file, checks existing-admin login, health,
  CPU/memory, retry and logout; it then provisions a disposable bootstrap key and
  disk allowlist, completes real development setup in the browser, waits for
  storage reopen, logs in the new administrator and checks URL/browser storage
  for leaked credentials.

Do not substitute static source checks for browser acceptance. If local Chrome
is blocked by the environment's sandbox, report that limitation and require the
CI Chrome job; do not retry with security bypass flags. Existing G4/font and
file-metadata/history/branch tests continue unchanged.

## Protected completion and local presets

Authorized setup status may expose `completion_available`. It is the only UI
admission signal for the separate, explicitly confirmed
`complete-initialization` action. Generic recovery, an existing pool or missing
credentials never infer permission to reset an administrator. After an unknown
initialize outcome, new initialization remains blocked; the user must reenter
the original key and recheck status before a server-proven completion can be
selected. Changing actions clears confirmation.

Local preset IDs are listed by authorized status. The user selects an opaque
ID and requests an authorized `preset-preview`; the sanitized configuration is
read-only, replaces disk roots with candidate IDs, and preserves all parameters.
Initialize submits only the selected ID, its reviewed `config_digest`, and the
administrator credentials. It never submits configuration JSON, host paths, the
wizard's disk selection, or mode overrides. The backend rejects a locally
changed preset until reviewed again. Preset disk count is not restricted to the
three-disk wizard.

Focused Web unit regressions cover proof-only completion (including uncertain
outcomes), no authority-reset inference, explicit reconfirmation, unreviewed or
unapproved preset rejection, and exact bounded request bodies. The real Chrome runner now additionally exercises a four-disk local preset:
review its sanitized summary, modify the local preset, assert the real HTTP 409
SETUP_PRESET_CHANGED rejection leaves storage and authentication absent, re-review,
initialize, compare the entire preserved Config, and log in. It also seeds a
genuine interrupted pre-authority initialization using the setup service with a
test-owned failing install callback, then completes it through the real Web form
and logs in while asserting the pool configuration was not rewritten. No product
debug endpoint or filesystem-permission trick is used. A native unit test checks
that the fixture has genuine completion proof without published authority.
These Chrome scenarios are implemented, not yet executed; native compilation
and unit tests must not be reported as real browser acceptance.
