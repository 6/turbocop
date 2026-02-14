use std::process;

use clap::Parser;

use rblint::cli::Args;

fn main() {
    let args = Args::parse();
    match rblint::run(args) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(2);
        }
    }
}
