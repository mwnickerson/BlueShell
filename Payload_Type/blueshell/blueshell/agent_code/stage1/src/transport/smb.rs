use super::{Transport, TransportError};
use std::time::Duration;

pub struct SmbTransport {
    pipe: String,
}

impl SmbTransport {
    pub fn new(pipe: &str) -> Self {
        Self { pipe: pipe.into() }
    }
}

impl Transport for SmbTransport {
    fn exchange(&mut self, message: &str, _timeout: Duration) -> Result<String, TransportError> {
        use std::fs::OpenOptions;
        use std::io::{Read, Write};
        let mut pipe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.pipe)
            .map_err(|_| TransportError)?;
        let bytes = message.as_bytes();
        pipe.write_all(&(bytes.len() as u32).to_be_bytes())
            .map_err(|_| TransportError)?;
        pipe.write_all(bytes).map_err(|_| TransportError)?;
        let mut size = [0u8; 4];
        pipe.read_exact(&mut size).map_err(|_| TransportError)?;
        let mut response = vec![0u8; u32::from_be_bytes(size) as usize];
        pipe.read_exact(&mut response).map_err(|_| TransportError)?;
        String::from_utf8(response).map_err(|_| TransportError)
    }
}
