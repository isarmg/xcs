/// Dummy test to make it seem hang when compiled as `--test`
/// See https://github.com/matklad/once_cell/issues/79
#[test]
fn dummy_test() {
    std::thread::sleep(std::time::Duration::from_secs(4));
}
