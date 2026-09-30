use crate::packet::{Crtp, Port, null_packet};
use std::{net::UdpSocket, time::Duration};

const RETRIES: usize = 12;
const BUF_SIZE: usize = 128;

pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub fn connect() -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:3400").map_err(|e| format!("error binding: {e}"))?;
        socket
            .connect("192.168.1.101:2390")
            .map_err(|e| format!("error connecting: {e}"))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(350)))
            .map_err(|e| format!("error setting read timeout: {e}"))?;
        Ok(Self { socket })
    }

    pub fn recv_crtp(&self) -> Result<Crtp, String> {
        let res = self.recv_with_retries()?;

        if res.is_empty() {
            return Err("no data received".into());
        }

        println!("received: {} -> {:?}", res.len(), res);
        let hdr = res[0];
        let body = &res[1..];

        let port = Port::from(hdr >> 4);
        let ch = hdr & 0b11;

        Ok(Crtp::from_raw(ch, port, body))
    }

    pub fn recv_with_retries(&self) -> Result<Vec<u8>, String> {
        let mut buf = [0; BUF_SIZE];
        for _ in 0..RETRIES {
            let n = self
                .socket
                .recv(&mut buf)
                .map_err(|e| format!("error receiving: {e}"))?;
            if n > 0 {
                return Ok(buf[..n].to_vec());
            }

            sleep(100);
        }

        Err("no response received".into())
    }

    pub fn write_with_retries(&self, msg: &[u8]) -> Result<(), String> {
        let mut i = 1;
        while let Err(e) = self.socket.send(msg) {
            eprintln!("error sending package: {}", e);
            if i > RETRIES {
                return Err("max retries reached, cannot write to the drone".into());
            }
            i += 1;
            sleep(1000);
        }

        Ok(())
    }
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

pub fn send_null_packet(transport: &UdpTransport) -> Result<(), String> {
    let npck = null_packet();
    transport.write_with_retries(&npck)
}
