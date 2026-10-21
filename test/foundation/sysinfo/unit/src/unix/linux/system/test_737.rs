#[cfg(test)]
mod test {
    use super::InfoType;
    #[cfg(target_os = "android")]
    use super::get_system_info_android;
    #[cfg(not(target_os = "android"))]
    use super::get_system_info_linux;
    use super::read_table;
    use super::system_info_as_list;
    use std::collections::HashMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_table() {
        // Create a temporary file with test content
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1:100 kB").unwrap();
        writeln!(file, "KEY2:200 kB").unwrap();
        writeln!(file, "KEY3:300 kB").unwrap();
        writeln!(file, "KEY4:invalid").unwrap();

        let file_path = file.path().to_str().unwrap();

        // Test reading the table
        let mut result = HashMap::new();
        read_table(file_path, ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert_eq!(result.get("KEY1"), Some(&100));
        assert_eq!(result.get("KEY2"), Some(&200));
        assert_eq!(result.get("KEY3"), Some(&300));
        assert_eq!(result.get("KEY4"), None);

        // Test with different separator and units
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1 400 MB").unwrap();
        writeln!(file, "KEY2 500 GB").unwrap();
        writeln!(file, "KEY3 600").unwrap();

        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ' ', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert_eq!(result.get("KEY1"), Some(&400));
        assert_eq!(result.get("KEY2"), Some(&500));
        assert_eq!(result.get("KEY3"), Some(&600));

        // Test with empty file
        let file = NamedTempFile::new().unwrap();
        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert!(result.is_empty());

        // Test with non-existent file
        let mut result = HashMap::new();
        read_table("/nonexistent/file", ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert!(result.is_empty());
    }

    #[test]
    #[cfg(target_os = "android")]
    fn lsb_release_fallback_android() {
        assert!(get_system_info_android(InfoType::OsVersion).is_some());
        assert!(get_system_info_android(InfoType::Name).is_some());
        assert!(get_system_info_android(InfoType::DistributionID).is_none());
        assert!(get_system_info_android(InfoType::DistributionIDLike).is_none());
    }

    #[test]
    #[cfg(not(target_os = "android"))]
    fn lsb_release_fallback_not_android() {
        use std::path::Path;

        let dir = tempfile::tempdir().expect("failed to create temporary directory");
        let tmp1 = dir.path().join("tmp1");
        let tmp2 = dir.path().join("tmp2");

        // /etc/os-release
        std::fs::write(
            &tmp1,
            r#"NAME="Ubuntu"
VERSION="20.10 (Groovy Gorilla)"
ID=ubuntu
ID_LIKE=debian
PRETTY_NAME="Ubuntu 20.10"
VERSION_ID="20.10"
VERSION_CODENAME=groovy
UBUNTU_CODENAME=groovy
"#,
        )
        .expect("Failed to create tmp1");

        // /etc/lsb-release
        std::fs::write(
            &tmp2,
            r#"DISTRIB_ID=Ubuntu
DISTRIB_RELEASE=20.10
DISTRIB_CODENAME=groovy
DISTRIB_DESCRIPTION="Ubuntu 20.10"
"#,
        )
        .expect("Failed to create tmp2");

        // Check for the "normal" path: "/etc/os-release"
        assert_eq!(
            get_system_info_linux(InfoType::OsVersion, &tmp1, Path::new("")),
            Some("20.10".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::Name, &tmp1, Path::new("")),
            Some("Ubuntu".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::DistributionID, &tmp1, Path::new("")),
            Some("ubuntu".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::DistributionIDLike, &tmp1, Path::new("")),
            Some("debian".to_owned())
        );

        // Check for the "fallback" path: "/etc/lsb-release"
        assert_eq!(
            get_system_info_linux(InfoType::OsVersion, Path::new(""), &tmp2),
            Some("20.10".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::Name, Path::new(""), &tmp2),
            Some("Ubuntu".to_owned())
        );
        assert_eq!(
            get_system_info_linux(InfoType::DistributionID, Path::new(""), &tmp2),
            None
        );
        assert_eq!(
            get_system_info_linux(InfoType::DistributionIDLike, Path::new(""), &tmp2),
            None
        );
    }

    #[test]
    fn test_system_info_as_list() {
        // No value.
        assert_eq!(system_info_as_list(None), Vec::<String>::new());
        // Empty value.
        assert_eq!(
            system_info_as_list(Some("".to_string())),
            Vec::<String>::new(),
        );
        // Whitespaces only.
        assert_eq!(
            system_info_as_list(Some(" ".to_string())),
            Vec::<String>::new(),
        );
        // Single value.
        assert_eq!(
            system_info_as_list(Some("debian".to_string())),
            vec!["debian".to_string()],
        );
        // Multiple values.
        assert_eq!(
            system_info_as_list(Some("rhel fedora".to_string())),
            vec!["rhel".to_string(), "fedora".to_string()],
        );
        // Multiple spaces.
        assert_eq!(
            system_info_as_list(Some("rhel        fedora".to_string())),
            vec!["rhel".to_string(), "fedora".to_string()],
        );
    }
}
