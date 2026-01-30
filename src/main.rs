use serde::Serialize;
use std::env;
use std::error::Error;
use std::process;

mod ftp;

#[derive(Serialize)]
pub struct BridgeError {
    pub code: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        eprintln!("Usage: {} <command>", args[0]);
        process::exit(1);
    }

    let wrong_arg_error = serde_json::to_string(&BridgeError { code: 1 })?;

    match args[1].as_str() {
        "ftp" => ftp::init(),
        _ => {
            eprintln!("{}", wrong_arg_error);
            process::exit(1);
        }
    }
}
