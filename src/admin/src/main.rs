mod args;
mod credentials;
mod local;
mod pending;
mod private_io;
mod remote;

use args::Args;
use command_line::Parser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn print(value: impl data_serialization::Serialize) -> Result<()> {
    println!("{}", json_codec::to_string_pretty(&value)?);
    Ok(())
}
fn op(value: Option<String>) -> String {
    value.unwrap_or_else(|| storage_engine::new_id("op"))
}
#[tokio::main]
async fn main() {
    tracing::collector::fmt()
        .with_env_filter(
            tracing::collector::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    if let Err(error) = run(Args::parse()).await {
        eprintln!("zxs: {error}");
        std::process::exit(1);
    }
}
async fn run(args: Args) -> Result<()> {
    if let Some(address) = &args.server {
        remote::run(&args, address).await
    } else {
        local::run(args).await
    }
}
