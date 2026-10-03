use crate::channels::{Channel, CommanderChannel};
use crate::packet::build_packet;
use crate::transport::{Transport, UdpTransport};
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

    pub fn send(&self, transport: &UdpTransport) -> Result<(), String> {
        let packet = build_packet(
            &Channel::Commander(CommanderChannel::Default),
            &self.to_bytes(),
        );

        transport.send_with_retries(&packet)
    }
}
