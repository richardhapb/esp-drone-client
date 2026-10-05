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
