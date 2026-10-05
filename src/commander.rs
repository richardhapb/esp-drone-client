use crate::channels::{Channel, CommanderChannel};
use crate::link::Link;
use crate::packet::build_packet;
const COMMAND_SIZE: usize = 14;

#[derive(Debug, Default)]
pub struct Commander {
    roll: f32,
    pitch: f32,
    yaw: f32,
    thrust: u16,
}

impl Commander {
    pub fn new(roll: f32, pitch: f32, yaw: f32, thrust: u16) -> Self {
        Self {
            roll,
            pitch,
            yaw,
            thrust,
        }
    }

    pub fn to_bytes(&self) -> [u8; COMMAND_SIZE] {
        let mut buf = [0u8; COMMAND_SIZE];
        buf[0..4].copy_from_slice(&self.roll.to_le_bytes());
        buf[4..8].copy_from_slice(&self.pitch.to_le_bytes());
        buf[8..12].copy_from_slice(&self.yaw.to_le_bytes());
        buf[12..COMMAND_SIZE].copy_from_slice(&self.thrust.to_le_bytes());
        buf
    }

    pub fn send<L: Link>(&self, link: &L) -> Result<(), String> {
        let packet = build_packet(
            &Channel::Commander(CommanderChannel::Default),
            &self.to_bytes(),
        );

        link.send_with_retries(&packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Recorder {
        sent: RefCell<Vec<Vec<u8>>>,
    }

    impl Link for Recorder {
        fn recv(&self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }

        fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
            self.sent.borrow_mut().push(buf.to_vec());
            Ok(buf.len())
        }
    }

    #[test]
    fn default_is_all_zero() {
        assert_eq!(Commander::default().to_bytes(), [0u8; COMMAND_SIZE]);
    }

    #[test]
    fn encodes_little_endian_fields() {
        let bytes = Commander::new(1.5, -2.0, 0.25, 0x1234).to_bytes();
        assert_eq!(bytes[0..4], 1.5f32.to_le_bytes());
        assert_eq!(bytes[4..8], (-2.0f32).to_le_bytes());
        assert_eq!(bytes[8..12], 0.25f32.to_le_bytes());
        assert_eq!(bytes[12..14], [0x34, 0x12]);
    }

    #[test]
    fn send_wraps_setpoint_in_commander_packet() {
        let link = Recorder::default();
        let cmd = Commander::new(0.0, 0.0, 0.0, 10000);
        cmd.send(&link).unwrap();

        let sent = link.sent.borrow();
        assert_eq!(sent.len(), 1);
        let packet = &sent[0];
        // commander port (3) << 4, channel 0
        assert_eq!(packet[0], 0x30);
        assert_eq!(packet[1..15], cmd.to_bytes());
        let sum = packet[..15].iter().map(|b| *b as u32).sum::<u32>() % 256;
        assert_eq!(packet[15] as u32, sum);
    }
}
