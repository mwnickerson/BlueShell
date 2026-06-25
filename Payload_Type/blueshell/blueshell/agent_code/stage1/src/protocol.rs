use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Checkin<'a> {
    pub action: &'static str,
    pub uuid: &'a str,
    pub user: String,
    pub host: String,
    pub pid: u32,
    pub architecture: &'static str,
    pub process_name: String,
}

#[derive(Deserialize)]
pub struct CheckinReply {
    pub id: String,
    pub status: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Delegate {
    pub message: String,
    pub c2_profile: String,
    pub uuid: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProxyPacket {
    pub exit: bool,
    pub server_id: u32,
    #[serde(default)]
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u32>,
}

#[derive(Serialize)]
pub struct GetTasking {
    pub action: &'static str,
    pub tasking_size: i32,
    pub get_delegate_tasks: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub responses: Vec<TaskResponse>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub socks: Vec<ProxyPacket>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rpfwd: Vec<ProxyPacket>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub delegates: Vec<Delegate>,
}

#[derive(Deserialize)]
pub struct TaskingReply {
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub socks: Vec<ProxyPacket>,
    #[serde(default)]
    pub rpfwd: Vec<ProxyPacket>,
    #[serde(default)]
    pub delegates: Vec<Delegate>,
}

#[derive(Deserialize)]
pub struct Task {
    pub command: String,
    #[serde(default)]
    pub parameters: String,
    pub id: String,
}

#[derive(Clone, Serialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub completed: bool,
    pub status: String,
    pub user_output: String,
}
