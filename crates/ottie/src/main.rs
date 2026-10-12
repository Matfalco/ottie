// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! The `ottie` command. `ottie serve` will run the daemon; the other
//! subcommands will talk to it over its HTTP API.

use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (output, code) = respond(&args);
    if code == 0 {
        println!("{output}");
    } else {
        eprintln!("{output}");
    }
    ExitCode::from(code)
}

/// What to print for the given arguments, and the exit code.
fn respond(args: &[String]) -> (String, u8) {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => (help(), 0),
        Some("-V" | "--version") => (format!("ottie {VERSION}"), 0),
        Some(other) => (format!("ottie: unknown command `{other}`\n\n{}", help()), 2),
    }
}

fn help() -> String {
    format!(
        "Ottie {VERSION}: tucks your files away.\n\
         A cross-platform download manager (early development).\n\
         \n\
         This release only provides --help and --version. The daemon\n\
         (`ottie serve`) and download commands are being built at\n\
         {REPOSITORY}\n\
         \n\
         Usage: ottie [--help | --version]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> (String, u8) {
        respond(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn version() {
        assert_eq!(run(&["--version"]), (format!("ottie {VERSION}"), 0));
    }

    #[test]
    fn help_is_the_default() {
        assert_eq!(run(&[]).1, 0);
        assert!(run(&["--help"]).0.contains("download manager"));
    }

    #[test]
    fn unknown_command_fails() {
        let (output, code) = run(&["serve"]);
        assert_eq!(code, 2);
        assert!(output.contains("unknown command `serve`"));
    }
}
