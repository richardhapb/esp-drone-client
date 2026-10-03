use crate::Port;
use crate::packet::Crtp;

pub mod udp;

// Re-exports
pub use udp::UdpTransport;

const RETRIES: usize = 12;
const BUF_SIZE: usize = 128;

#[allow(dead_code)]
pub trait Transport: Sized {
    fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn send(&self, buf: &[u8]) -> std::io::Result<usize>;

    fn recv_with_retries(&self) -> Result<Vec<u8>, String> {
        let mut buf = [0; BUF_SIZE];
        for _ in 0..RETRIES {
            let n = self
                .recv(&mut buf)
                .map_err(|e| format!("error receiving: {e}"))?;
            if n > 0 {
                return Ok(buf[..n].to_vec());
            }

            sleep(100);
        }

        Err("no response received".into())
    }

    fn send_with_retries(&self, msg: &[u8]) -> Result<(), String> {
        let mut i = 1;
        while let Err(e) = self.send(msg) {
            eprintln!("error sending package: {}", e);
            if i > RETRIES {
                return Err("max retries reached, cannot write to the drone".into());
            }
            i += 1;
            sleep(1000);
        }

        Ok(())
    }

    fn recv_crtp(&self) -> Result<Crtp, String> {
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
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
