use std::fmt::Display;

use crate::errno::Errno;
use crate::packet::{Crtp, Port, channels};
use crate::{link::Link, packet::build_packet};
pub const GET_ITEM_V2: u8 = 0x02;
pub const GET_INFO_V2: u8 = 0x03;
pub const DELETE_BLOCK: u8 = 0x02;
pub const START_BLOCK: u8 = 0x03;
pub const STOP_BLOCK: u8 = 0x04;
pub const RESET: u8 = 0x05;
pub const CREATE_BLOCK_V2: u8 = 0x06;
pub const APPEND_BLOCK_V2: u8 = 0x07;

#[derive(Debug, Clone, Copy)]
pub struct Logging<'a, L: Link> {
    link: &'a L,
}

#[allow(dead_code)]
impl<'a, L: Link> Logging<'a, L> {
    pub fn new(link: &'a L) -> Self {
        Self { link }
    }

    pub fn get_info_v2(&self) -> Result<TocInfoV2, String> {
        let packet = build_packet(
            &channels::Channel::Log(channels::LogChannel::Toc),
            &[GET_INFO_V2],
        );

        self.link.send_with_retries(&packet)?;
        let res = self.link.recv_with_retries()?;

        Ok(TocInfoV2::new(res[1..10].try_into().unwrap()))
    }

    pub fn get_item_v2(&self, item_id: u16) -> Result<TocItemV2, String> {
        let mut payload = Vec::with_capacity(3);
        payload.push(GET_ITEM_V2);
        payload.extend_from_slice(&item_id.to_le_bytes());

        let packet = build_packet(
            &channels::Channel::Log(channels::LogChannel::Toc),
            payload.as_slice(),
        );

        self.link.send_with_retries(&packet)?;
        let res = self.link.recv_with_retries()?;

        Ok(TocItemV2::new(&res[1..]))
    }

    pub fn reset(&self) -> Result<(), String> {
        let res = self.control(&[RESET])?;

        if res.get(1) == Some(&RESET) && res.get(3) == Some(&0) {
            Ok(())
        } else {
            Err(format!("reset failed, received {:?}", res))
        }
    }

