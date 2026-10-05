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

    use std::cell::RefCell;
    use std::time::Duration;

    #[derive(Default)]
    struct Scripted {
        reply: RefCell<Vec<u8>>,
        sent: RefCell<Vec<Vec<u8>>>,
    }

    impl Scripted {
        fn replying(reply: &[u8]) -> Self {
            Self {
                reply: RefCell::new(reply.to_vec()),
                ..Default::default()
            }
        }

        fn last_sent(&self) -> Vec<u8> {
            self.sent.borrow().last().cloned().unwrap()
        }
    }

    impl Link for Scripted {
        fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
            let r = self.reply.borrow();
            buf[..r.len()].copy_from_slice(&r);
            Ok(r.len())
        }

        fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
            self.sent.borrow_mut().push(buf.to_vec());
            Ok(buf.len())
        }
    }

    // control channel header 0x51, command, block id, errno
    fn ack(cmd: u8, block: u8, errno: u8) -> [u8; 4] {
        [0x51, cmd, block, errno]
    }

    fn item(kind: u8, id: u16) -> TocItemV2 {
        let mut raw = vec![GET_ITEM_V2];
        raw.extend_from_slice(&id.to_le_bytes());
        raw.extend_from_slice(&[kind, b'g', 0, b'n', 0]);
        TocItemV2::new(&raw)
    }

    #[test]
    fn get_info_sends_toc_request() {
        let link = Scripted::replying(&[&[0x50][..], &get_basic_toc_info()].concat());
        Logging::new(&link).get_info_v2().unwrap();
        assert_eq!(link.last_sent()[..2], [0x50, GET_INFO_V2]);
    }

    #[test]
    fn get_item_sends_little_endian_id() {
        let link = Scripted::replying(&[&[0x50][..], &get_pm_vbat()].concat());
        let it = Logging::new(&link).get_item_v2(0x0102).unwrap();
        assert_eq!(link.last_sent()[..4], [0x50, GET_ITEM_V2, 0x02, 0x01]);
        assert_eq!(it.name(), "vbat");
    }

    #[test]
    fn reset_ok() {
        let link = Scripted::replying(&ack(RESET, 0, 0));
        Logging::new(&link).reset().unwrap();
        assert_eq!(link.last_sent()[..2], [0x51, RESET]);
    }

    #[test]
    fn reset_error_and_short_response() {
        let link = Scripted::replying(&ack(RESET, 0, 5));
        assert!(Logging::new(&link).reset().is_err());

        let link = Scripted::replying(&[0x51]);
        assert!(Logging::new(&link).reset().is_err());
    }

    #[test]
    fn create_block_encodes_variables() {
        let link = Scripted::replying(&ack(CREATE_BLOCK_V2, 7, 0));
        let a = item(7, 0x0A);
        let b = item(2, 0x0102);
        Logging::new(&link).create_block(7, &[&a, &b]).unwrap();
        assert_eq!(
            link.last_sent()[..9],
            [0x51, CREATE_BLOCK_V2, 7, 7, 0x0A, 0, 2, 0x02, 0x01]
        );
    }

    #[test]
    fn create_block_reports_errno() {
        let link = Scripted::replying(&ack(CREATE_BLOCK_V2, 7, 17));
        let err = Logging::new(&link).create_block(7, &[]).unwrap_err();
        assert!(err.contains("EEXIST"), "{err}");
    }

    #[test]
    fn block_command_rejects_mismatched_ack() {
        let link = Scripted::replying(&ack(CREATE_BLOCK_V2, 8, 0));
        assert!(Logging::new(&link).create_block(7, &[]).is_err());
    }

    #[test]
    fn block_command_rejects_short_response() {
        let link = Scripted::replying(&[0x51, STOP_BLOCK]);
        assert!(Logging::new(&link).stop_block(1).is_err());
    }

    #[test]
    fn append_block_encodes_variables() {
        let link = Scripted::replying(&ack(APPEND_BLOCK_V2, 3, 0));
        let a = item(7, 0x0A);
        Logging::new(&link).append_block(3, &[&a]).unwrap();
        assert_eq!(
            link.last_sent()[..6],
            [0x51, APPEND_BLOCK_V2, 3, 7, 0x0A, 0]
        );
    }

    #[test]
    fn delete_and_stop_block() {
        let link = Scripted::replying(&ack(DELETE_BLOCK, 4, 0));
        Logging::new(&link).delete_block(4).unwrap();
        assert_eq!(link.last_sent()[..3], [0x51, DELETE_BLOCK, 4]);

        let link = Scripted::replying(&ack(STOP_BLOCK, 4, 0));
        Logging::new(&link).stop_block(4).unwrap();
        assert_eq!(link.last_sent()[..3], [0x51, STOP_BLOCK, 4]);
    }

    #[test]
    fn start_block_encodes_period_in_millis() {
        let link = Scripted::replying(&ack(START_BLOCK, 7, 0));
        Logging::new(&link)
            .start_block(7, Duration::from_millis(0x0164))
            .unwrap();
        assert_eq!(link.last_sent()[..5], [0x51, START_BLOCK, 7, 0x64, 0x01]);
    }

    #[test]
    fn start_block_rejects_oversized_period() {
        let link = Scripted::replying(&ack(START_BLOCK, 7, 0));
        let err = Logging::new(&link)
            .start_block(7, Duration::from_secs(120))
            .unwrap_err();
        assert!(err.contains("u16"));
        assert!(link.sent.borrow().is_empty());
    }

    #[test]
    fn toc_display_includes_fields() {
        let toc = TocInfoV2::new(get_basic_toc_info()).to_string();
        assert!(toc.contains("Count: 10"));
        assert!(toc.contains("Max blocks: 5"));
        let it = TocItemV2::new(&get_pm_vbat()).to_string();
        assert!(it.contains("Group: pm"));
        assert!(it.contains("Name: vbat"));
    }

    #[test]
    fn log_type_sizes_and_unknown() {
        assert_eq!(LogType::try_from(1).unwrap().size(), 1);
        assert_eq!(LogType::try_from(5).unwrap().size(), 2);
        assert_eq!(LogType::try_from(7).unwrap().size(), 4);
        assert!(LogType::try_from(0).is_err());
        assert!(LogType::try_from(8).is_err());
    }

    #[test]
    fn log_data_from_raw() {
        // data channel header, block 7, timestamp 0x030201, payload
        let data = LogData::from_raw(&[0x52, 7, 1, 2, 3, 0xAA, 0xBB]).unwrap();
        assert_eq!(data.block_id(), 7);
        assert_eq!(data.timestamp(), 0x030201);
        assert_eq!(data.data(), [0xAA, 0xBB]);
    }

    #[test]
    fn log_data_without_values() {
        let data = LogData::from_raw(&[0x52, 7, 1, 0, 0]).unwrap();
        assert!(data.data().is_empty());
    }

    #[test]
    fn log_data_rejects_other_packets() {
        assert!(LogData::from_raw(&[]).is_err());
        // control channel
        assert!(LogData::from_raw(&[0x51, 7, 1, 2, 3]).is_err());
        // commander port
        assert!(LogData::from_raw(&[0x32, 7, 1, 2, 3]).is_err());
        // too short for block id + timestamp
        assert!(LogData::from_raw(&[0x52, 7, 1]).is_err());
    }

    #[test]
    fn log_data_decodes_values() {
        let vars = [item(1, 1), item(5, 2), item(7, 3), item(4, 4)];
        let refs: Vec<_> = vars.iter().collect();
        let mut raw = vec![0x52, 7, 0, 0, 0, 200];
        raw.extend_from_slice(&(-2i16).to_le_bytes());
        raw.extend_from_slice(&3.5f32.to_le_bytes());
        raw.push(0xFF);

        let values = LogData::from_raw(&raw).unwrap().values(&refs).unwrap();
        assert_eq!(
            values,
            vec![
                LogValue::U8(200),
                LogValue::I16(-2),
                LogValue::F32(3.5),
                LogValue::I8(-1)
            ]
        );
        assert_eq!(values[1].to_string(), "-2");
    }

    #[test]
    fn log_data_decodes_wide_integers() {
        let vars = [item(2, 1), item(3, 2), item(6, 3)];
        let refs: Vec<_> = vars.iter().collect();
        let mut raw = vec![0x52, 1, 0, 0, 0];
        raw.extend_from_slice(&500u16.to_le_bytes());
        raw.extend_from_slice(&70000u32.to_le_bytes());
        raw.extend_from_slice(&(-70000i32).to_le_bytes());

        let values = LogData::from_raw(&raw).unwrap().values(&refs).unwrap();
        assert_eq!(
            values,
            vec![
                LogValue::U16(500),
                LogValue::U32(70000),
                LogValue::I32(-70000)
            ]
        );
    }

    #[test]
    fn log_data_values_errors() {
        let short = LogData::from_raw(&[0x52, 1, 0, 0, 0, 1]).unwrap();
        let v = item(3, 1);
        assert!(short.values(&[&v]).is_err());

        let bad = item(9, 1);
        assert!(short.values(&[&bad]).is_err());
    }
}
