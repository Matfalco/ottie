// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The `ottie` command. `ottie serve` runs the daemon; the other subcommands
//! are small helpers or, later, clients of its HTTP API (docs/decisions/0004).

mod config;
mod serve;
mod token;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::config::DirArgs;
use crate::serve::ServeArgs;

#[derive(Parser)]
#[command(
    name = "ottie",
    bin_name = "ottie",
    version,
    about = "Ottie tucks your files away: a cross-platform download manager.",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the daemon: the download engine and its HTTP API (127.0.0.1:16889).
    Serve(ServeArgs),
    /// Print the API token, for the web UI's login page or the browser extension.
    Token(DirArgs),
}

#[tokio::main]
async fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Serve(args) => serve::run(args).await,
        Command::Token(dirs) => token::print(&dirs),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // `:#` prints the whole chain: "could not …: because …".
            eprintln!("ottie: {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn serve_takes_folders_address_and_port() {
        let cli = Cli::try_parse_from([
            "ottie",
            "serve",
            "--port",
            "9000",
            "--address",
            "0.0.0.0",
            "--data-dir",
            "/d",
        ])
        .unwrap();
        let Command::Serve(args) = cli.command else {
            panic!("not serve");
        };
        assert_eq!(args.port, Some(9000));
        assert_eq!(args.address, Some([0, 0, 0, 0].into()));
        assert_eq!(args.dirs.data_dir.as_deref(), Some("/d".as_ref()));
    }
}
