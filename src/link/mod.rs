pub mod udp;

// Re-exports
pub use udp::UdpLink;

const RETRIES: usize = 12;
const BUF_SIZE: usize = 128;

pub trait Link: Sized {
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
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    struct Scripted {
        recv_calls: Cell<usize>,
        empty_reads: usize,
        recv_error: bool,
        send_failures: Cell<usize>,
        sent: RefCell<Vec<Vec<u8>>>,
    }

    impl Link for Scripted {
        fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.recv_error {
                return Err(std::io::Error::other("boom"));
            }
            let call = self.recv_calls.get();
            self.recv_calls.set(call + 1);
            if call < self.empty_reads {
                return Ok(0);
            }
            buf[..3].copy_from_slice(&[1, 2, 3]);
            Ok(3)
        }

        fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
            if self.send_failures.get() > 0 {
                self.send_failures.set(self.send_failures.get() - 1);
                return Err(std::io::Error::other("busy"));
            }
            self.sent.borrow_mut().push(buf.to_vec());
            Ok(buf.len())
        }
    }

    #[test]
    fn recv_returns_available_bytes() {
        let link = Scripted::default();
        assert_eq!(link.recv_with_retries().unwrap(), vec![1, 2, 3]);
        assert_eq!(link.recv_calls.get(), 1);
    }

    #[test]
    fn recv_retries_on_empty_reads() {
        let link = Scripted {
            empty_reads: 2,
            ..Default::default()
        };
        assert_eq!(link.recv_with_retries().unwrap(), vec![1, 2, 3]);
        assert_eq!(link.recv_calls.get(), 3);
    }

    #[test]
    fn recv_gives_up_after_retries() {
        let link = Scripted {
            empty_reads: usize::MAX,
            ..Default::default()
        };
        assert_eq!(link.recv_with_retries(), Err("no response received".into()));
        assert_eq!(link.recv_calls.get(), RETRIES);
    }

    #[test]
    fn recv_error_is_reported_immediately() {
        let link = Scripted {
            recv_error: true,
            ..Default::default()
        };
        let err = link.recv_with_retries().unwrap_err();
        assert!(err.contains("error receiving"));
    }

    #[test]
    fn send_delivers_message() {
        let link = Scripted::default();
        link.send_with_retries(&[9, 8]).unwrap();
        assert_eq!(*link.sent.borrow(), vec![vec![9, 8]]);
    }

    #[test]
    fn send_retries_after_failure() {
        let link = Scripted::default();
        link.send_failures.set(1);
        link.send_with_retries(&[7]).unwrap();
        assert_eq!(*link.sent.borrow(), vec![vec![7]]);
    }
}
