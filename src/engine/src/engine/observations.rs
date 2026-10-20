//! Non-authoritative disk observations detached from the single writer.
//!
//! Capture configuration and logical accounting while holding the engine;
//! filesystem/seal probes run later without blocking publication. Never use an
//! observation as admission for a write: commit paths verify current bindings.
use super::Engine;
use crate::{disk::*, *};
use std::collections::BTreeMap;

pub struct DiskObservationPlan {
    config: Config,
    used_bytes: BTreeMap<String, u64>,
}

impl Engine {
    pub fn prepare_disk_observation(&self) -> DiskObservationPlan {
        let mut used_bytes: BTreeMap<String, u64> = BTreeMap::new();
        for placement in self.state.placements.values() {
            if self
                .state
                .content_states
                .get(&placement.version_id)
                .is_some_and(|state| state == "PURGED")
            {
                continue;
            }
            let Some(version) = self.state.versions.get(&placement.version_id) else {
                continue;
            };
            for (index, slot) in placement.route_slot_ids.iter().enumerate() {
                // Match used_bytes' membership semantics even for duplicate slots.
                if placement.route_slot_ids[..index].contains(slot) {
                    continue;
                }
                let used = used_bytes.entry(slot.clone()).or_default();
                *used = used.saturating_add(version.logical_size);
            }
        }
        DiskObservationPlan {
            config: self.config.clone(),
            used_bytes,
        }
    }

    pub fn disk_status(&self) -> Result<Vec<DiskStatus>> {
        self.prepare_disk_observation().status()
    }

    /// Filesystem capacity is sampled from the actual managed member mount.
    /// Development directories may share one physical filesystem.
    pub fn disk_capacity(&self, slot: &str) -> Result<(u64, u64, u64)> {
        capacity(self.disk(slot)?)
    }
}

fn capacity(disk: &DiskConfig) -> Result<(u64, u64, u64)> {
    let stat = rustix::fs::statvfs(&disk.root).map_err(std::io::Error::from)?;
    let total = stat.f_blocks.saturating_mul(stat.f_frsize);
    let available = stat.f_bavail.saturating_mul(stat.f_frsize);
    let used = stat
        .f_blocks
        .saturating_sub(stat.f_bfree)
        .saturating_mul(stat.f_frsize);
    Ok((total, available, used))
}

impl DiskObservationPlan {
    /// Observe physical availability using the captured binding configuration.
    pub fn status(&self) -> Result<Vec<DiskStatus>> {
        Ok(self
            .config
            .disks
            .iter()
            .map(|disk| {
                let seal = validate_seal(&self.config, disk).ok();
                DiskStatus {
                    slot_id: disk.slot_id.clone(),
                    disk_id: disk.disk_id.clone(),
                    level: disk.level,
                    upstream_slot_id: disk.upstream_slot_id.clone(),
                    online: seal.is_some(),
                    metadata_member: disk.metadata_member,
                    physical_device: seal.map(|seal| seal.physical_device).unwrap_or_default(),
                    available_bytes: available(&disk.root).unwrap_or(0),
                    used_bytes: self.used_bytes.get(&disk.slot_id).copied().unwrap_or(0),
                    simulation: self.config.mode == Mode::Development,
                }
            })
            .collect())
    }

    pub fn capacity(&self, slot: &str) -> Result<(u64, u64, u64)> {
        let disk = self
            .config
            .disks
            .iter()
            .find(|disk| disk.slot_id.eq_ignore_ascii_case(slot))
            .ok_or_else(|| Error::new("SLOT_NOT_FOUND", "storage slot is not registered"))?;
        capacity(disk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_owns_snapshot_and_probes_without_engine() {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]);
        let engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let plan = engine.prepare_disk_observation();
        let expected = engine.disk_status().unwrap();
        drop(engine);
        let observed = plan.status().unwrap();
        assert_eq!(observed.len(), expected.len());
        for (observed, expected) in observed.iter().zip(&expected) {
            assert_eq!(observed.slot_id, expected.slot_id);
            assert_eq!(observed.online, expected.online);
            assert_eq!(observed.used_bytes, expected.used_bytes);
            assert!(
                plan.capacity(&observed.slot_id.to_ascii_lowercase())
                    .unwrap()
                    .0
                    > 0
            );
        }
        assert_eq!(
            plan.capacity("missing").unwrap_err().code(),
            "SLOT_NOT_FOUND"
        );
    }

    #[test]
    fn accounting_snapshot_preserves_purge_and_duplicate_route_semantics() {
        let root = tempfile::tempdir().unwrap();
        let config = Config::development([
            root.path().join("d1"),
            root.path().join("d2"),
            root.path().join("d3"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let repo = engine
            .create_repository("Accounting", "P01", 3, "owner", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "owner")
            .unwrap();
        let source = root.path().join("source");
        std::fs::write(&source, b"snapshot bytes").unwrap();
        engine
            .stage_file(&workspace.workspace_id, "sample.txt", &source)
            .unwrap();
        engine
            .commit_workspace(CommitRequest {
                workspace_id: workspace.workspace_id,
                operation_id: new_id("op"),
                principal: "owner".into(),
                expected_head: workspace.base_head,
                fencing_token: workspace.fencing_token,
                message: String::new(),
                request_digest: None,
            })
            .unwrap();
        for placement in engine.state.placements.values_mut() {
            placement.route_slot_ids.push("P01".into());
        }
        let plan = engine.prepare_disk_observation();
        for disk in plan.status().unwrap() {
            assert_eq!(disk.used_bytes, engine.used_bytes(&disk.slot_id));
        }
        for version in engine.state.versions.keys() {
            engine
                .state
                .content_states
                .insert(version.clone(), "PURGED".into());
        }
        assert!(
            engine
                .prepare_disk_observation()
                .status()
                .unwrap()
                .iter()
                .all(|disk| disk.used_bytes == 0)
        );
        assert!(
            plan.status()
                .unwrap()
                .iter()
                .any(|disk| disk.used_bytes > 0)
        );
    }
}
