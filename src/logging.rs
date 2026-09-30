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
            &[GET_INFO_V2],
        );

        transport.write_with_retries(&packet)?;
        let res = transport.recv_with_retries()?;

        Ok(TocInfoV2::new(res[1..10].try_into().unwrap()))
    }

    pub fn get_item_v2(transport: &UdpTransport, item_id: u16) -> Result<TocItemV2, String> {
        let mut payload = [GET_ITEM_V2];
        payload[1..].copy_from_slice(&item_id.to_le_bytes());

        let packet = build_packet(&channels::Channel::Log(channels::LogChannel::Toc), &payload);

        transport.write_with_retries(&packet)?;
        let res = transport.recv_with_retries()?;

        Ok(TocItemV2::new(&res[1..]))
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

    pub fn count(&self) -> u16 {
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
            "TOC:\nCount: {}\nCRC: {:?}\nMax blocks: {}\nMax ops: {}\n",
            self.count(),
            self.crc(),
            self.max_blocks(),
            self.max_ops()
        )
    }
}

#[derive(Debug, Clone)]
pub struct TocItemV2 {
    data: Vec<u8>,
}

impl TocItemV2 {
    pub fn new(data: &[u8]) -> Self {
        let data = data.to_vec();
        Self { data }
    }

    pub fn id(&self) -> u16 {
        let raw: [u8; 2] = self.data[1..3].try_into().unwrap();
        u16::from_le_bytes(raw)
    }

    pub fn r#type(&self) -> u8 {
        self.data[3]
    }

    pub fn group(&self) -> String {
        // Ignore last byte (0x0)
        let group_raw = self.data[4..self.data.len() - 1]
            .split(|b| *b == 0x0)
            .next()
            .unwrap_or_default();

        String::from_utf8_lossy(group_raw).to_string()
    }

    pub fn name(&self) -> String {
        // Ignore last byte (0x0)
        let name_raw = self.data[4..self.data.len() - 1]
            .split(|b| *b == 0x0)
            .next_back()
            .unwrap_or_default();

        String::from_utf8_lossy(name_raw).to_string()
    }
}

impl Display for TocItemV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TOC:\nID: {}\nType: {:?}\nGroup: {}\nName: {}\n",
            self.id(),
            self.r#type(),
            self.group(),
            self.name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toc_info() {
        let response = [GET_INFO_V2, 0xA, 0, 0x10, 0x15, 0x1F, 0xF2, 5, 4];

        let toc = TocInfoV2::new(response);
        assert_eq!(toc.count(), 10);
        assert_eq!(toc.crc(), [0x10, 0x15, 0x1F, 0xF2]);
        assert_eq!(toc.max_blocks(), 5);
        assert_eq!(toc.max_ops(), 4);
    }

    #[test]
    fn parse_toc_item() {
        // item 10, type 5, group "hello" and name "world"
        let response = [
            GET_ITEM_V2,
            0xA,
            0,
            0x5,
            0x68,
            0x65,
            0x6C,
            0x6C,
            0x6F,
            0x0,
            0x77,
            0x6F,
            0x72,
            0x6C,
            0x64,
            0x0,
        ];

        let toc = TocItemV2::new(&response);
        assert_eq!(toc.id(), 10);
        assert_eq!(toc.r#type(), 5);
        assert_eq!(toc.group(), "hello");
        assert_eq!(toc.name(), "world");
    }
}
