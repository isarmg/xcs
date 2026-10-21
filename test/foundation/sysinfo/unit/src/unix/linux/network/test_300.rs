#[cfg(test)]
mod test {
    use super::refresh_networks_list_from_sysfs;
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn refresh_networks_list_add_interface() {
        let sys_net_dir = tempfile::tempdir().expect("failed to create temporary directory");

        fs::create_dir(sys_net_dir.path().join("itf1")).expect("failed to create subdirectory");

        let mut interfaces = HashMap::new();

        refresh_networks_list_from_sysfs(&mut interfaces, false, sys_net_dir.path());
        assert_eq!(interfaces.keys().collect::<Vec<_>>(), ["itf1"]);

        fs::create_dir(sys_net_dir.path().join("itf2")).expect("failed to create subdirectory");

        refresh_networks_list_from_sysfs(&mut interfaces, false, sys_net_dir.path());
        let mut itf_names: Vec<String> = interfaces.keys().map(|n| n.to_owned()).collect();
        itf_names.sort();
        assert_eq!(itf_names, ["itf1", "itf2"]);
    }

    #[test]
    fn refresh_networks_list_remove_interface() {
        let sys_net_dir = tempfile::tempdir().expect("failed to create temporary directory");

        let itf1_dir = sys_net_dir.path().join("itf1");
        let itf2_dir = sys_net_dir.path().join("itf2");
        fs::create_dir(&itf1_dir).expect("failed to create subdirectory");
        fs::create_dir(itf2_dir).expect("failed to create subdirectory");

        let mut interfaces = HashMap::new();

        refresh_networks_list_from_sysfs(&mut interfaces, false, sys_net_dir.path());
        let mut itf_names: Vec<String> = interfaces.keys().map(|n| n.to_owned()).collect();
        itf_names.sort();
        assert_eq!(itf_names, ["itf1", "itf2"]);

        fs::remove_dir(&itf1_dir).expect("failed to remove subdirectory");

        refresh_networks_list_from_sysfs(&mut interfaces, true, sys_net_dir.path());
        assert_eq!(interfaces.keys().collect::<Vec<_>>(), ["itf2"]);
    }

    #[test]
    fn refresh_networks_list_with_multiple_interfaces() {
        let dir = tempfile::tempdir().expect("failed to create temporary directory");

        for (name, rx) in [
            ("if_a", "100"),
            ("if_b", "1234567890123"),
            ("if_c", "9876543210987"),
            ("if_d", "18446744073709551615"),
        ] {
            let stats = dir.path().join(name).join("statistics");

            fs::create_dir_all(&stats).expect("failed to create statistics dir");

            for f in &[
                "rx_bytes",
                "tx_bytes",
                "rx_packets",
                "tx_packets",
                "rx_errors",
                "tx_errors",
            ] {
                fs::write(stats.join(f), rx).expect("failed to write stats");
            }

            fs::write(dir.path().join(name).join("mtu"), "1500").expect("failed to write mtu");

            fs::write(dir.path().join(name).join("operstate"), "up\n")
                .expect("failed to write operstate");
        }

        let mut interfaces = HashMap::new();

        refresh_networks_list_from_sysfs(&mut interfaces, false, dir.path());
        refresh_networks_list_from_sysfs(&mut interfaces, false, dir.path());

        assert_eq!(interfaces.get("if_a").unwrap().inner.rx_bytes, 100);
        assert_eq!(
            interfaces.get("if_b").unwrap().inner.rx_bytes,
            1_234_567_890_123
        );
        assert_eq!(
            interfaces.get("if_c").unwrap().inner.rx_bytes,
            9_876_543_210_987
        );
        assert_eq!(interfaces.get("if_d").unwrap().inner.rx_bytes, u64::MAX);

        for name in ["if_a", "if_b", "if_c", "if_d"] {
            let interface = interfaces.get(name).unwrap();
            assert_eq!(interface.inner.mtu, 1500, "{name}: mtu");
        }
    }
}
