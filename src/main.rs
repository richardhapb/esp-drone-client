use crate::packet::{Command, Crtp, Packet, Port, channels};
use std::net::UdpSocket;

mod packet;

const RETRIES: usize = 12;

fn null_packet() -> Vec<u8> {
    let crtp = Crtp::new(&channels::Channel::Link(channels::LinkChannel::Source), &[]);
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

    Ok(Crtp::from_raw(ch, port, body))
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

fn build_packet(channel: &channels::Channel, data: &[u8]) -> Vec<u8> {
    let info_crtp = Crtp::new(channel, data);
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

    let packet_info = build_packet(&channels::Channel::Log(channels::LogChannel::Toc), &[1]);
    write_with_retries(&socket, &packet_info).unwrap();
    let res = recv_from_drone(&socket).unwrap();
    println!("{res}");

    for _ in 1..30 {
        println!("Thrust: {}", thrust);
        let cmd = Command::new(0f32, 0f32, 0f32, thrust);
        let packet = build_packet(
            &channels::Channel::Commander(channels::CommanderChannel::Default),
            &cmd.to_bytes(),
        );

        socket.send(&packet).unwrap();
        sleep(100);
    }

    let zero_cmd = Command::default();
    let zero_packet = build_packet(
        &channels::Channel::Commander(channels::CommanderChannel::Default),
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
