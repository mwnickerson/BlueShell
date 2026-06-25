use crate::protocol::ProxyPacket;
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::Rng;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream},
};

pub struct SocksManager {
    connections: HashMap<u32, TcpStream>,
    pending: Vec<ProxyPacket>,
}

impl SocksManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub fn ingest(&mut self, packets: Vec<ProxyPacket>) {
        for packet in packets {
            if packet.exit {
                self.connections.remove(&packet.server_id);
                continue;
            }
            let Ok(data) = STANDARD.decode(&packet.data) else {
                continue;
            };
            if let Some(stream) = self.connections.get_mut(&packet.server_id) {
                if stream.write_all(&data).is_err() {
                    self.connections.remove(&packet.server_id);
                }
                continue;
            }
            match parse_socks_connect(&data).and_then(|addr| TcpStream::connect(addr).ok()) {
                Some(stream) => {
                    let _ = stream.set_nonblocking(true);
                    self.connections.insert(packet.server_id, stream);
                    self.pending.push(ProxyPacket {
                        exit: false,
                        server_id: packet.server_id,
                        data: STANDARD.encode([5, 0, 0, 1, 0, 0, 0, 0, 0, 0]),
                        port: None,
                    });
                }
                None => {
                    self.pending.push(ProxyPacket {
                        exit: true,
                        server_id: packet.server_id,
                        data: STANDARD.encode([5, 1, 0, 1, 0, 0, 0, 0, 0, 0]),
                        port: None,
                    });
                }
            }
        }
    }

    pub fn drain(&mut self) -> Vec<ProxyPacket> {
        poll_connections(&mut self.connections, &mut self.pending, None);
        std::mem::take(&mut self.pending)
    }
}

pub struct RpfwdManager {
    listeners: HashMap<u16, TcpListener>,
    connections: HashMap<u32, (u16, TcpStream)>,
    pending: Vec<ProxyPacket>,
}

impl RpfwdManager {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
            connections: HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub fn listen(&mut self, port: u16) -> std::io::Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        listener.set_nonblocking(true)?;
        self.listeners.insert(port, listener);
        Ok(())
    }

    pub fn stop(&mut self, port: u16) {
        self.listeners.remove(&port);
        self.connections.retain(|_, (p, _)| *p != port);
    }

    pub fn ingest(&mut self, packets: Vec<ProxyPacket>) {
        for packet in packets {
            if packet.exit {
                self.connections.remove(&packet.server_id);
            } else if let Ok(data) = STANDARD.decode(packet.data) {
                if let Some((_, stream)) = self.connections.get_mut(&packet.server_id) {
                    let _ = stream.write_all(&data);
                }
            }
        }
    }

    pub fn drain(&mut self) -> Vec<ProxyPacket> {
        for (&port, listener) in &self.listeners {
            while let Ok((stream, _)) = listener.accept() {
                let _ = stream.set_nonblocking(true);
                let mut id = rand::thread_rng().gen::<u32>();
                while self.connections.contains_key(&id) {
                    id = rand::thread_rng().gen();
                }
                self.connections.insert(id, (port, stream));
            }
        }
        let mut closed = Vec::new();
        let mut buffer = [0u8; 16 * 1024];
        for (&id, (port, stream)) in &mut self.connections {
            match stream.read(&mut buffer) {
                Ok(0) => closed.push((id, *port)),
                Ok(count) => self.pending.push(ProxyPacket {
                    exit: false,
                    server_id: id,
                    data: STANDARD.encode(&buffer[..count]),
                    port: Some(*port as u32),
                }),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => closed.push((id, *port)),
            }
        }
        for (id, port) in closed {
            self.connections.remove(&id);
            self.pending.push(ProxyPacket {
                exit: true,
                server_id: id,
                data: String::new(),
                port: Some(port as u32),
            });
        }
        std::mem::take(&mut self.pending)
    }
}

fn poll_connections(
    connections: &mut HashMap<u32, TcpStream>,
    pending: &mut Vec<ProxyPacket>,
    port: Option<u32>,
) {
    let mut closed = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    for (&id, stream) in connections.iter_mut() {
        match stream.read(&mut buffer) {
            Ok(0) => closed.push(id),
            Ok(count) => pending.push(ProxyPacket {
                exit: false,
                server_id: id,
                data: STANDARD.encode(&buffer[..count]),
                port,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => closed.push(id),
        }
    }
    for id in closed {
        connections.remove(&id);
        pending.push(ProxyPacket {
            exit: true,
            server_id: id,
            data: String::new(),
            port,
        });
    }
}

fn parse_socks_connect(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 7 || data[0] != 5 || data[1] != 1 {
        return None;
    }
    match data[3] {
        1 if data.len() >= 10 => Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(data[4], data[5], data[6], data[7])),
            u16::from_be_bytes([data[8], data[9]]),
        )),
        4 if data.len() >= 22 => {
            let octets: [u8; 16] = data[4..20].try_into().ok()?;
            Some(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from(octets)),
                u16::from_be_bytes([data[20], data[21]]),
            ))
        }
        3 => {
            let length = *data.get(4)? as usize;
            let host = std::str::from_utf8(data.get(5..5 + length)?).ok()?;
            let port = u16::from_be_bytes([*data.get(5 + length)?, *data.get(6 + length)?]);
            (host, port).to_socket_addrs().ok()?.next()
        }
        _ => None,
    }
}

use std::net::ToSocketAddrs;
