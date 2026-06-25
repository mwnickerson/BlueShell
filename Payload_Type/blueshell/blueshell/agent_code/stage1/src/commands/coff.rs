use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;

use crate::coff::CoffLoader;

#[derive(Deserialize)]
struct Args {
    object: String,
    #[serde(default = "default_entrypoint")]
    entrypoint: String,
    #[serde(default)]
    arguments: String,
}

fn default_entrypoint() -> String {
    "go".into()
}

fn resolve_api(_module_hash: u32, _symbol_hash: u32) -> Option<usize> {
    None
}

pub fn run(parameters: &str) -> Result<String, String> {
    let args: Args = serde_json::from_str(parameters).map_err(|e| e.to_string())?;
    let object = STANDARD.decode(args.object).map_err(|e| e.to_string())?;
    let arguments = STANDARD
        .decode(&args.arguments)
        .unwrap_or_else(|_| args.arguments.into_bytes());
    let output = CoffLoader::new(resolve_api)
        .execute(&object, &args.entrypoint, &arguments)
        .map_err(|e| format!("{e:?}"))?;
    Ok(String::from_utf8_lossy(&output).into_owned())
}
