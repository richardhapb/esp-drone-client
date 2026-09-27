use std::fmt::Display;

use crate::packet::channels;
use crate::{packet::build_packet, udp::UdpTransport};
pub const GET_ITEM_V2: u8 = 0x02;
pub const GET_INFO_V2: u8 = 0x03;

#[derive(Debug, Clone, Copy)]
pub struct Toc;

impl Toc {
    pub fn get_info_v2(transport: &UdpTransport) -> Result<TocInfoV2, String> {
        let packet = build_packet(
            &channels::Channel::Log(channels::LogChannel::Toc),
            &[GET_ITEM_V2],
        );

        transport.write_with_retries(&packet)?;
        let res = transport.recv_with_retries()?;

        Ok(TocInfoV2::new(res[..9].try_into().unwrap()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TocInfoV2 {
    data: [u8; 9],
}

impl TocInfoV2 {
    pub fn new(data: [u8; 9]) -> Self {
        Self { data }
    }

    pub fn log_len(&self) -> u16 {
        let raw: [u8; 2] = self.data[1..3].try_into().unwrap();
        u16::from_le_bytes(raw)
    }

    pub fn crc(&self) -> &[u8] {
        &self.data[3..7]
    }

    pub fn max_blocks(&self) -> u8 {
        self.data[7]
    }

    pub fn max_ops(&self) -> u8 {
        self.data[8]
    }
}

impl Display for TocInfoV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TOC:\n Len: {}\nCRC: {:?}\nMax blocks:{}\nMax ops:{}\n",
            self.log_len(),
            self.crc(),
            self.max_blocks(),
            self.max_ops()
        )
    }
}
