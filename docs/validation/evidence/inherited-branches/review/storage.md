# Inherited branch storage update

Latest semantics: registration still writes metadata plus empty stubs; reads inherit the exact frozen parent version until first content write. Physical container projection unchanged.

Production change: maintenance::check_purge again pins every version used by active branch HEAD or open/sealed workspace base, including inherited pending sources. Removed the now-unused crate-private pending helper re-export introduced only for the old purge exemption.

Regression updates: both purge scenarios now reject CONTENT_PINNED before first own content, then permit source purge after successful materialization and closure of other references; the independently written FULL stays readable. Directory-main proxy selection has the same rule. History package and decision-replay tests expect the visible parent version/size, and old inherited snapshots keep the frozen parent after the live branch writes a different own version. Container stub creation, rename, legacy paths and payload inodes remain unchanged.

Validation: container_storage 6/6, history_package 4/4 and maintenance::split_tests 7/7 passed (17 total). Log: the historical temporary test directory/log No live runtime/data accessed or modified. Production source frozen after this pass.

Docs and schema now distinguish version_id (visible real parent or own), own_version_id (null until first write), materialized, inherited, and visible_content_state (real parent or own logical content state). Branch/Commit pending maps remain canonical-compatible ownership/inheritance records. Evidence lives in test/evidence/inherited-branches/contract; previous lazy-branches evidence left intact.
