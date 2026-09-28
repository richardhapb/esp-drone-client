use crate::{
    commander::Commander,
    logging::Toc,
    packet::{Port, channels},
    udp::{UdpTransport, send_null_packet},
};

mod commander;
mod logging;
mod packet;
mod udp;

fn main() {
    let thrust = 10000;
    let transport = UdpTransport::connect().unwrap();

    send_null_packet(&transport).unwrap();
    let res = transport.recv_crtp().unwrap();
    println!("{res}");

    let toc = Toc::get_info_v2(&transport).unwrap();
    println!("{}", toc);

    for _ in 1..30 {
        let cmd = Commander::new(0f32, 0f32, 0f32, thrust);
        cmd.send(&transport).unwrap();
        sleep(100);
    }

    let zero_cmd = Commander::default();

    // Cool down
    for _ in 1..30 {
        zero_cmd.send(&transport).unwrap();
        sleep(10);
    }
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
