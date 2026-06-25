use std::{env, process::Command};

pub fn run(command: &str, parameters: &str) -> Result<String, String> {
    match command {
        "cd" => {
            let path = serde_json::from_str::<serde_json::Value>(parameters)
                .ok()
                .and_then(|value| value.get("path")?.as_str().map(str::to_owned))
                .unwrap_or_else(|| parameters.trim().to_owned());
            env::set_current_dir(path).map_err(|e| e.to_string())?;
            env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| e.to_string())
        }
        "pwd" => env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| e.to_string()),
        "whoami" => command_output("whoami", &[]),
        "hostname" => command_output("hostname", &[]),
        _ => Err(String::new()),
    }
}

fn command_output(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into())
    }
}
