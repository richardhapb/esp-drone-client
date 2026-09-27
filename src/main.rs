use std::{fmt::Display, net::UdpSocket};

const COMMAND_SIZE: usize = 14;
const RETRIES: usize = 12;

pub mod channels {
    pub mod link {
        pub const ECHO: u8 = 0;
        pub const SOURCE: u8 = 1;
        pub const SINK: u8 = 2; // ignored by the drone
        pub const NULL: u8 = 3; // ignored by the drone
    }

    pub mod commander {
        pub const DEFAULT: u8 = 0;
    }

    pub mod log {
        pub const TOC: u8 = 0; // table of conent
        pub const CONTROL: u8 = 1; // used for adding/removing/starting/pausing log blocks
        pub const DATA: u8 = 2; // used to send log data from the Crazyflie to the client
    }

    pub mod localization {
        pub const EXTERNAL: u8 = 0;
        pub const GENERIC: u8 = 1;
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Default, Copy)]
enum Port {
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
struct Crtp {
    channel: u8,
    port: Port,
    payload: Vec<u8>,
}

impl Crtp {
    fn new(channel: u8, port: Port, data: &[u8]) -> Self {
        let payload = data.to_vec();
        Self {
            channel,
            port,
            payload,
        }
    }

    fn header(&self) -> u8 {
        (u8::from(&self.port) << 4) | (self.channel as u8 & 0b11)
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.len());
        buf.extend_from_slice(&self.header().to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf
    }

    fn len(&self) -> usize {
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

#[derive(Debug, Default)]
struct Command {
    roll: f32,
    pitch: f32,
    yaw: f32,
    thrust: u16,
}

impl Command {
    fn new(roll: f32, pitch: f32, yaw: f32, thrust: u16) -> Self {
        Self {
            roll,
            pitch,
            yaw,
            thrust,
        }
    }

    fn to_bytes(&self) -> [u8; COMMAND_SIZE] {
        let mut buf = [0u8; COMMAND_SIZE];
        buf[0..4].copy_from_slice(&self.roll.to_le_bytes());
        buf[4..8].copy_from_slice(&self.pitch.to_le_bytes());
        buf[8..12].copy_from_slice(&self.yaw.to_le_bytes());
        buf[12..COMMAND_SIZE].copy_from_slice(&self.thrust.to_le_bytes());
        buf
    }
}

struct Packet {
    crtp: Crtp,
    cksum: u8,
}

impl Packet {
    fn new(crtp: Crtp) -> Self {
        let mut cksum_raw: u16 = 0;

        for b in crtp.to_bytes() {
            cksum_raw += b as u16;
        }
        let cksum: u8 = (cksum_raw % 256) as u8;

        Self { crtp, cksum }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.crtp.len() + 1);
        buf.extend_from_slice(&self.crtp.to_bytes());
        buf.extend_from_slice(&self.cksum.to_le_bytes());
        buf
    }
}

fn null_packet() -> Vec<u8> {
    let crtp = Crtp::new(crate::channels::link::SOURCE, Port::LinkLayer, &[]);
    Packet::new(crtp).to_bytes()
}

fn recv_with_retries(socket: &UdpSocket) -> [u8; 128] {
    let mut buf = [0; 128];
    for _ in 0..RETRIES {
        let n = socket.recv(&mut buf).unwrap();
        if n > 0 {
            return buf;
        }

        sleep(100);
    }

    buf
}

fn recv_from_drone(socket: &UdpSocket) -> Result<Crtp, &'static str> {
    let res = recv_with_retries(socket);

    if res.is_empty() {
        return Err("no data received");
    }

    println!("received: {} -> {:?}", res.len(), res);
    let hdr = res[0];
    let body = &res[1..];

    let port = Port::from(hdr >> 4);
    let ch = hdr & 0b11;

    Ok(Crtp::new(ch.into(), port, body))
}

fn write_with_retries(socket: &UdpSocket, msg: &[u8]) -> Result<(), &'static str> {
    println!("sending : {:?}", msg);
    let mut i = 1;
    while let Err(e) = socket.send(msg) {
        eprintln!("error sending package: {}", e);
        if i > RETRIES {
            return Err("Max retries reached, cannot write to the drone");
        }
        i += 1;
        sleep(1000);
    }

    Ok(())
}

fn build_packet(channel: u8, port: Port, data: &[u8]) -> Vec<u8> {
    let info_crtp = Crtp::new(channel, port, data);
    let packet_info = Packet::new(info_crtp);
    packet_info.to_bytes()
}

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:3400").expect("bind should succeed");
    socket.connect("192.168.1.99:2390").expect("should connect");
    let thrust = 10000;

    let npck = null_packet();
    write_with_retries(&socket, &npck).unwrap();
    let res = recv_from_drone(&socket).unwrap();
    println!("{res}");

    let packet_info = build_packet(crate::channels::log::TOC, Port::DataLogging, &[1]);
    write_with_retries(&socket, &packet_info).unwrap();
    let res = recv_from_drone(&socket).unwrap();
    println!("{res}");

    for _ in 1..30 {
        println!("Thrust: {}", thrust);
        let cmd = Command::new(0f32, 0f32, 0f32, thrust);
        let packet = build_packet(
            crate::channels::commander::DEFAULT,
            Port::Commander,
            &cmd.to_bytes(),
        );

        socket.send(&packet).unwrap();
        sleep(100);
    }

    let zero_cmd = Command::default();
    let zero_packet = build_packet(
        crate::channels::commander::DEFAULT,
        Port::Commander,
        &zero_cmd.to_bytes(),
    );

    // Cool down
    for _ in 1..30 {
        socket.send(&zero_packet).unwrap();
        sleep(10);
    }
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
