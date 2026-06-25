mod http;
mod smb;
mod tcp;

use std::{fmt, time::Duration};

pub use http::HttpTransport;
pub use smb::SmbTransport;
pub use tcp::TcpTransport;

#[derive(Debug)]
pub struct TransportError;

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("transport failure")
    }
}

pub trait Transport: Send {
    fn exchange(&mut self, message: &str, timeout: Duration) -> Result<String, TransportError>;
}

pub fn create(kind: &str, endpoint: &str, uri: &str) -> Result<Box<dyn Transport>, TransportError> {
    match kind {
        "http" => Ok(Box::new(HttpTransport::new(endpoint, uri, false))),
        "httpx" => Ok(Box::new(HttpTransport::new(endpoint, uri, true))),
        "smb" => Ok(Box::new(SmbTransport::new(endpoint))),
        "tcp" => Ok(Box::new(TcpTransport::new(endpoint))),
        _ => Err(TransportError),
    }
}
