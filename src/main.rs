mod cli;
mod commands;
mod error;
mod output;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    let env = commands::dispatch(&cli);
    output::print_envelope(&env);

    if !env.ok {
        std::process::exit(1);
    }
}
