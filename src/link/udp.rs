use super::Link;
use std::{net::UdpSocket, time::Duration};

pub struct UdpLink {
    socket: UdpSocket,
}

impl UdpLink {
    pub fn connect(address: &str) -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:3400").map_err(|e| format!("error binding: {e}"))?;
        socket
            .connect(address)
            .map_err(|e| format!("error connecting: {e}"))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(350)))
            .map_err(|e| format!("error setting read timeout: {e}"))?;
        Ok(Self { socket })
    }
}

impl Link for UdpLink {
    fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.recv(buf)
    }

    fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket.send(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_over_loopback() {
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let link = UdpLink::connect(&peer.local_addr().unwrap().to_string()).unwrap();

        link.send(&[1, 2, 3]).unwrap();
        let mut buf = [0u8; 16];
        let (n, from) = peer.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..n], [1, 2, 3]);

        peer.send_to(&[4, 5], from).unwrap();
        let n = link.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], [4, 5]);
    }

    #[test]
    fn connect_rejects_bad_address() {
        let err = UdpLink::connect("not-an-address").err().unwrap();
        assert!(err.starts_with("error"));
    }
}
