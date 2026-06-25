use std::process::Command;

pub fn run(command: &str, parameters: &str) -> Result<String, String> {
    let pid = serde_json::from_str::<serde_json::Value>(parameters)
        .ok()
        .and_then(|value| {
            value
                .get("pid")?
                .as_u64()
                .map(|pid| pid.to_string())
                .or_else(|| value.get("pid")?.as_str().map(str::to_owned))
        })
        .unwrap_or_else(|| parameters.trim().to_owned());
    let output = match command {
        "ps" => {
            #[cfg(windows)]
            let cmd = Command::new("tasklist")
                .args(["/fo", "csv", "/nh"])
                .output();
            #[cfg(not(windows))]
            let cmd = Command::new("ps")
                .args(["-eo", "pid,ppid,user,comm"])
                .output();
            cmd
        }
        "kill" => {
            #[cfg(windows)]
            let cmd = Command::new("taskkill").args(["/f", "/pid", &pid]).output();
            #[cfg(not(windows))]
            let cmd = Command::new("kill").args(["-9", &pid]).output();
            cmd
        }
        _ => return Err(String::new()),
    }
    .map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(text)
    } else {
        Err(text)
    }
}
