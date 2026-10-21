#[cfg(test)]
mod test {
    use super::CGroupLimitsContext;
    use super::parse_cgroup_path;
    use super::read_table;
    use super::read_table_key;
    use super::v1_limits;
    use super::v2_limits;
    use std::collections::HashMap;
    use std::fs::{create_dir_all, write};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::{NamedTempFile, tempdir};

    #[test]
    fn test_read_table() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1:100 kB").unwrap();
        writeln!(file, "KEY2:200 kB").unwrap();
        writeln!(file, "KEY3:300 kB").unwrap();
        writeln!(file, "KEY4:invalid").unwrap();

        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert_eq!(result.get("KEY1"), Some(&100));
        assert_eq!(result.get("KEY2"), Some(&200));
        assert_eq!(result.get("KEY3"), Some(&300));
        assert_eq!(result.get("KEY4"), None);

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

        let file = NamedTempFile::new().unwrap();
        let file_path = file.path().to_str().unwrap();

        let mut result = HashMap::new();
        read_table(file_path, ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert!(result.is_empty());

        let mut result = HashMap::new();
        read_table("/nonexistent/file", ':', |key, value| {
            result.insert(key.to_string(), value);
        });

        assert!(result.is_empty());
    }

    #[test]
    fn test_read_table_key() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1:100 kB").unwrap();
        writeln!(file, "KEY2:200 kB").unwrap();
        writeln!(file, "KEY3:300 kB").unwrap();

        let file_path = file.path();

        assert_eq!(read_table_key(file_path, "KEY1", ':'), Some(100));
        assert_eq!(read_table_key(file_path, "KEY2", ':'), Some(200));
        assert_eq!(read_table_key(file_path, "KEY3", ':'), Some(300));
        assert_eq!(read_table_key(file_path, "KEY4", ':'), None);

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "KEY1 400 kB").unwrap();
        writeln!(file, "KEY2 500 kB").unwrap();

        let file_path = file.path();

        assert_eq!(read_table_key(file_path, "KEY1", ' '), Some(400));
        assert_eq!(read_table_key(file_path, "KEY2", ' '), Some(500));
        assert_eq!(
            read_table_key(Path::new("/nonexistent/file"), "KEY1", ':'),
            None
        );
    }

    #[test]
    fn test_v2_parent_limit() {
        let root = tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");

        create_dir_all(&child).unwrap();
        write(root.path().join("memory.max"), "max").unwrap();
        write(parent.join("memory.max"), "500").unwrap();
        write(parent.join("memory.current"), "350").unwrap();
        write(child.join("memory.max"), "max").unwrap();
        write(child.join("memory.current"), "100").unwrap();
        write(child.join("memory.stat"), "anon 30\n").unwrap();

        let limits = v2_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, 500);
        assert_eq!(limits.free_memory, 150);
        assert_eq!(limits.free_swap, 700);
        assert_eq!(limits.rss, 30);
    }

    #[test]
    fn test_v2_parent_free_memory() {
        let root = tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");

        create_dir_all(&child).unwrap();
        write(root.path().join("memory.max"), "max").unwrap();
        write(parent.join("memory.max"), "500").unwrap();
        write(parent.join("memory.current"), "450").unwrap();
        write(child.join("memory.max"), "200").unwrap();
        write(child.join("memory.current"), "100").unwrap();
        write(child.join("memory.stat"), "anon 30\n").unwrap();

        let limits = v2_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, 200);
        assert_eq!(limits.free_memory, 50);
    }

    #[test]
    fn test_v1_parent_limit() {
        let root = tempdir().unwrap();
        let parent = root.path().join("parent");
        let child = parent.join("child");

        create_dir_all(&child).unwrap();
        write(
            root.path().join("memory.limit_in_bytes"),
            u64::MAX.to_string(),
        )
        .unwrap();
        write(parent.join("memory.limit_in_bytes"), "500").unwrap();
        write(parent.join("memory.usage_in_bytes"), "350").unwrap();
        write(child.join("memory.limit_in_bytes"), u64::MAX.to_string()).unwrap();
        write(child.join("memory.usage_in_bytes"), "100").unwrap();
        write(child.join("memory.stat"), "total_rss 30\n").unwrap();

        let limits = v1_limits(
            &child,
            root.path(),
            CGroupLimitsContext {
                mem_total: 2000,
                swap_total: 1000,
                swap_free: 700,
            },
        )
        .unwrap();

        assert_eq!(limits.total_memory, 500);
        assert_eq!(limits.free_memory, 150);
        assert_eq!(limits.rss, 30);
    }

    #[test]
    fn test_parse_cgroup_path_v2() {
        assert_eq!(
            parse_cgroup_path("0::/user.slice/session.scope"),
            Some(PathBuf::from("user.slice/session.scope"))
        );
    }

    #[test]
    fn test_parse_cgroup_path_v1_memory() {
        assert_eq!(
            parse_cgroup_path("12:cpuset:/\n11:memory:/system.slice/service.scope"),
            Some(PathBuf::from("system.slice/service.scope")),
        );
    }

    #[test]
    fn test_parse_cgroup_path_missing_memory_controller() {
        assert_eq!(parse_cgroup_path("12:cpuset:/\n10:cpu:/"), None);
    }
}
