use command_line::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Network port to use
    #[arg(value_parser = command_line::value_parser!(u16).range(1..))]
    port: u16,
}

fn main() {
    let cli = Cli::parse();

    println!("PORT = {}", cli.port);
}
