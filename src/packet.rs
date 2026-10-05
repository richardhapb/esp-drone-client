use std::fmt::Display;

pub mod channels {
    use crate::Port;

    #[derive(Debug, Clone, Copy)]
    pub enum LinkChannel {
        Echo = 0,
        Source = 1,
        Sink = 2, // ignored by the drone
        Null = 3, // ignored by the drone
    }

    #[derive(Debug, Clone, Copy)]
    pub enum CommanderChannel {
        Default = 0,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum LogChannel {
        Toc = 0,     // table of content
        Control = 1, // used for adding/removing/starting/pausing log blocks
        Data = 2,    // used to send log data from the Crazyflie to the client
    }

    #[derive(Debug, Clone, Copy)]
    pub enum LocalizationChannel {
        External = 0,
        Generic = 1,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Channel {
        Link(LinkChannel),
        Commander(CommanderChannel),
        Log(LogChannel),
        Localization(LocalizationChannel),
    }

    impl Channel {
        pub fn port(&self) -> Port {
            match self {
                Channel::Link(_) => Port::LinkLayer,
                Channel::Commander(_) => Port::Commander,
                Channel::Log(_) => Port::DataLogging,
                Channel::Localization(_) => Port::Localization,
            }
        }
    }

    impl From<&Channel> for u8 {
        fn from(value: &Channel) -> Self {
            match value {
                Channel::Link(c) => *c as u8,
                Channel::Commander(c) => *c as u8,
                Channel::Log(c) => *c as u8,
                Channel::Localization(c) => *c as u8,
            }
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub enum Port {
    Console = 0,
    Parameters = 2,
    #[default]
    Commander = 3,
    MemoryAccess = 4,
    DataLogging = 5,
    Localization = 6,
    GenericSetPoint = 7,
    Platform = 13,
    ClientSideDebug = 14,
    LinkLayer = 15,
}

impl From<&Port> for u8 {
    fn from(value: &Port) -> Self {
        *value as u8
    }
}

impl From<u8> for Port {
    fn from(value: u8) -> Self {
        match value {
            0 => Port::Console,
            2 => Port::Parameters,
            3 => Port::Commander,
            4 => Port::MemoryAccess,
            5 => Port::DataLogging,
            6 => Port::Localization,
            7 => Port::GenericSetPoint,
            13 => Port::Platform,
            14 => Port::ClientSideDebug,
            15 => Port::LinkLayer,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Crtp {
    channel: u8,
    port: Port,
    payload: Vec<u8>,
}

#[allow(dead_code)]
impl Crtp {
    pub fn new(channel: &channels::Channel, data: &[u8]) -> Self {
        Self {
            channel: channel.into(),
            port: channel.port(),
            payload: data.to_vec(),
        }
    }

    pub fn from_raw(data: &[u8]) -> Result<Crtp, String> {
        if data.is_empty() {
            return Err("no data received".into());
        }

        let hdr = data[0];
        let body = if data.len() > 1 { &data[1..] } else { &[] };

        let port = Port::from(hdr >> 4);
        let ch = hdr & 0b11;

        Ok(Self {
            channel: ch,
            port,
            payload: body.to_vec(),
        })
    }

    pub fn channel(&self) -> u8 {
        self.channel
    }

    pub fn port(&self) -> Port {
        self.port
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn header(&self) -> u8 {
        (u8::from(&self.port) << 4) | (self.channel & 0b11)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.len());
        buf.extend_from_slice(&self.header().to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn len(&self) -> usize {
        self.payload.len() + 1
    }
}

impl Display for Crtp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = format!(
            "Channel: {}, Port: {:?}, Data: {}",
            self.channel,
            self.port,
            String::from_utf8_lossy(&self.payload)
        );

        write!(f, "{}", msg)
    }
}

pub struct Packet {
    crtp: Crtp,
    cksum: u8,
}

impl Packet {
    pub fn new(crtp: Crtp) -> Self {
        let mut cksum_raw: u16 = 0;

        for b in crtp.to_bytes() {
            cksum_raw += b as u16;
        }
        let cksum: u8 = (cksum_raw % 256) as u8;

        Self { crtp, cksum }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.crtp.len() + 1);
        buf.extend_from_slice(&self.crtp.to_bytes());
        buf.extend_from_slice(&self.cksum.to_le_bytes());
        buf
    }
}

pub fn build_packet(channel: &channels::Channel, data: &[u8]) -> Vec<u8> {
    let info_crtp = Crtp::new(channel, data);
    let packet_info = Packet::new(info_crtp);
    packet_info.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::channels::*;
    use super::*;

    #[test]
    fn channel_ports_and_numbers() {
        let c = Channel::Log(LogChannel::Data);
        assert_eq!(c.port(), Port::DataLogging);
        assert_eq!(u8::from(&c), 2);
        assert_eq!(Channel::Link(LinkChannel::Sink).port(), Port::LinkLayer);
        assert_eq!(
            Channel::Commander(CommanderChannel::Default).port(),
            Port::Commander
        );
        assert_eq!(
            u8::from(&Channel::Localization(LocalizationChannel::Generic)),
            1
        );
    }

    #[test]
    fn port_round_trip() {
        for n in [0u8, 2, 3, 4, 5, 6, 7, 13, 14, 15] {
            assert_eq!(u8::from(&Port::from(n)), n);
        }
    }

    #[test]
    fn header_packs_port_and_channel() {
        let crtp = Crtp::new(&Channel::Log(LogChannel::Control), &[]);
        assert_eq!(crtp.header(), 0x51);
        assert_eq!(crtp.len(), 1);
    }

    #[test]
    fn to_bytes_is_header_then_payload() {
        let crtp = Crtp::new(&Channel::Log(LogChannel::Toc), &[3, 4]);
        assert_eq!(crtp.to_bytes(), vec![0x50, 3, 4]);
        assert_eq!(crtp.len(), 3);
    }

    #[test]
    fn from_raw_parses_header_and_payload() {
        let crtp = Crtp::from_raw(&[0x52, 1, 2, 3]).unwrap();
        assert_eq!(crtp.port(), Port::DataLogging);
        assert_eq!(crtp.channel(), 2);
        assert_eq!(crtp.payload(), [1, 2, 3]);
    }

    #[test]
    fn from_raw_single_byte_payload() {
        let crtp = Crtp::from_raw(&[0x50, 9]).unwrap();
        assert_eq!(crtp.payload(), [9]);
    }

    #[test]
    fn from_raw_header_only() {
        let crtp = Crtp::from_raw(&[0x30]).unwrap();
        assert_eq!(crtp.port(), Port::Commander);
        assert!(crtp.payload().is_empty());
    }

    #[test]
    fn from_raw_empty_is_error() {
        assert!(Crtp::from_raw(&[]).is_err());
    }

    #[test]
    fn from_raw_round_trips_to_bytes() {
        let crtp = Crtp::new(&Channel::Log(LogChannel::Control), &[5, 6, 7]);
        let back = Crtp::from_raw(&crtp.to_bytes()).unwrap();
        assert_eq!(back.to_bytes(), crtp.to_bytes());
    }

    #[test]
    fn display_shows_fields() {
        let crtp = Crtp::new(&Channel::Log(LogChannel::Toc), b"hi");
        assert_eq!(crtp.to_string(), "Channel: 0, Port: DataLogging, Data: hi");
    }

    #[test]
    fn packet_appends_checksum() {
        let bytes = build_packet(&Channel::Log(LogChannel::Toc), &[3]);
        assert_eq!(bytes, vec![0x50, 3, 0x53]);
    }

    #[test]
    fn checksum_wraps_modulo_256() {
        let bytes = build_packet(
            &Channel::Commander(CommanderChannel::Default),
            &[0xFF, 0xFF],
        );
        // 0x30 + 0xFF + 0xFF = 0x22E
        assert_eq!(*bytes.last().unwrap(), 0x2E);
    }
}
