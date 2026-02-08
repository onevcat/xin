mod cli;
mod commands;
mod error;
mod output;
mod config;
mod jmap;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();

    let env = commands::dispatch(&cli).await;
    output::print_envelope(&env);

    if !env.ok {
        std::process::exit(1);
    }
}
