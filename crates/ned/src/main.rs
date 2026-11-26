//! Main entry point for the ned terminal modal text editor.
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("ned: fast, disciplined modal terminal text editor");
        println!();
        println!("Usage: ned [OPTIONS] [FILES]...");
        println!();
        println!("Arguments:");
        println!("  [FILES]...  Files to open in editor buffers");
        println!();
        println!("Options:");
        println!("  -h, --help     Print help information");
        println!("  -v, --version  Print version information");
        return;
    }

    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("ned {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let files: Vec<String> = args.into_iter().filter(|a| !a.starts_with('-')).collect();

    if let Err(e) = ned::run(&files) {
        eprintln!("ned error: {e}");
        process::exit(1);
    }
}
