//! Read-only sampling of an owned, detached disk observation plan.
use super::error;
use crate::ApiError;
use serde_json::{Value, json};
use storage_engine::DiskObservationPlan;

pub(super) fn observe(observation: DiskObservationPlan) -> Result<Value, ApiError> {
    let mut disks = Vec::new();
    for disk in observation.status().map_err(error)? {
        let capacity = observation.capacity(&disk.slot_id).ok();
        let (power_watts, power_unavailable_reason) =
            crate::telemetry::disk_power(&disk.physical_device, disk.simulation, disk.online);
        let usage_percent = capacity
            .filter(|capacity| capacity.0 > 0)
            .map(|capacity| capacity.2 as f64 / capacity.0 as f64 * 100.0);
        disks.push(json!({
            "id":disk.slot_id,"name":disk.slot_id,"online":disk.online,
            "read_only":disk.level!=1,"simulation":disk.simulation,
            "total_bytes":capacity.map(|c|c.0.to_string()),
            "available_bytes":capacity.map(|c|c.1.to_string()),
            "used_bytes":capacity.map(|c|c.2.to_string()),
            "usage_percent":usage_percent,"power_watts":power_watts,
            "power_unavailable_reason":power_unavailable_reason
        }));
    }
    Ok(json!({"disks":disks}))
}
