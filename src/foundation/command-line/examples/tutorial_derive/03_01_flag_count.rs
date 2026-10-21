use command_line::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, action = command_line::ArgAction::Count)]
    verbose: u8,
}

fn main() {
    let cli = Cli::parse();

    println!("verbose: {:?}", cli.verbose);
}
