use crate::{
    commander::Commander,
    logging::Toc,
    packet::{Port, channels},
    udp::UdpTransport,
};

mod commander;
mod logging;
mod packet;
mod udp;

fn main() {
    let thrust = 10000;
    let transport = UdpTransport::connect().unwrap();

    let toc = Toc::get_info_v2(&transport).unwrap();
    println!("{}", toc);
    println!();

    for i in 0..toc.count() {
        let item = Toc::get_item_v2(&transport, i).unwrap();
        println!("{}", item);
    }

    println!("Ramping...");
    for _ in 1..30 {
        let cmd = Commander::new(0f32, 0f32, 0f32, thrust);
        cmd.send(&transport).unwrap();
        sleep(100);
    }

    let zero_cmd = Commander::default();

    // Cool down
    println!("Cooling down...");
    for _ in 1..30 {
        zero_cmd.send(&transport).unwrap();
        sleep(10);
    }
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
