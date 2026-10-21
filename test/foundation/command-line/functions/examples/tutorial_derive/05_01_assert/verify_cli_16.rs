#[test]
fn verify_cli() {
    use command_line::CommandFactory;
    Cli::command().debug_assert();
}
