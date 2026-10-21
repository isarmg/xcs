use super::*;
#[test]
fn metadata_metrics_count_flat_and_sharded_files_by_kind() {
    let temp = tempfile::tempdir().unwrap();
    let objects = temp.path().join("member/metadata/objects");
    let hash = "ab".repeat(32);
    let flat = objects.join("snapshot").join(format!("{hash}.json"));
    let shard = objects.join("state-page/ab").join(format!("{hash}.json"));
    fs::create_dir_all(flat.parent().unwrap()).unwrap();
    fs::create_dir_all(shard.parent().unwrap()).unwrap();
    fs::write(flat, b"123").unwrap();
    fs::write(shard, b"12345").unwrap();
    let metrics = disk_bytes(temp.path()).unwrap();
    assert_eq!(metrics["files"], 2);
    assert_eq!(metrics["metadata_object_bytes_all_members"], 8);
    assert_eq!(
        metrics["metadata_objects_by_kind_all_members"],
        json!({"snapshot":{"bytes":3,"files":1},"state-page":{"bytes":5,"files":1}})
    );
    for bad in [
        format!("state-page/ff/{hash}.json"),
        format!("state-page/ab/de/{hash}.json"),
        "state-page/not-a-hash.json".into(),
    ] {
        assert!(metadata_object_kind(&objects.join(bad)).is_err());
    }
}
