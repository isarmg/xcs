fn main() {
    let cmd = command_line::Command::new("cargo")
        .bin_name("cargo")
        .styles(COMMAND_LINE_STYLING)
        .subcommand_required(true)
        .subcommand(
            command_line::command!("example").arg(
                command_line::arg!(--"manifest-path" <PATH>)
                    .value_parser(command_line::value_parser!(std::path::PathBuf)),
            ),
        );
    let matches = cmd.get_matches();
    let matches = match matches.subcommand() {
        Some(("example", matches)) => matches,
        _ => unreachable!("clap should ensure we don't get here"),
    };
    let manifest_path = matches.get_one::<std::path::PathBuf>("manifest-path");
    println!("{manifest_path:?}");
}

// See also `command_line_cargo::style::COMMAND_LINE_STYLING`
pub const COMMAND_LINE_STYLING: command_line::builder::styling::Styles = command_line::builder::styling::Styles::styled()
    .header(command_line_cargo::style::HEADER)
    .usage(command_line_cargo::style::USAGE)
    .literal(command_line_cargo::style::LITERAL)
    .placeholder(command_line_cargo::style::PLACEHOLDER)
    .error(command_line_cargo::style::ERROR)
    .valid(command_line_cargo::style::VALID)
    .invalid(command_line_cargo::style::INVALID);
