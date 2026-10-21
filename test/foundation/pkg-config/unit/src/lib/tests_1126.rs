#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn system_library_mac_test() {
        use std::path::Path;

        let system_roots = vec![PathBuf::from("/Library"), PathBuf::from("/System")];

        assert!(!is_static_available(
            "PluginManager",
            &system_roots,
            &[PathBuf::from("/Library/Frameworks")]
        ));
        assert!(!is_static_available(
            "python2.7",
            &system_roots,
            &[PathBuf::from(
                "/System/Library/Frameworks/Python.framework/Versions/2.7/lib/python2.7/config"
            )]
        ));
        assert!(!is_static_available(
            "ffi_convenience",
            &system_roots,
            &[PathBuf::from(
                "/Library/Ruby/Gems/2.0.0/gems/ffi-1.9.10/ext/ffi_c/libffi-x86_64/.libs"
            )]
        ));

        // Homebrew is in /usr/local, and it's not a part of the OS
        if Path::new("/usr/local/lib/libpng16.a").exists() {
            assert!(is_static_available(
                "png16",
                &system_roots,
                &[PathBuf::from("/usr/local/lib")]
            ));

            let libpng = Config::new()
                .range_version("1".."99")
                .probe("libpng16")
                .unwrap();
            assert!(libpng.version.find('\n').is_none());
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn system_library_linux_test() {
        assert!(!is_static_available(
            "util",
            &[PathBuf::from("/usr")],
            &[PathBuf::from("/usr/lib/x86_64-linux-gnu")]
        ));
        assert!(!is_static_available(
            "dialog",
            &[PathBuf::from("/usr")],
            &[PathBuf::from("/usr/lib")]
        ));
    }

    fn test_library_filename(target: &str, filename: &str) {
        assert_eq!(
            Library::extract_lib_from_filename(target, filename),
            Some("foo")
        );
    }

    #[test]
    fn link_filename_linux() {
        let target = "x86_64-unknown-linux-gnu";
        test_library_filename(target, "libfoo.a");
        test_library_filename(target, "libfoo.so");
    }

    #[test]
    fn link_filename_apple() {
        let target = "x86_64-apple-darwin";
        test_library_filename(target, "libfoo.a");
        test_library_filename(target, "libfoo.so");
        test_library_filename(target, "libfoo.dylib");
    }

    #[test]
    fn link_filename_msvc() {
        let target = "x86_64-pc-windows-msvc";
        // static and dynamic libraries have the same .lib suffix
        test_library_filename(target, "foo.lib");
    }

    #[test]
    fn link_filename_mingw() {
        let target = "x86_64-pc-windows-gnu";
        test_library_filename(target, "foo.lib");
        test_library_filename(target, "libfoo.a");
        test_library_filename(target, "foo.dll");
        test_library_filename(target, "foo.dll.a");
    }
}
