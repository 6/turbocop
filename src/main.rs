use std::process;

use clap::Parser;

use turbocop::cli::Args;

fn main() {
    let args = Args::parse();
    match turbocop::run(args) {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(3);
        }
    }
}
