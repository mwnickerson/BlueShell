use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize)]
struct Args {
    #[serde(default)]
    path: String,
    #[serde(default)]
    destination: String,
    #[serde(default)]
    data: String,
}

pub fn run(command: &str, parameters: &str) -> Result<String, String> {
    let args: Args = serde_json::from_str(parameters).unwrap_or_else(|_| Args {
        path: parameters.into(),
        destination: String::new(),
        data: String::new(),
    });
    match command {
        "cat" => fs::read(&args.path)
            .map(|v| STANDARD.encode(v))
            .map_err(|e| e.to_string()),
        "write" | "upload" => {
            let data = STANDARD.decode(args.data).map_err(|e| e.to_string())?;
            fs::write(args.path, data)
                .map(|_| String::new())
                .map_err(|e| e.to_string())
        }
        "ls" => {
            let mut rows = Vec::new();
            for entry in fs::read_dir(if args.path.is_empty() {
                "."
            } else {
                &args.path
            })
            .map_err(|e| e.to_string())?
            {
                let entry = entry.map_err(|e| e.to_string())?;
                let metadata = entry.metadata().map_err(|e| e.to_string())?;
                rows.push(format!(
                    "{}\t{}\t{}",
                    if metadata.is_dir() { "d" } else { "f" },
                    metadata.len(),
                    entry.file_name().to_string_lossy()
                ));
            }
            Ok(rows.join("\n"))
        }
        "mkdir" => fs::create_dir_all(args.path)
            .map(|_| String::new())
            .map_err(|e| e.to_string()),
        "rm" => {
            let path = PathBuf::from(args.path);
            if path.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            }
            .map(|_| String::new())
            .map_err(|e| e.to_string())
        }
        "mv" => fs::rename(args.path, args.destination)
            .map(|_| String::new())
            .map_err(|e| e.to_string()),
        _ => Err(String::new()),
    }
}

pub fn read(parameters: &str) -> Result<(String, Vec<u8>), String> {
    let args: Args = serde_json::from_str(parameters).unwrap_or_else(|_| Args {
        path: parameters.into(),
        destination: String::new(),
        data: String::new(),
    });
    let data = fs::read(&args.path).map_err(|e| e.to_string())?;
    Ok((args.path, data))
}
