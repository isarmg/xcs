#[cfg(test)]
mod tests {
    #[test]
    fn test_serde_process_name() {
        if !crate::IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = crate::System::new();
        s.refresh_processes_specifics(
            crate::ProcessesToUpdate::All,
            false,
            crate::ProcessRefreshKind::nothing(),
        );

        if s.processes().is_empty() {
            panic!("no processes?");
        }

        for p in s.processes().values() {
            let values = match serde_json::to_value(p) {
                Ok(serde_json::Value::Object(values)) => values,
                other => panic!("expected object, found `{other:?}`"),
            };
            match values.get("name") {
                Some(serde_json::Value::String(_)) => {}
                value => panic!("expected a string, found `{value:?}`"),
            }
        }
    }

    #[test]
    #[cfg(feature = "network")]
    fn test_serde_mac_address() {
        let m = crate::MacAddr([0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]);

        let value = match serde_json::to_value(m) {
            Ok(serde_json::Value::String(value)) => value,
            other => panic!("expected string, found `{other:?}`"),
        };
        assert_eq!(value, "12:34:56:78:9a:bc");
    }

    #[test]
    #[cfg(feature = "disk")]
    fn test_serde_disk_file_system() {
        let mut disk = crate::Disk {
            inner: crate::DiskInner::default(),
        };
        disk.inner.file_system = "ZFS".into();

        let obj = match serde_json::to_value(disk) {
            Ok(serde_json::Value::Object(obj)) => obj,
            other => panic!("expected object, found `{other:?}`"),
        };
        assert_eq!(
            obj.get("file_system"),
            Some(&serde_json::Value::String("ZFS".to_string()))
        );
    }
}
