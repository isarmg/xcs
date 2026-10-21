# External test sources

Executable test code and test-only fixtures live under the root `test/` tree. Cargo
package identities, manifests, and production entry points remain under `src/`
(or the existing tool roots). Explicit Cargo test targets and gated Rust includes
retain package-local feature and private-module semantics. There is no test Cargo
workspace or independent lockfile.

`compliance/external-tests/*.json` accounts for current third-party test material
by owning source directory. Each owner records exact destination roots, current
file hashes and original source paths, including fragments extracted from a
production file that still exists. The source catalog binds these mappings to the
unchanged original upstream identity and hashes the mapping documents. Verification
rejects missing, changed, added, unattributed, overlapping, escaping and symlinked
material. Refresh verifies these explicit inventories before recording the source
catalog; it does not learn new test files implicitly. Removing a file from an
unchanged upstream source identity requires an explicit external source mapping.

Ring additionally uses `compliance/ring-test-relocation.json`, whose digest is
pinned by its verifier. Exact live production and test inventories are authenticated
before uniquely reversible source-routing changes reconstruct the prior source
view. Existing target-pruning, retired archive, style, original-overlap and test
material proofs retain their original bytes and are checked against that view.
The historical view cannot supply a missing current file. Production DER inputs
and unconditionally compiled public test-support APIs remain production sources;
executable assertions and their input vectors are external. The unconditional
compile-time assertions in `polyfill/cstr` preserve their original compilation
behavior through an external include.

The initial separation also has a compact, compressed reverse-line-delta receipt:
`compliance/test-source-relocation.json.gz`. Its compressed digest is pinned by
`source_relocation.py`; expansion is bounded. Bootstrap refresh checks every
remaining source and extracted fragment, rebuilds each pre-separation source from
those current bytes and reverse edits, and requires the result to match the prior
catalog's source hash. An empty/truncated replacement, different source attribution,
or altered current fragment cannot silently pass this first relocation audit.
The receipt is immutable historical evidence, not the mutable current inventory.
Later deliberate source/test edits follow the existing review-and-refresh process:
update the explicit current mapping and review the diff before refreshing. Such a
refresh accepts reviewed current code changes, not a claim that edited tests remain
byte-identical to the initial separation. Missing files and unrecorded additions
still fail, and removing previously attributed fragments is not ordinary refresh.
