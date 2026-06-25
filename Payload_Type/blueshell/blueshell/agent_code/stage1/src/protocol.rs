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
    pub responses: Vec<ResponseAck>,
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
pub struct ResponseAck {
    pub task_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub file_id: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<Download>,
}

#[derive(Clone, Serialize)]
pub struct Download {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_chunks: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_num: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_data: Option<String>,
}
