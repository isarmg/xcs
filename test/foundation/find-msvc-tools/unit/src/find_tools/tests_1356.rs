#[cfg(test)]
    mod tests {
        use super::*;
        use std::path::Path;
        // Import the find function from the module level
        use crate::find_tools::find;

        fn host_arch_to_string(host_arch_value: u16) -> &'static str {
            match host_arch_value {
                X86 => "x86",
                X86_64 => "x64",
                AARCH64 => "arm64",
                _ => panic!("Unsupported host architecture: {}", host_arch_value),
            }
        }

        #[test]
        fn test_find_cl_exe() {
            // Test that we can find cl.exe for common target architectures
            // and validate the correct host-target combination paths
            // This should pass on Windows CI with Visual Studio installed

            let target_architectures = ["x64", "x86", "arm64"];
            let mut found_any = false;

            // Determine the host architecture
            let host_arch_value = host_arch();
            let host_name = host_arch_to_string(host_arch_value);

            for &target_arch in &target_architectures {
                if let Some(cmd) = find(target_arch, "cl.exe") {
                    // Verify the command looks valid
                    assert!(
                        !cmd.get_program().is_empty(),
                        "cl.exe program path should not be empty"
                    );
                    assert!(
                        Path::new(cmd.get_program()).exists(),
                        "cl.exe should exist at: {:?}",
                        cmd.get_program()
                    );

                    // Verify the path contains the correct host-target combination
                    // Use case-insensitive comparison since VS IDE uses "Hostx64" while Build Tools use "HostX64"
                    let path_str = cmd.get_program().to_string_lossy();
                    let path_str_lower = path_str.to_lowercase();
                    let expected_host_target_path =
                        format!("\\bin\\host{host_name}\\{target_arch}");
                    let expected_host_target_path_unix =
                        expected_host_target_path.replace("\\", "/");

                    assert!(
                        path_str_lower.contains(&expected_host_target_path) || path_str_lower.contains(&expected_host_target_path_unix),
                        "cl.exe path should contain host-target combination (case-insensitive) '{}' for {} host targeting {}, but found: {}",
                        expected_host_target_path,
                        host_name,
                        target_arch,
                        path_str
                    );

                    found_any = true;
                }
            }

            assert!(found_any, "Expected to find cl.exe for at least one target architecture (x64, x86, or arm64) on Windows CI with Visual Studio installed");
        }

        #[test]
        #[cfg(not(disable_clang_cl_tests))]
        fn test_find_llvm_tools() {
            // Import StdEnvGetter from the parent module
            use crate::find_tools::StdEnvGetter;

            // Test the actual find_llvm_tool function with various LLVM tools
            // This test assumes CI environment has Visual Studio + Clang installed
            // We test against x64 target since clang can cross-compile to any target
            let target_arch = TargetArch::new("x64").expect("Should support x64 architecture");
            let llvm_tools = ["clang.exe", "clang++.exe", "lld.exe", "llvm-ar.exe"];

            // Determine expected host-specific path based on host architecture
            let host_arch_value = host_arch();
            let expected_host_path = match host_arch_value {
                X86 => "LLVM\\bin",            // x86 host
                X86_64 => "LLVM\\x64\\bin",    // x64 host
                AARCH64 => "LLVM\\ARM64\\bin", // arm64 host
                _ => panic!("Unsupported host architecture: {}", host_arch_value),
            };

            let host_name = host_arch_to_string(host_arch_value);

            let mut found_tools_count = 0;

            for &tool in &llvm_tools {
                // Test finding LLVM tools using the standard environment getter
                let env_getter = StdEnvGetter;
                let result = find_llvm_tool(tool, target_arch, &env_getter);

                if let Some(found_tool) = result {
                    found_tools_count += 1;

                    // Verify the found tool has a valid, non-empty path
                    assert!(
                        !found_tool.path().as_os_str().is_empty(),
                        "Found LLVM tool '{}' should have a non-empty path",
                        tool
                    );

                    // Verify the tool path actually exists on filesystem
                    assert!(
                        found_tool.path().exists(),
                        "LLVM tool '{}' path should exist: {:?}",
                        tool,
                        found_tool.path()
                    );

                    // Verify the tool path contains the expected tool name
                    let path_str = found_tool.path().to_string_lossy();
                    assert!(
                        path_str.contains(tool.trim_end_matches(".exe")),
                        "Tool path '{}' should contain tool name '{}'",
                        path_str,
                        tool
                    );

                    // Verify it's in the correct host-specific VS LLVM directory
                    assert!(
                        path_str.contains(expected_host_path) || path_str.contains(&expected_host_path.replace("\\", "/")),
                        "LLVM tool should be in host-specific VS LLVM directory '{}' for {} host, but found: {}",
                        expected_host_path,
                        host_name,
                        path_str
                    );
                }
            }

            // On CI with VS + Clang installed, we should find at least some LLVM tools
            assert!(
                found_tools_count > 0,
                "Expected to find at least one LLVM tool on CI with Visual Studio + Clang installed for {} host. Found: {}",
                host_name,
                found_tools_count
            );
        }
    }
