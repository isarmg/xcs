//! Read-only sampling of an owned, detached disk observation plan.
use crate::ApiError;
use json_codec::{Value, json};
use storage_engine::{DiskObservation, DiskObservationPlan};

pub(super) fn observe(observation: DiskObservationPlan) -> Result<Value, ApiError> {
    let disks = observation
        .sample()
        .into_iter()
        .map(disk_value)
        .collect::<Vec<_>>();
    Ok(json!({"disks":disks}))
}

fn disk_value(observation: DiskObservation) -> Value {
    let DiskObservation {
        status: disk,
        capacity,
    } = observation;
    let (power_watts, power_unavailable_reason) =
        crate::telemetry::disk_power(&disk.physical_device, disk.simulation, disk.online);
    let usage_percent = capacity
        .filter(|capacity| capacity.0 > 0)
        .map(|capacity| capacity.2 as f64 / capacity.0 as f64 * 100.0);
    json!({
        "id":disk.slot_id,"name":disk.slot_id,"online":disk.online,
        "read_only":disk.level!=1,"simulation":disk.simulation,
        "total_bytes":capacity.map(|c|c.0.to_string()),
        "available_bytes":capacity.map(|c|c.1.to_string()),
        "used_bytes":capacity.map(|c|c.2.to_string()),
        "usage_percent":usage_percent,"power_watts":power_watts,
        "power_unavailable_reason":power_unavailable_reason
    })
}

#[cfg(test)]
#[path = "../../../../test/server/unit/engine_backend/disk_monitor_tests.rs"]
mod tests;
