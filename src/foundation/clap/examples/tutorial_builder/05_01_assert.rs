use clap::{arg, command, value_parser};

fn main() {
    let matches = cmd().get_matches();

    // Note, it's safe to call unwrap() because the arg is required
    let port: usize = *matches
        .get_one::<usize>("PORT")
        .expect("'PORT' is required and parsing will fail if its missing");
    println!("PORT = {port}");
}

fn cmd() -> clap::Command {
    command!() // requires `cargo` feature
        .arg(
            arg!(<PORT>)
                .help("Network port to use")
                .value_parser(value_parser!(usize)),
        )
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/clap/functions/examples/tutorial_builder/05_01_assert/verify_cmd_22.rs"));
