//! Main entry point for the ned terminal modal text editor.

use std::env;
use std::process;

use ned::cli::{cli_args, cli_error};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let parsed = match cli_args::parse(args) {
        Ok(args) => args,
        Err(cli_error::unknown_option(opt)) => {
            eprintln!("error: unexpected argument '{opt}' found\n");
            eprintln!("{}", cli_args::help_text());
            process::exit(2);
        }
    };

    if parsed.help {
        print!("{}", cli_args::help_text());
        return;
    }

    if parsed.version {
        println!("{}", cli_args::version_text());
        return;
    }

    if let Err(e) = ned::run_with_config(&parsed.files, parsed.config.as_deref()) {
        eprintln!("ned error: {e}");
        process::exit(1);
    }
}
