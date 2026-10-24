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

## 项目新写的回归测试

全新编写、并非从上游代码抽取的测试仍在所属包的 `test/` 根下显式登记，
不得虚构上游来源路径。映射保持 `schema_version: 1`，仅这类新行增加
`source_kind: "project-authored"`；已有历史行不加标记、不改写。
首次登记时，`source` 等于实际 `path`，`source_sha256_before` 等于该文件
首次登记的实际字节 SHA-256，`sha256` 记录当前内容。refresh 同时检查首次
路径和实际字节，不会把任意旧哈希当作本地创作证据。

后续经审查的内容修改只更新 `sha256`；来源类型、初始路径、初始哈希及
原有 origin 标记必须保持不变。所属包的 upstream/version 身份不变，
但这不表示新测试来自该上游版本。新标记不能抵消上游文件移除，也不能
把旧的抽取行改成本地新作。精确清单、路径安全、禁止丢失既有行以及显式
退役归档规则继续适用。此支持仅覆盖本 owner 下的新作登记和后续内容
编辑，不授权跨 owner 转移或根重命名；这些变更未有专门证明时保持拒绝，
不能通过删除标记或重写初始来源绕过。

## Retirement and upgrade of an external-test owner

Ordinary refresh still refuses disappearance of an external owner or fragment.
An explicit `--retire-plan` or `--upgrade-plan` item for such an owner must include
`external_action: "remove"` and `external_tests` equal to the complete last owner
receipt from the catalog, including origin identity, roots, files, hashes, count,
and bytes. Remove that owner's entry from the current mapping and remove its
entire recorded test roots before refreshing. Missing approval, mismatched last
receipt, remaining mapping, leftover root (even empty), or symlink fails closed.

Retirement archives this exact external receipt under
`retired[directory].retirement.external_tests`. Its counts and bytes are added to
the retired summary totals. Upgrade archives it under
`superseded[prior_origin].external_tests`; unchanged old-version tests cannot be
silently attributed to a successor release. These nested receipts are protected
by the existing immutable retired/superseded record hashes. The initial relocation
proof remains checked even when no current external owners remain.

Upgrade supports removal, not in-place substitution of the old external corpus.
Restore successor tests inside the release-authenticated `source_prefixes` first,
then relocate successor tests in a separate reviewed change. If an upgrade needs
to retain or replace external tests directly, it must supply a separately designed
successor-preservation proof; the current workflow gives an explicit removal
instruction rather than exempting that owner from checks. Upgrade and retirement
of the same package require separate refreshes. Later, properly attributed tests
may reuse a superseded owner's former test root; a retired owner's root must stay
absent. Other active owners remain subject to the ordinary anti-drop checks.

## Correcting a whole-file classification

A file mistakenly classified as test-only may return to its exact former
production path during ordinary refresh, but only as a complete byte-preserving
restoration. The previous owner must have exactly one attribution for that source;
its original-source and external-file hashes must agree. The previous production
inventory must lack that file, the new production inventory and actual bytes must
match the recorded hash, the old external path must be absent, and both package
identities must retain the same owner and origin. Extracted fragments, altered
content, alternate paths, other owners, and retained external copies do not
qualify. This exception conserves source material rather than approving deletion.

The catalog retains `initial_test_relocation_sha256` after the last current
external owner is retired or restored, so the immutable initial separation proof
continues to be required independently of the current source classification.

Subsequent platform-test include-path fixes are authenticated separately by
`compliance/platform-test-routing-corrections.json`. The verifier pins this new
receipt, checks the corrected live bytes, and reverses its exact routing edit and
whole-file fixture move before applying the unchanged platform relocation proof.
Modified current bytes, incorrect reversals, reappearing old fixture paths, and
receipt tampering fail closed; the old platform and runtime receipts stay frozen.

## Explicit source naming transitions

`refresh --rename-plan PLAN.json` supports reviewed foundation package-root,
package-name and corresponding external-test-root renames. Schema 1 plans contain
`packages` entries with `directory`, the exact prior `package`,
`destination_directory`, `destination_name`, `reason`, the exact reviewed post-move
`files_sha256`, the exact previous `external_tests` owner (or null), and the exact
reviewed `destination_external_tests` owner (or null). The enriched owner includes
origin, file count and byte total. Destination directory basename and package name
must match. Relative production filenames and external fragment counts remain
bijective; changed routing/import bytes require explicit reviewed after hashes.

Schema 5 catalogs preserve the original package and external owner in append-only
`renames`, authenticated by `rename_records_sha256`. External `source` and
`source_sha256_before` labels remain historical; only validated same-origin rename
history allows that old source prefix under the current owner. Old production and
test roots must be absent. Ordinary refresh still rejects lost external fragments,
unknown files, changed ownership and rewritten history. Later local content edits
follow ordinary explicit refresh without rewriting the naming receipt. Prior
schema 4 catalogs without naming history remain verifiable.

Rename and retirement/upgrade of one identity require separate changes. A renamed
owner can later be explicitly retired with its exact external receipt. Repeat
renames, upstream upgrades after renaming, renames of already-upgraded sources,
and sources covered by fixed ring/platform live-view proofs fail closed until a
chain-aware or historical-view adapter is supplied. Original upstream names,
archives, licenses and security identities are never renamed or overwritten.

