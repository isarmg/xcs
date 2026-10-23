use super::*;
use std::os::unix::fs::PermissionsExt;

fn online_observation(capacity: Option<(u64, u64, u64)>) -> DiskObservation {
    DiskObservation {
        status: storage_engine::DiskStatus {
            slot_id: "P01".into(),
            disk_id: "disk-1".into(),
            level: 1,
            upstream_slot_id: None,
            online: true,
            metadata_member: true,
            physical_device: String::new(),
            available_bytes: capacity.map(|capacity| capacity.1).unwrap_or(0),
            used_bytes: 99,
            simulation: true,
        },
        capacity,
    }
}

#[test]
fn online_member_with_unknown_capacity_keeps_unknown_monitoring_fields() {
    let value = disk_value(online_observation(None));
    assert_eq!(value["online"], true);
    for field in [
        "total_bytes",
        "available_bytes",
        "used_bytes",
        "usage_percent",
    ] {
        assert!(value[field].is_null(), "unknown {field} must remain null");
    }
}

#[test]
fn monitoring_uses_sampled_physical_bytes_not_logical_accounting() {
    let value = disk_value(online_observation(Some((1000, 700, 300))));
    assert_eq!(value["total_bytes"], "1000");
    assert_eq!(value["available_bytes"], "700");
    assert_eq!(value["used_bytes"], "300");
    assert_eq!(value["usage_percent"], 30.0);
}

#[test]
fn unmounted_member_capacity_is_unknown_in_management_payload() {
    let temporary = tempfile::tempdir().unwrap();
    let config = storage_contract::Config::development([
        temporary.path().join("d1"),
        temporary.path().join("d2"),
        temporary.path().join("d3"),
    ]);
    let engine =
        storage_engine::Engine::initialize(&temporary.path().join("system"), config.clone())
            .unwrap();
    let plan = engine.prepare_disk_observation();
    let member = &config.disks[0];
    std::fs::rename(&member.root, temporary.path().join("removed-member")).unwrap();
    std::fs::create_dir(&member.root).unwrap();
    std::fs::set_permissions(&member.root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let result = observe(plan).unwrap();
    let disks = result["disks"].as_array().unwrap();
    let missing = disks
        .iter()
        .find(|disk| disk["id"] == member.slot_id)
        .unwrap();
    assert_eq!(missing["online"], false);
    for field in [
        "total_bytes",
        "available_bytes",
        "used_bytes",
        "usage_percent",
    ] {
        assert!(
            missing[field].is_null(),
            "offline {field} must not describe the host filesystem"
        );
    }
    let present = disks
        .iter()
        .find(|disk| disk["id"] == config.disks[1].slot_id)
        .unwrap();
    assert_eq!(present["online"], true);
    assert!(
        present["total_bytes"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
}
