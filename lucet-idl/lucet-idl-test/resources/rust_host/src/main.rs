use std::env;
use std::path::PathBuf;

mod harness;
mod idl;
mod run;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: lucet-idl-test-rust-host <lucet module path>");
        std::process::exit(1);
    }
    let module_path = PathBuf::from(&args[1]);
    match run::run(module_path) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    }
}
