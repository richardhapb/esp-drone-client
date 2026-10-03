use super::Transport;
use std::{net::UdpSocket, time::Duration};

pub struct UdpTransport {
    socket: UdpSocket,
}

impl Transport for UdpTransport {
    fn connect() -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:3400").map_err(|e| format!("error binding: {e}"))?;
        socket
            .connect("192.168.1.101:2390")
            .map_err(|e| format!("error connecting: {e}"))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(350)))
            .map_err(|e| format!("error setting read timeout: {e}"))?;
        Ok(Self { socket })
    }

    fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.recv(buf)
    }

    fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket.send(buf)
    }
}
