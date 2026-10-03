use std::fmt::Display;

use crate::packet::channels;
use crate::{packet::build_packet, transport::Transport};
pub const GET_ITEM_V2: u8 = 0x02;
pub const GET_INFO_V2: u8 = 0x03;

#[derive(Debug, Clone, Copy)]
pub struct Toc;

impl Toc {
    pub fn get_info_v2<T: Transport>(transport: &T) -> Result<TocInfoV2, String> {
        let packet = build_packet(
            &channels::Channel::Log(channels::LogChannel::Toc),
            &[GET_INFO_V2],
        );

        transport.send_with_retries(&packet)?;
        let res = transport.recv_with_retries()?;

        Ok(TocInfoV2::new(res[1..10].try_into().unwrap()))
    }

    pub fn get_item_v2<T: Transport>(transport: &T, item_id: u16) -> Result<TocItemV2, String> {
        let mut payload = Vec::with_capacity(3);
        payload.push(GET_ITEM_V2);
        payload.extend_from_slice(&item_id.to_le_bytes());

        let packet = build_packet(
            &channels::Channel::Log(channels::LogChannel::Toc),
            payload.as_slice(),
        );

        transport.send_with_retries(&packet)?;
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

    pub fn get_item<T: Transport>(&self, transport: &T, name: &str) -> Option<TocItemV2> {
        let parts = name.split_once(".").unwrap_or_default();
        let group = parts.0;
        let name = parts.1;

        for i in 0..self.count() {
            let item = Toc::get_item_v2(transport, i).unwrap();
            if group == item.group() && name == item.name() {
                return Some(item);
            }
        }

        None
    }
}

impl Display for TocInfoV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TOC Info\n--------\nCount: {}\nCRC: {:?}\nMax blocks: {}\nMax ops: {}\n",
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
        Self::capture_string_until_terminator(&self.data[4..])
    }

    pub fn name(&self) -> String {
        let mut i = 4;

        for j in 4..self.data.len() {
            if self.data[j] == 0x0 {
                i = (j + 1).min(self.data.len() - 1);
                break;
            }
        }

        Self::capture_string_until_terminator(&self.data[i..])
    }

    fn capture_string_until_terminator(raw_bytes: &[u8]) -> String {
        let mut g = String::new();

        for b in raw_bytes.iter() {
            if *b == 0x0 {
                break;
            }

            g.push(*b as char);
        }

        g
    }
}

impl Display for TocItemV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TOC Item\n--------\nID: {}\nType: {:?}\nGroup: {}\nName: {}\n",
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
    use std::cell::Cell;

    #[derive(Debug, Default)]
    struct DumpTransport {
        in_item: Cell<bool>,
    }

    impl Transport for DumpTransport {
        fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut data = vec![0];
            if self.in_item.get() {
                data.extend_from_slice(&get_pm_vbat())
            } else {
                data.extend_from_slice(&get_basic_toc_info());
            }

            buf[..data.len()].copy_from_slice(&data);

            Ok(data.len())
        }

        fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
            // skip (0) <- header
            let kind = buf[1];
            match kind {
                GET_INFO_V2 => self.in_item.set(false),
                GET_ITEM_V2 => self.in_item.set(true),
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "are you serious?",
                    ));
                }
            }
            Ok(buf.len())
        }
    }

    fn get_pm_vbat() -> [u8; 12] {
        [
            GET_ITEM_V2,
            0xA,
            0,
            0x5,
            0x70,
            0x6D,
            0x0,
            0x76,
            0x62,
            0x61,
            0x74,
            0x0,
        ]
    }

    fn get_basic_toc_info() -> [u8; 9] {
        // item 10, type 5, group "pm" and name "vbat"
        [GET_INFO_V2, 0xA, 0, 0x10, 0x15, 0x1F, 0xF2, 5, 4]
    }

    #[test]
    fn parse_toc_info() {
        let toc_info = get_basic_toc_info();
        let toc = TocInfoV2::new(toc_info);
        assert_eq!(toc.count(), 10);
        assert_eq!(toc.crc(), [0x10, 0x15, 0x1F, 0xF2]);
        assert_eq!(toc.max_blocks(), 5);
        assert_eq!(toc.max_ops(), 4);
    }

    #[test]
    fn parse_toc_item() {
        let pm_vbat = get_pm_vbat();

        let toc = TocItemV2::new(&pm_vbat);
        assert_eq!(toc.id(), 10);
        assert_eq!(toc.r#type(), 5);
        assert_eq!(toc.group(), "pm");
        assert_eq!(toc.name(), "vbat");
    }

    #[test]
    fn parse_get_item_some() {
        let t = DumpTransport::default();
        let toc = Toc::get_info_v2(&t).unwrap();

        let item = toc.get_item(&t, "pm.vbat");
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.group(), "pm");
        assert_eq!(item.name(), "vbat");
    }

    #[test]
    fn parse_get_item_none() {
        let t = DumpTransport::default();
        let toc = Toc::get_info_v2(&t).unwrap();

        let item = toc.get_item(&t, "nothing");
        assert!(item.is_none());
    }
}
