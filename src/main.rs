mod app_config;
mod backend;
mod cli;
mod commands;
mod config;
mod debug;
mod error;
mod headers;
mod jmap;
mod output;
mod plain;
mod schema;
mod sugar;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();

    crate::debug::set_verbose(cli.verbose);

    let env = commands::dispatch(&cli).await;

    // watch is primarily a streaming command; suppress the final envelope for stream-only consumers.
    let suppress_envelope =
        matches!(cli.command, crate::cli::Command::Watch(ref args) if args.no_envelope || cli.plain);

    if !suppress_envelope {
        if cli.plain {
            output::print_plain(&env);
        } else {
            output::print_envelope(&env);
        }
    }

    if !env.ok {
        std::process::exit(1);
    }
}