    pub fn create_block(&self, block_id: u8, variables: &[&TocItemV2]) -> Result<(), String> {
        let mut payload = vec![CREATE_BLOCK_V2, block_id];
        for v in variables {
            payload.push(v.r#type());
            payload.extend_from_slice(&v.id().to_le_bytes());
        }

        self.block_command("create block", &payload)
    }

    pub fn append_block(&self, block_id: u8, variables: &[&TocItemV2]) -> Result<(), String> {
        let mut payload = vec![APPEND_BLOCK_V2, block_id];
        for v in variables {
            payload.push(v.r#type());
            payload.extend_from_slice(&v.id().to_le_bytes());
        }

        self.block_command("append block", &payload)
    }

    pub fn delete_block(&self, block_id: u8) -> Result<(), String> {
        self.block_command("delete block", &[DELETE_BLOCK, block_id])
    }

    pub fn start_block(&self, block_id: u8, period: std::time::Duration) -> Result<(), String> {
        let period: u16 = period
            .as_millis()
            .try_into()
            .map_err(|e| format!("error transforming period to u16: {e}"))?;
        let mut payload = vec![START_BLOCK, block_id];
        payload.extend_from_slice(&period.to_le_bytes());

        self.block_command("start block", &payload)
    }

    pub fn stop_block(&self, block_id: u8) -> Result<(), String> {
        self.block_command("stop block", &[STOP_BLOCK, block_id])
    }

    fn control(&self, payload: &[u8]) -> Result<Vec<u8>, String> {
        let packet = build_packet(
            &channels::Channel::Log(channels::LogChannel::Control),
            payload,
        );

        self.link.send_with_retries(&packet)?;
        self.link.recv_with_retries()
    }

    fn block_command(&self, what: &str, payload: &[u8]) -> Result<(), String> {
        let res = self.control(payload)?;

        // header, command, block id, errno
        if res.len() < 4 {
            return Err(format!("{what} failed, short response {:?}", res));
        }

        if res[1] == payload[0] && res[2] == payload[1] && res[3] == 0 {
            Ok(())
        } else {
            Err(format!("{what} failed, error: {}", Errno::from(res[3])))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogType {
    U8,
    U16,
    U32,
    I8,
    I16,
    I32,
    F32,
}

impl LogType {
    pub fn size(&self) -> usize {
        match self {
            LogType::U8 | LogType::I8 => 1,
            LogType::U16 | LogType::I16 => 2,
            LogType::U32 | LogType::I32 | LogType::F32 => 4,
        }
    }
}

impl TryFrom<u8> for LogType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(LogType::U8),
            2 => Ok(LogType::U16),
            3 => Ok(LogType::U32),
            4 => Ok(LogType::I8),
            5 => Ok(LogType::I16),
            6 => Ok(LogType::I32),
            7 => Ok(LogType::F32),
            other => Err(format!("unsupported log type {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogValue {
    U8(u8),
    U16(u16),
    U32(u32),
    I8(i8),
    I16(i16),
    I32(i32),
    F32(f32),
}

impl Display for LogValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogValue::U8(v) => write!(f, "{v}"),
            LogValue::U16(v) => write!(f, "{v}"),
            LogValue::U32(v) => write!(f, "{v}"),
            LogValue::I8(v) => write!(f, "{v}"),
            LogValue::I16(v) => write!(f, "{v}"),
            LogValue::I32(v) => write!(f, "{v}"),
            LogValue::F32(v) => write!(f, "{v}"),
        }
    }
}

/// A log block sample received on the data channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogData {
    block_id: u8,
    timestamp: u32,
    data: Vec<u8>,
}

#[allow(dead_code)]
impl LogData {
    pub fn from_raw(raw: &[u8]) -> Result<Self, String> {
        let crtp = Crtp::from_raw(raw)?;

        if crtp.port() != Port::DataLogging || crtp.channel() != channels::LogChannel::Data as u8 {
            return Err(format!("not a log data packet: {crtp}"));
        }

        // block id + 3 bytes timestamp
        let payload = crtp.payload();
        if payload.len() < 4 {
            return Err(format!("log data too short: {:?}", payload));
        }

        let timestamp = u32::from_le_bytes([payload[1], payload[2], payload[3], 0]);

        Ok(Self {
            block_id: payload[0],
            timestamp,
            data: payload[4..].to_vec(),
        })
    }

    pub fn block_id(&self) -> u8 {
        self.block_id
    }

    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn values(&self, variables: &[&TocItemV2]) -> Result<Vec<LogValue>, String> {
        let mut values = Vec::with_capacity(variables.len());
        let mut offset = 0;

        for v in variables {
            let kind = LogType::try_from(v.r#type())?;
            let raw = self
                .data
                .get(offset..offset + kind.size())
                .ok_or_else(|| format!("log data too short for {}.{}", v.group(), v.name()))?;
            offset += kind.size();

            values.push(match kind {
                LogType::U8 => LogValue::U8(raw[0]),
                LogType::I8 => LogValue::I8(raw[0] as i8),
                LogType::U16 => LogValue::U16(u16::from_le_bytes(raw.try_into().unwrap())),
                LogType::I16 => LogValue::I16(i16::from_le_bytes(raw.try_into().unwrap())),
                LogType::U32 => LogValue::U32(u32::from_le_bytes(raw.try_into().unwrap())),
                LogType::I32 => LogValue::I32(i32::from_le_bytes(raw.try_into().unwrap())),
                LogType::F32 => LogValue::F32(f32::from_le_bytes(raw.try_into().unwrap())),
            });
        }

        Ok(values)
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

    pub fn get_item<'a, L: Link>(&self, log: &'a Logging<L>, name: &str) -> Option<TocItemV2> {
        let parts = name.split_once(".").unwrap_or_default();
        let group = parts.0;
        let name = parts.1;

        for i in 0..self.count() {
            let item = log.get_item_v2(i).ok()?;
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
    struct Dumplink {
        in_item: Cell<bool>,
    }

    impl Link for Dumplink {
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
        let t = Dumplink::default();
        let log = Logging::new(&t);
        let toc = log.get_info_v2().unwrap();

        let item = toc.get_item(&log, "pm.vbat");
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.group(), "pm");
        assert_eq!(item.name(), "vbat");
    }

    #[test]
    fn parse_get_item_none() {
        let t = Dumplink::default();
        let log = Logging::new(&t);
        let toc = log.get_info_v2().unwrap();

        let item = toc.get_item(&log, "nothing");
        assert!(item.is_none());
    }
}
