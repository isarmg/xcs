use command_line::Parser;

#[derive(Parser)] // requires `derive` feature
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(styles = COMMAND_LINE_STYLING)]
enum CargoCli {
    ExampleDerive(ExampleDeriveArgs),
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

#[derive(command_line::Args)]
#[command(version, about, long_about = None)]
struct ExampleDeriveArgs {
    #[arg(long)]
    manifest_path: Option<std::path::PathBuf>,
}

fn main() {
    let CargoCli::ExampleDerive(args) = CargoCli::parse();
    println!("{:?}", args.manifest_path);
}
