#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

    #[test]
    fn test_component_refresh_simple() {
        let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
        let hwmon0_dir = temp_dir.path().join("hwmon/hwmon0");

        fs::create_dir_all(temp_dir.path().join("hwmon/hwmon0"))
            .expect("failed to create hwmon/hwmon0 directory");

        fs::write(hwmon0_dir.join("name"), "test_name").expect("failed to write to name file");
        fs::write(hwmon0_dir.join("temp1_input"), "1234")
            .expect("failed to write to temp1_input file");

        let mut components = ComponentsInner::new();
        components.refresh_from_sys_class_path(temp_dir.path());
        let components = components.into_vec();

        assert_eq!(components.len(), 1);
        assert_eq!(components[0].inner.name, "test_name");
        assert_eq!(components[0].label(), "test_name temp1");
        assert_eq!(components[0].temperature(), Some(1.234));
        assert_eq!(components[0].id(), Some("hwmon0_1"));
    }

    #[test]
    fn test_component_refresh_with_more_data() {
        let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
        let hwmon0_dir = temp_dir.path().join("hwmon/hwmon0");

        // create hwmon0 file including device/model file
        fs::create_dir_all(hwmon0_dir.join("device"))
            .expect("failed to create hwmon/hwmon0 directory");

        fs::write(hwmon0_dir.join("name"), "test_name").expect("failed to write to name file");
        fs::write(hwmon0_dir.join("device/model"), "test_model")
            .expect("failed to write to model file");

        fs::write(hwmon0_dir.join("temp1_label"), "test_label1")
            .expect("failed to write to temp1_label file");
        fs::write(hwmon0_dir.join("temp1_input"), "1234")
            .expect("failed to write to temp1_input file");
        fs::write(hwmon0_dir.join("temp1_crit"), "100").expect("failed to write to temp1_min file");

        let mut components = ComponentsInner::new();
        components.refresh_from_sys_class_path(temp_dir.path());
        let components = components.into_vec();

        assert_eq!(components.len(), 1);
        assert_eq!(components[0].inner.name, "test_name");
        assert_eq!(components[0].label(), "test_name test_label1 test_model");
        assert_eq!(components[0].temperature(), Some(1.234));
        assert_eq!(components[0].max(), Some(1.234));
        assert_eq!(components[0].critical(), Some(0.1));
        assert_eq!(components[0].id(), Some("hwmon0_1"));
    }

    #[test]
    fn test_component_refresh_multiple_sensors() {
        let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
        let hwmon0_dir = temp_dir.path().join("hwmon/hwmon0");
        fs::create_dir_all(&hwmon0_dir).expect("failed to create hwmon/hwmon0 directory");

        fs::write(hwmon0_dir.join("name"), "test_name").expect("failed to write to name file");

        fs::write(hwmon0_dir.join("temp1_label"), "test_label1")
            .expect("failed to write to temp1_label file");
        fs::write(hwmon0_dir.join("temp1_input"), "1234")
            .expect("failed to write to temp1_input file");
        fs::write(hwmon0_dir.join("temp1_crit"), "100").expect("failed to write to temp1_min file");

        fs::write(hwmon0_dir.join("temp2_label"), "test_label2")
            .expect("failed to write to temp2_label file");
        fs::write(hwmon0_dir.join("temp2_input"), "5678")
            .expect("failed to write to temp2_input file");
        fs::write(hwmon0_dir.join("temp2_crit"), "200").expect("failed to write to temp2_min file");

        let mut components = ComponentsInner::new();
        components.refresh_from_sys_class_path(temp_dir.path());
        let mut components = components.into_vec();
        components.sort_by_key(|c| c.inner.label.clone());

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].inner.name, "test_name");
        assert_eq!(components[0].label(), "test_name test_label1");
        assert_eq!(components[0].temperature(), Some(1.234));
        assert_eq!(components[0].max(), Some(1.234));
        assert_eq!(components[0].id(), Some("hwmon0_1"));
        assert_eq!(components[0].critical(), Some(0.1));

        assert_eq!(components[1].inner.name, "test_name");
        assert_eq!(components[1].label(), "test_name test_label2");
        assert_eq!(components[1].temperature(), Some(5.678));
        assert_eq!(components[1].max(), Some(5.678));
        assert_eq!(components[1].id(), Some("hwmon0_2"));
        assert_eq!(components[1].critical(), Some(0.2));
    }

    #[test]
    fn test_component_refresh_multiple_sensors_with_device_model() {
        let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
        let hwmon0_dir = temp_dir.path().join("hwmon/hwmon0");

        // create hwmon0 file including device/model file
        fs::create_dir_all(hwmon0_dir.join("device"))
            .expect("failed to create hwmon/hwmon0 directory");

        fs::write(hwmon0_dir.join("name"), "test_name").expect("failed to write to name file");
        fs::write(hwmon0_dir.join("device/model"), "test_model")
            .expect("failed to write to model file");

        fs::write(hwmon0_dir.join("temp1_label"), "test_label1")
            .expect("failed to write to temp1_label file");
        fs::write(hwmon0_dir.join("temp1_input"), "1234")
            .expect("failed to write to temp1_input file");
        fs::write(hwmon0_dir.join("temp1_crit"), "100").expect("failed to write to temp1_min file");

        fs::write(hwmon0_dir.join("temp2_label"), "test_label2")
            .expect("failed to write to temp2_label file");
        fs::write(hwmon0_dir.join("temp2_input"), "5678")
            .expect("failed to write to temp2_input file");
        fs::write(hwmon0_dir.join("temp2_crit"), "200").expect("failed to write to temp2_min file");

        let mut components = ComponentsInner::new();
        components.refresh_from_sys_class_path(temp_dir.path());
        let mut components = components.into_vec();
        components.sort_by_key(|c| c.inner.label.clone());

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].inner.name, "test_name");
        assert_eq!(components[0].label(), "test_name test_label1 test_model");
        assert_eq!(components[0].temperature(), Some(1.234));
        assert_eq!(components[0].max(), Some(1.234));
        assert_eq!(components[0].critical(), Some(0.1));
        assert_eq!(components[0].id(), Some("hwmon0_1"));

        assert_eq!(components[1].inner.name, "test_name");
        assert_eq!(components[1].label(), "test_name test_label2 test_model");
        assert_eq!(components[1].temperature(), Some(5.678));
        assert_eq!(components[1].max(), Some(5.678));
        assert_eq!(components[1].critical(), Some(0.2));
        assert_eq!(components[1].id(), Some("hwmon0_2"));
    }

    #[test]
    fn test_thermal_zone() {
        let temp_dir = tempfile::tempdir().expect("failed to create temporary directory");
        let thermal_zone0_dir = temp_dir.path().join("thermal/thermal_zone0");
        let thermal_zone1_dir = temp_dir.path().join("thermal/thermal_zone1");

        // create thermal zone files
        fs::create_dir_all(thermal_zone0_dir.join("device"))
            .expect("failed to create thermal/thermal_zone0 directory");

        fs::write(thermal_zone0_dir.join("type"), "test_name")
            .expect("failed to write to name file");
        fs::write(thermal_zone0_dir.join("temp"), "1234").expect("failed to write to temp file");

        // create thermal zone files
        fs::create_dir_all(thermal_zone1_dir.join("device"))
            .expect("failed to create thermal/thermal_zone1 directory");

        fs::write(thermal_zone1_dir.join("type"), "test_name2")
            .expect("failed to write to name file");
        fs::write(thermal_zone1_dir.join("temp"), "5678").expect("failed to write to temp file");

        let mut components = ComponentsInner::new();
        components.refresh_from_sys_class_path(temp_dir.path());
        let mut components = components.into_vec();
        components.sort_by_key(|c| c.inner.name.clone());

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].inner.name, "test_name");
        assert_eq!(components[0].label(), "");
        assert_eq!(components[0].temperature(), Some(1.234));
        assert_eq!(components[0].max(), Some(1.234));
        assert_eq!(components[0].id(), Some("thermal_zone0"));

        assert_eq!(components[1].inner.name, "test_name2");
        assert_eq!(components[1].label(), "");
        assert_eq!(components[1].temperature(), Some(5.678));
        assert_eq!(components[1].max(), Some(5.678));
        assert_eq!(components[1].id(), Some("thermal_zone1"));
    }
}
