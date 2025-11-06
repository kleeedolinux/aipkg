use anyhow::Result;

mod cli;
mod config;
mod install;
mod uninstall;
mod upgrade;
mod repo;
mod yaml_gen;
mod verify;
mod utils;
mod query;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::parse_args();
    cli::handle_command(args).await
}

