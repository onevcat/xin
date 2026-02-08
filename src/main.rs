mod backend;
mod cli;
mod commands;
mod config;
mod error;
mod headers;
mod jmap;
mod output;
mod schema;
mod sugar;

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