## Identity-preserving internal file moves

`refresh --move-plan PLAN.json` moves internal production filenames or external
test filenames without changing the package root, name, version or upstream
identity. This also applies after a previous package-root rename. Plans use
`schema: 1` and `packages` entries containing `directory`, exact previous
`package`, all exact overlapping `merged_packages` (or an empty list), `reason`,
full reviewed post-move `files_sha256`, `file_moves`, exact previous
`external_tests` (or null), reviewed `destination_external_tests` (or null), and
`external_file_moves`.

Production file maps use package-relative paths; external maps use repository-
relative paths. Enumerate every changed file, including each descendant of a
moved directory. Unlisted files remain at their current paths. The implicit and
explicit maps together must be bijections: no lost file, invented source,
unknown destination, or overwrite of an unmoved file is accepted. After content
hashes and byte totals are checked against disk. External owner roots, original
`source` labels, original hashes, fragment counts and ownership remain unchanged.
At least one map must be nonempty; an external-only move is supported.

Schema 6 catalogs append content-addressed `source_moves` receipts containing the
exact before snapshots and reviewed maps. Subsequent moves append new receipts,
never rewrite old source moves or root-renaming records. Ordinary content edits
remain possible through explicit refresh. Historical receipt origins are checked
against the independently authenticated origin inventory; a move does not create
or remove an origin. Schemas 4 and 5 remain readable, and refresh emits schema 6.

A source cannot be internally moved and root-renamed, upgraded or retired in the
same refresh. First finish the root-identity transition, then snapshot its current
receipt for a separate move plan. Disjoint owners may use different explicit plans
in one refresh. This accounting proves declared file preservation, not behavioral
compatibility; affected consumers and protocol/format invariants still require
their normal tests.

## Reviewed multi-origin consolidation

`refresh --consolidate-plan PLAN.json` explicitly combines implementations. Schema
8 catalogs retain content-addressed `consolidations` and matching
`consolidation_records_sha256`; older empty-history catalogs remain readable.
Catalog schemas never decrease during refresh.

A schema-1 plan has `packages` entries naming a `primary_directory`,
`destination_directory`, `destination_name`, and review `reason`. Each `inputs`
entry contains the exact previous `package` and enriched `external_tests` receipt
(or null). `destination_files_sha256` is the complete reviewed production tree.
`file_routes` names every previous package-relative file per input directory:
`destination` is a current relative file; only redundant root Cargo manifests
may instead use byte-exact `archive_base64`. Explicit `new_files` accounts for
new glue independently of routed files. Production routes may be many-to-one.
They document reviewed content migration, NOT byte preservation or semantic
compatibility. Functional tests remain mandatory.

`destination_external_tests` is the complete enriched combined owner. Every row
retains its historical source/hash and metadata, adds the original input's
`origin_directory`, and updates only the current path/hash.
`external_file_routes` maps every previous repository-relative test path to one
unique `destination` plus `replacements`: ordered base64 `old`/`new` byte pairs.
Initial verification reverses unique nonempty bounded edits and reproduces the
exact last-current test hash. Whole-file substitutions, disappearance, extra
files, symlinks, ambiguous edits and wrong-origin attribution fail closed.
Unchanged files require no replacements. Grants name exact origin/source/original
hash triples, never a broad merged-source prefix.

Primary and absorbed origins retain separate upstream versions, security and
license identities; absorbed origins are merged, not reported as deleted. Old
source and external roots must be absent. Historical receipts and original
materials are immutable. Later reviewed current-file edits use ordinary refresh
without rewriting initial transfer evidence or dropping origin tags.

The initial bounded implementation rejects pinned proof families, previously
upgraded/merged inputs and chained consolidated-owner lifecycle
changes until an authenticated adapter supports them. Disjoint ordinary lifecycle
plans may run together, but overlapping plans never silently bypass checks.

### Explicit removed implementation archives

A consolidation may remove an unsupported, unreferenced Rust implementation
without pretending its bytes were integrated into a current file. Its production
route must instead contain exactly `kind: "archived-source"`, `sha256`,
`archive_base64`, and `reason`. The hash and decoded bytes must match the exact
last-current input inventory. The review reason must be substantive (40–10,000
characters) and explain the removal; it is immutable with the receipt.

This narrow disposition accepts only UTF-8 `.rs` implementation files beneath
`src/`, not crate entrypoints (`src/lib.rs`, `src/main.rs`), tests, fixtures,
examples, benches, manifests, legal material or provenance. Protected path
segments and file stems are rejected case-insensitively (including tests.rs,
fixtures.rs, license.rs and provenance.rs). Archives are bounded
to 1 MiB decoded per file, 64 files and 4 MiB decoded total per consolidation;
encoded sizes are checked before decoding. Missing bytes, generic discard routes
and source-to-test archive substitutions are rejected. Existing redundant-root-
manifest archives keep their separate format and identity checks.

