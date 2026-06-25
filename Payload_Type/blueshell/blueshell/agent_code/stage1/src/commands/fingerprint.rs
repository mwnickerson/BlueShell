use serde::Serialize;
use std::{env, process};

#[derive(Serialize)]
struct Fingerprint {
    user: String,
    host: String,
    pid: u32,
    arch: &'static str,
    os: &'static str,
}

pub fn run() -> Result<String, String> {
    let data = Fingerprint {
        user: env::var("USERNAME")
            .or_else(|_| env::var("USER"))
            .unwrap_or_default(),
        host: env::var("COMPUTERNAME")
            .or_else(|_| env::var("HOSTNAME"))
            .unwrap_or_default(),
        pid: process::id(),
        arch: env::consts::ARCH,
        os: env::consts::OS,
    };
    serde_json::to_string(&data).map_err(|e| e.to_string())
}
