# Publication receipt regression fixtures

These complete API responses were captured on 2026-10-19 from synthetic temporary
three-disk Engine test pools by the existing `zxs-server` HTTP Engine tests:

- `damage_after_normal_publication_keeps_original_target_met_in_save_wire`
- `recovered_permanent_loss_returns_degraded_save_wire_with_frozen_unavailable_slots`

Set `ZXS_TEST_WIRE_EVIDENCE_DIR` to an output directory and run
`cargo test -p zxs-server --test http_engine save_wire` to regenerate them.
The IDs and timestamps describe synthetic test data; no credentials are included.
These files retain the complete, unchanged responses. The UI unit test derives a
healthy comparison in memory by changing only `current_health.state`; that
comparison is explicitly synthetic and is not a separate captured response.

Fixtures live with the source so a clean checkout can run UI tests without
ignored/generated `tests/evidence` artifacts.
