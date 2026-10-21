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
                .is_some_and(|state| state == "PURGED" || state == "MIGRATED")
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
#[path = "../../../../test/engine/unit/engine/observations/tests.rs"]
mod tests;
