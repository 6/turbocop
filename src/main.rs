use std::process;

use clap::Parser;

use nitrocop::cli::Args;

fn main() {
    let args = Args::parse();
    match nitrocop::run(args) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(3);
        }
    }
}
