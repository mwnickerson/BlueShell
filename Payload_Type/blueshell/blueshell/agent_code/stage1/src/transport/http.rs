use super::{Transport, TransportError};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct HttpTransport {
    endpoint: String,
    uri: String,
    extended: bool,
}

impl HttpTransport {
    pub fn new(endpoint: &str, uri: &str, extended: bool) -> Self {
        Self {
            endpoint: endpoint.into(),
            uri: uri.into(),
            extended,
        }
    }
}

impl Transport for HttpTransport {
    fn exchange(&mut self, message: &str, timeout: Duration) -> Result<String, TransportError> {
        let mut stream = TcpStream::connect(&self.endpoint).map_err(|_| TransportError)?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|_| TransportError)?;
        let connection = if self.extended { "keep-alive" } else { "close" };
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: text/plain\r\nConnection: {}\r\nContent-Length: {}\r\n\r\n{}",
            self.uri, self.endpoint, connection, message.len(), message
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|_| TransportError)?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).map_err(|_| TransportError)?;
        let split = raw
            .windows(4)
            .position(|v| v == b"\r\n\r\n")
            .ok_or(TransportError)?
            + 4;
        String::from_utf8(raw[split..].to_vec()).map_err(|_| TransportError)
    }
}
