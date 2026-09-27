use crate::packet::{Crtp, Port, null_packet};
use std::{
    net::UdpSocket,
    time::{Duration, Instant},
};

const RETRIES: usize = 12;
const BUF_SIZE: usize = 128;

pub struct UdpTransport {
    socket: UdpSocket,
}

impl UdpTransport {
    pub fn connect() -> Result<Self, String> {
        let socket = UdpSocket::bind("0.0.0.0:3400").map_err(|e| format!("error binding: {e}"))?;
        socket
            .connect("192.168.1.99:2390")
            .map_err(|e| format!("error connecting: {e}"))?;
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
        println!("sending : {:?}", msg);
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

    pub fn write_until_get_response(
        &self,
        data: &[u8],
        duration: Duration,
    ) -> Result<Vec<u8>, String> {
        let prev_timeout = self
            .socket
            .read_timeout()
            .map_err(|e| format!("error retrieving timeout: {e}"))?;

        self.socket
            .set_read_timeout(Some(Duration::from_millis(1000)))
            .map_err(|e| format!("error setting timeout: {e}"))?;

        println!("timeout: {:?}", self.socket.read_timeout());

        let result = (|| {
            let start = Instant::now();
            let mut buf = [0; BUF_SIZE];

            self.write_with_retries(data)?;

            loop {
                match self.socket.recv(&mut buf) {
                    Ok(n) if n > 0 => return Ok(buf[..n].to_vec()),
                    Ok(_) => {
                        self.write_with_retries(data)?;
                    }
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        self.write_with_retries(data)?;
                    }
                    Err(e) => return Err(format!("error receiving: {e}")),
                }

                if start.elapsed() >= duration {
                    return Err("no data received until retry timeout".to_string());
                }
            }
        })();

        self.socket
            .set_read_timeout(prev_timeout)
            .map_err(|e| format!("error restoring timeout: {e}"))?;

        result
    }
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

pub fn send_null_packet(transport: &UdpTransport) -> Result<Vec<u8>, String> {
    let npck = null_packet();
    transport.write_until_get_response(&npck, Duration::from_secs(10))
}
