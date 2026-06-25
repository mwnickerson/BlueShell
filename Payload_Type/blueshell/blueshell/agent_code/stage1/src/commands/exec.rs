use std::process::Command;

pub fn run(parameters: &str) -> Result<String, String> {
    let command = serde_json::from_str::<serde_json::Value>(parameters)
        .ok()
        .and_then(|value| value.get("command")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| parameters.to_owned());
    #[cfg(windows)]
    let output = Command::new("cmd")
        .args(["/d", "/s", "/c", &command])
        .output();
    #[cfg(not(windows))]
    let output = Command::new("sh").args(["-c", &command]).output();
    let output = output.map_err(|e| e.to_string())?;
    let mut result = String::from_utf8_lossy(&output.stdout).into_owned();
    result.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(result)
    } else {
        Err(result)
    }
}
