//! Derived from wasm-bindgen-cli 0.2.129's wasm_bindgen module and binary.
//! Upstream copyright/license material and exact source mapping: ../PROVENANCE.json.
//! ZXS only generates browser bindings; test servers and other targets are absent.
use anyhow::{bail, Result};
use clap::{Parser, ValueEnum};
use std::{ffi::OsString, path::PathBuf};
use wasm_bindgen_cli_support::Bindgen;

#[derive(Debug, Clone, ValueEnum)]
enum Target {
    Web,
}

#[derive(Debug, Parser)]
#[command(name = "wasm-bindgen", version, about)]
struct Args {
    input: PathBuf,
    #[arg(long, value_enum, required = true)]
    target: Target,
    #[arg(long, value_name = "DIR")]
    out_dir: PathBuf,
    #[arg(
        long,
        required = true,
        help = "Generate JavaScript without TypeScript declarations"
    )]
    no_typescript: bool,
    #[arg(long, value_name = "NAME")]
    out_name: String,
}

fn run<I, T>(arguments: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match Args::try_parse_from(arguments) {
        Ok(args) => args,
        Err(error) => match error.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                print!("{error}");
                return Ok(());
            }
            _ => bail!(error),
        },
    };
    let mut bindgen = Bindgen::new();
    let Target::Web = args.target;
    debug_assert!(args.no_typescript, "required by the CLI output contract");
    // Match upstream CLI defaults exactly for the formal ZXS invocation.
    // Bindgen::new() differs here: it initially omits the default Wasm URL and
    // reads an externref environment override that the upstream CLI clears.
    #[allow(deprecated)]
    bindgen
        .input_path(&args.input)
        .out_name(&args.out_name)
        .omit_default_module_path(false)
        .reference_types(false);
    bindgen.generate(&args.out_dir)
}

fn main() {
    env_logger::init();
    if let Err(error) = run(std::env::args_os()) {
        eprintln!("error: {error:?}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_browser_binding_contract_is_accepted() {
        let arguments = [
            "wasm-bindgen",
            "--target",
            "web",
            "--no-typescript",
            "--out-name",
            "zxs",
            "--out-dir",
            "out",
            "app.wasm",
        ];
        assert!(Args::try_parse_from(arguments).is_ok());
        for target in [
            "bundler",
            "nodejs",
            "nodejs-module",
            "deno",
            "no-modules",
            "emscripten",
            "module",
        ] {
            let mut changed = arguments;
            changed[2] = target;
            assert!(Args::try_parse_from(changed).is_err(), "{target}");
        }
        assert!(Args::try_parse_from(
            arguments
                .into_iter()
                .filter(|arg| *arg != "--no-typescript")
        )
        .is_err());
    }

    #[test]
    fn emscripten_markers_fail_before_descriptor_processing() {
        for name in [
            "__wasm_bindgen_emscripten_marker",
            "__wasm_bindgen_emscripten_jspi_marker",
        ] {
            let mut wasm = b"\0asm\x01\0\0\0".to_vec();
            // Each name and section fit in a single unsigned LEB128 byte.
            wasm.extend([0, (name.len() + 1) as u8, name.len() as u8]);
            wasm.extend(name.as_bytes());
            let mut generator = Bindgen::new();
            generator.input_bytes("unsupported", wasm);
            let error = generator
                .generate_output()
                .err()
                .expect("must reject unsupported backend");
            assert!(
                error
                    .to_string()
                    .contains("unsupported Emscripten module marker"),
                "{error}"
            );
        }
    }

    #[test]
    fn help_and_version_do_not_require_a_wasm_input() {
        assert!(run(["wasm-bindgen", "--version"]).is_ok());
        assert!(run(["wasm-bindgen", "--help"]).is_ok());
    }
}

#[cfg(test)]
mod logging_tests;