A separate `archived-build-script` disposition can preserve only a secondary
input's exact root `build.rs`. Its fields are `kind`, `sha256`, `archive_base64`,
`reason`, and `destination_manifest_base64`. The original root manifest must be
archived with its exact input hash; its package name/version and explicit or
implicit `build.rs` declaration must match. Primary scripts, custom build paths,
native `links` owners and legal-material ownership are rejected. The extra
manifest archive must match the destination Cargo.toml hash and name and declare
`build = false`; the destination must contain neither root `build.rs` nor an
exact copy of the retired script. Current manifest validation also requires this
destination to remain build-disabled and without `links` or root `build.rs`.
The same source UTF-8, reason, per-file and aggregate bounds apply. This does not
relax `archived-source`, authorize arbitrary entrypoint deletion or grant test
migration permissions. Both manifests and the script remain immutable evidence.

A primary script cannot use the secondary disposition. Its separate
`primary-build-script-retirement` route must additionally contain
`original_manifest_base64`, `replacement_sources`, and `toolchain`. The original
manifest is authenticated against the primary input hash and identity while its
normal manifest destination route remains intact. The destination must also
retain the primary version. `replacement_sources` records exact UTF-8 bytes and
SHA-256 hashes for 1–16 inventoried production Rust files, including `src/lib.rs`,
with a 256 KiB per-file and 1 MiB aggregate limit. Tests, fixtures, legal files,
custom build entrypoints and non-source paths are not replacement evidence.
`toolchain` declares `rust_version: "1.99"`, the ordered targets
`["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"]`, and substantive
`assumptions` (40–10,000 characters); the destination manifest must declare that
Rust version. This project-specific bounded disposition does not authorize other
toolchains or targets. It retains the original script and replacement evidence
without claiming their behavioral or bytecode equivalence; feature, generated
API and target-behavior tests remain mandatory engineering review evidence.
Pinned-source consolidation cannot use this route, even with a pinned-source
authorizer. Existing pinned projection still forbids every primary archive.

Archived implementation bytes live only in the explicit consolidation receipt,
not disguised as maintained files in `src/`. They are historical material, not
active production inventory or evidence of functional equivalence. Original
origin/version/license identities remain represented by the primary or merged
owner; independently frozen upstream evidence is never changed. External test
transfers still require their complete exact bijection and reversible proof.

### Consolidation after an ordinary rename or internal move

The optional `input_history` field binds each affected current input directory to
exact `renames` and `source_moves` receipt maps, copied unchanged from the last
catalog. All intersecting receipts are required; arbitrary subsets, unrelated
origins, stale current snapshots and ambiguous rename chains are rejected. The
binding is part of the content-addressed consolidation receipt and remains equal
to the original history on every later verification. At most 64 linked receipts
and one rename per input are supported, within the existing 32 MiB receipt bound.

Earlier snapshots are historical inventories, not claims that every old file is
still maintained. Legitimately reviewed changes or trims between the original
move and consolidation do not require deleted files to reappear. The complete
last-current package and external snapshots must match the pre-consolidation
catalog exactly, and every current file still requires an explicit disposition.
Original versions, legal identities and origin records remain unchanged.

A renamed input consumed by consolidation resolves through its authenticated
last-current snapshot to its separately checked current primary or merged origin.
The verifier does not open the vanished renamed root as though it were live. Both
old rename roots and removed consolidation roots must remain absent. Historical
external labels receive only that exact input's validated rename prefix before
the existing per-origin, per-file transfer proof is checked.

Upgraded inputs, previously merged inputs, repeated consolidations and pinned
proof families still require separately reviewed adapters; this history binding
does not authorize any of them or weaken physical inventory checks.

### Active historical proof routing and a pinned-primary consolidation

Active proof-owned roots cannot use missing-file retirement archives. Bounded,
content-addressed `proof-routing` transactions authenticate the complete live
inventory and reverse unique byte replacements to replay unchanged historical
platform/runtime/ring proofs. Current-path and package routing also drives the
actual ring matrix, rather than merely making historical verification pass.
Receipts are explicitly pinned in the helper and registered in the catalog;
refresh accepts only one new step per origin against its exact previous snapshot.
Existing schemas and consolidation history are never downgraded or replaced.

A terminal `consolidation` proof binding names the exact registered consolidation
record. The original pinned primary's production and test inventory remains a
bijection, with every original file reconstructed from its present current file.
Imported origins and new glue are authenticated against the complete combined
physical inventory but never enter that primary's historical file projection.
The original primary cannot archive files or share a routed file with an imported
origin. A second pinned input requires a separate future adapter and is rejected.

Large transformations must use meaningful reviewed same-root stages within the
existing 512 replacement/2 MiB decoded patch bounds, followed by the terminal
root/consolidation transaction. The final consolidation takes exact snapshots from
the immediately preceding stage, not the original unstaged catalog. Its external
transfers retain their independent bound and exact per-origin attribution.

A consolidated pinned chain is deliberately terminal: later edits, renames,
retirement or other lifecycle changes need a new authenticated adapter. Ordinary
unrelated owners retain their existing reviewed-refresh behavior. No historical
receipt, proof checksum, original origin/license material or verification step is
rewritten or skipped by this routing mechanism.
