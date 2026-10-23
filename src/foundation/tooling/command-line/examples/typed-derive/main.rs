use command_line::Parser;

mod builtin;
mod custom;
mod fn_parser;
mod foreign_crate;
mod implicit;

#[derive(Parser, Debug)] // requires `derive` feature
#[command(term_width = 0)] // Just to make testing across command_line features easier
#[command(bin_name = "typed-derive")] // Make testing easier by avoiding `.exe` in Usage on Windows
enum Cli {
    Implicit(implicit::ImplicitParsers),
    Builtin(builtin::BuiltInParsers),
    FnParser(fn_parser::FnParser),
    Custom(custom::CustomParser),
}

fn main() {
    let cli = Cli::parse();
    println!("{cli:?}");
}
