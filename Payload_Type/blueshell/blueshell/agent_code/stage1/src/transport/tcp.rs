use super::{Transport, TransportError};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct TcpTransport {
    endpoint: String,
}

impl TcpTransport {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

impl Transport for TcpTransport {
    fn exchange(&mut self, message: &str, timeout: Duration) -> Result<String, TransportError> {
        let mut stream = TcpStream::connect(&self.endpoint).map_err(|_| TransportError)?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|_| TransportError)?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|_| TransportError)?;
        let data = message.as_bytes();
        stream
            .write_all(&(data.len() as u32).to_be_bytes())
            .map_err(|_| TransportError)?;
        stream.write_all(data).map_err(|_| TransportError)?;
        let mut size = [0u8; 4];
        stream.read_exact(&mut size).map_err(|_| TransportError)?;
        let mut response = vec![0u8; u32::from_be_bytes(size) as usize];
        stream
            .read_exact(&mut response)
            .map_err(|_| TransportError)?;
        String::from_utf8(response).map_err(|_| TransportError)
    }
}
